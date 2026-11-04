use crate::{
    ledger::{Ledger, LedgerError, Tx, decode, finish, lock_accounts},
    production::*,
    production_service::{lock_capacity, plan_in, recipe_in},
    provisioning::valid_text,
};
use sqlx::Row;
use uuid::Uuid;

impl Ledger {
    pub async fn complete_production(
        &self,
        key: &str,
        id: Uuid,
        mut request: CompletionRequest,
    ) -> Result<ProductionCompletion, LedgerError> {
        if !valid_text(&request.basis, 1000)
            || request
                .shortfall_basis
                .as_ref()
                .is_some_and(|s| !valid_text(s, 1000))
            || request.losses.len() > 32
        {
            return Err(LedgerError::Invalid(
                "basis, optional shortfall basis and at most 32 losses required",
            ));
        }
        request.losses.sort_by(|a, b| a.slot.cmp(&b.slot));
        if request.losses.windows(2).any(|w| w[0].slot == w[1].slot) {
            return Err(LedgerError::Invalid("duplicate loss slot"));
        }
        let payload =
            serde_json::json!({"version":1,"production_completion":{"id":id,"request":request}});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        if sqlx::query("SELECT id FROM production_plans WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .is_none()
        {
            return Err(LedgerError::ProductionNotFound);
        }
        let plan = plan_in(&mut tx, id).await?;
        if plan.state != "reserved" {
            return Err(LedgerError::ProductionClosed);
        }
        lock_capacity(&mut tx, plan.request.capacity).await?;
        let recipe = recipe_in(&mut tx, plan.request.recipe).await?;
        let expected: i64 = plan
            .planned_output
            .parse()
            .map_err(|_| LedgerError::Overflow)?;
        let actual = request.output.as_ref().map_or(0, |o| o.quantity.units());
        if actual > expected || (actual < expected && request.shortfall_basis.is_none()) {
            return Err(LedgerError::Invalid(
                "output cannot exceed plan; shortfall needs an explicit basis",
            ));
        }
        if let Some(output) = &request.output {
            // Account before resources: retain the resident provisioning lock order.
            let accounts = lock_accounts(&mut tx, &[output.producer]).await?;
            let unit: Option<Uuid> =
                sqlx::query_scalar("SELECT unit_id FROM unit_providers WHERE producer_id=$1")
                    .bind(output.producer)
                    .fetch_optional(&mut *tx)
                    .await?;
            if accounts[&output.producer] != "producer" || unit != Some(recipe.unit) {
                return Err(LedgerError::Invalid(
                    "output producer must belong to the executing unit",
                ));
            }
        }
        let ids: Vec<_> = plan.request.inputs.iter().map(|i| i.resource).collect();
        sqlx::query("SELECT id FROM resources WHERE id=ANY($1) ORDER BY id FOR UPDATE")
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await?;
        for resource in &ids {
            if !crate::evidence::usable(&mut tx, *resource).await? {
                return Err(LedgerError::EvidenceRequired);
            }
        }
        for loss in &request.losses {
            let quantity: Option<i64> = sqlx::query_scalar(
                "SELECT quantity FROM production_plan_inputs WHERE plan_id=$1 AND slot=$2",
            )
            .bind(id)
            .bind(&loss.slot)
            .fetch_optional(&mut *tx)
            .await?;
            if quantity.is_none_or(|q| loss.quantity.units() > q) {
                return Err(LedgerError::Invalid(
                    "loss must match an input and cannot exceed withdrawn quantity",
                ));
            }
        }
        if let Some(output) = &request.output {
            let inserted=sqlx::query("INSERT INTO resources(id,producer_id,category,unit,initial_quantity,unit_measure,basis) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
                .bind(output.id).bind(output.producer).bind(recipe.output_category.as_str()).bind(&recipe.output_unit).bind(actual).bind(output.unit_measure.units()).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
            if inserted == 0 {
                return Err(LedgerError::ResourceConflict);
            }
        }
        sqlx::query("INSERT INTO production_completions(plan_id,output_resource_id,actual_quantity,basis,shortfall_basis) VALUES ($1,$2,$3,$4,$5)")
            .bind(id).bind(request.output.as_ref().map(|o|o.id)).bind(actual).bind(&request.basis).bind(&request.shortfall_basis).execute(&mut *tx).await?;
        for loss in &request.losses {
            sqlx::query(
                "INSERT INTO production_input_losses(plan_id,slot,quantity) VALUES ($1,$2,$3)",
            )
            .bind(id)
            .bind(&loss.slot)
            .bind(loss.quantity.units())
            .execute(&mut *tx)
            .await?;
        }
        let result = completion_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn production_completion(
        &self,
        id: Uuid,
    ) -> Result<ProductionCompletion, LedgerError> {
        let mut tx = self.pool.begin().await?;
        completion_in(&mut tx, id).await
    }

    pub async fn resource_provenance(&self, id: Uuid) -> Result<ResourceProvenance, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query("SELECT basis FROM resources WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(LedgerError::ResourceNotFound)?;
        let plan: Option<Uuid> = sqlx::query_scalar(
            "SELECT plan_id FROM production_completions WHERE output_resource_id=$1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let completion = if let Some(plan) = plan {
            Some(completion_in(&mut tx, plan).await?)
        } else {
            None
        };
        Ok(ResourceProvenance {
            resource: id,
            origin: if completion.is_some() {
                "production"
            } else {
                "declared"
            }
            .into(),
            declaration_basis: row.try_get("basis")?,
            completion,
        })
    }
}
async fn completion_in(tx: &mut Tx<'_>, id: Uuid) -> Result<ProductionCompletion, LedgerError> {
    let value=sqlx::query_scalar("SELECT jsonb_build_object('plan',p.id,'recipe',r.id,'output_resource',c.output_resource_id,'planned_output',(p.batches::numeric*r.output_per_batch)::text,'actual_output',c.actual_quantity::text,'output_shortfall',(p.batches::numeric*r.output_per_batch-c.actual_quantity)::text,'output_category',r.output_category,'output_unit',r.output_unit,'capacity_consumed',(p.batches::numeric*r.capacity_per_batch)::text,'basis',c.basis,'shortfall_basis',c.shortfall_basis,'inputs',COALESCE((SELECT jsonb_agg(jsonb_build_object('slot',i.slot,'resource',i.resource_id,'category',s.category,'unit',s.unit,'withdrawn',i.quantity::text,'loss',COALESCE(l.quantity,0)::text,'used',(i.quantity-COALESCE(l.quantity,0))::text) ORDER BY i.slot) FROM production_plan_inputs i JOIN resources s ON s.id=i.resource_id LEFT JOIN production_input_losses l ON l.plan_id=i.plan_id AND l.slot=i.slot WHERE i.plan_id=p.id),'[]'::jsonb)) FROM production_completions c JOIN production_plans p ON p.id=c.plan_id JOIN production_recipes r ON r.id=p.recipe_id WHERE c.plan_id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ProductionNotFound)?;
    decode(value)
}
