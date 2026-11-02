use crate::{
    ledger::{Ledger, LedgerError, Tx, decode, finish},
    production::*,
    provisioning::{ResolutionRequest, valid_period, valid_text},
    provisioning_service::resource_in,
};
use sqlx::Row;
use std::collections::BTreeSet;
use uuid::Uuid;

impl Ledger {
    pub async fn create_capacity(
        &self,
        key: &str,
        request: CapacityRequest,
    ) -> Result<Capacity, LedgerError> {
        if !valid_period(&request.period)
            || !valid_text(&request.capacity_unit, 40)
            || !valid_text(&request.basis, 1000)
        {
            return Err(LedgerError::Invalid(
                "period, capacity unit and basis required",
            ));
        }
        let payload = serde_json::json!({"version":1,"capacity":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        require_unit(&mut tx, request.unit).await?;
        let inserted=sqlx::query("INSERT INTO production_capacities(id,unit_id,period,capacity_unit,quantity,basis) VALUES ($1,$2,$3::text::date,$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(request.unit).bind(&request.period).bind(&request.capacity_unit).bind(request.quantity.units()).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ProductionConflict);
        }
        let result = capacity_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn capacity(&self, id: Uuid) -> Result<Capacity, LedgerError> {
        let mut tx = self.pool.begin().await?;
        capacity_in(&mut tx, id).await
    }

    pub async fn create_recipe(
        &self,
        key: &str,
        mut request: RecipeRequest,
    ) -> Result<RecipeRequest, LedgerError> {
        if !valid_text(&request.name, 120)
            || !valid_text(&request.capacity_unit, 40)
            || !valid_text(&request.output_unit, 40)
            || !valid_text(&request.basis, 1000)
            || request.inputs.len() > 32
            || request
                .inputs
                .iter()
                .any(|i| !valid_text(&i.slot, 40) || !valid_text(&i.unit, 40))
        {
            return Err(LedgerError::Invalid(
                "valid recipe fields and at most 32 distinct inputs required",
            ));
        }
        request.inputs.sort_by(|a, b| a.slot.cmp(&b.slot));
        if request.inputs.windows(2).any(|w| w[0].slot == w[1].slot) {
            return Err(LedgerError::Invalid("duplicate recipe slot"));
        }
        let payload = serde_json::json!({"version":1,"recipe":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        require_unit(&mut tx, request.unit).await?;
        let inserted=sqlx::query("INSERT INTO production_recipes(id,unit_id,name,capacity_unit,capacity_per_batch,output_category,output_unit,output_per_batch,basis) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(request.unit).bind(&request.name).bind(&request.capacity_unit).bind(request.capacity_per_batch.units()).bind(request.output_category.as_str()).bind(&request.output_unit).bind(request.output_per_batch.units()).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ProductionConflict);
        }
        for i in &request.inputs {
            sqlx::query("INSERT INTO production_recipe_inputs(recipe_id,slot,category,unit,quantity_per_batch) VALUES ($1,$2,$3,$4,$5)")
                .bind(request.id).bind(&i.slot).bind(i.category.as_str()).bind(&i.unit).bind(i.quantity_per_batch.units()).execute(&mut *tx).await?;
        }
        finish(tx, key, &payload, request).await
    }

    pub async fn recipe(&self, id: Uuid) -> Result<RecipeRequest, LedgerError> {
        let mut tx = self.pool.begin().await?;
        recipe_in(&mut tx, id).await
    }

    /// A consistent observation only. A plan rechecks under capacity/resource locks.
    pub async fn assess_production(
        &self,
        mut request: PlanRequest,
    ) -> Result<ProductionAssessment, LedgerError> {
        normalize(&mut request)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        assess_in(&mut tx, &request).await
    }

    pub async fn plan_production(
        &self,
        key: &str,
        mut request: PlanRequest,
    ) -> Result<PlanAttempt, LedgerError> {
        normalize(&mut request)?;
        let payload = serde_json::json!({"version":1,"production_plan":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        lock_capacity(&mut tx, request.capacity).await?;
        let ids: Vec<Uuid> = request.inputs.iter().map(|i| i.resource).collect();
        // Production never acquires account locks: it changes physical holds only.
        // Lock all input lots in UUID order before reading availability.
        sqlx::query("SELECT id FROM resources WHERE id=ANY($1) ORDER BY id FOR UPDATE")
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await?;
        let assessment = assess_in(&mut tx, &request).await?;
        let plan_id = if assessment.feasible {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO production_plans(id,recipe_id,capacity_id,batches,basis) VALUES ($1,$2,$3,$4,$5)")
                .bind(id).bind(request.recipe).bind(request.capacity).bind(request.batches.units()).bind(&request.basis).execute(&mut *tx).await?;
            for i in &assessment.inputs {
                let quantity: i64 = i.required.parse().map_err(|_| LedgerError::Overflow)?;
                sqlx::query("INSERT INTO production_plan_inputs(plan_id,slot,resource_id,quantity) VALUES ($1,$2,$3,$4)")
                    .bind(id).bind(&i.slot).bind(i.resource).bind(quantity).execute(&mut *tx).await?;
            }
            Some(id)
        } else {
            None
        };
        let result = PlanAttempt {
            status: if plan_id.is_some() {
                "reserved"
            } else {
                "blocked"
            }
            .into(),
            plan_id,
            assessment,
        };
        finish(tx, key, &payload, result).await
    }

    pub async fn production_plan(&self, id: Uuid) -> Result<ProductionPlan, LedgerError> {
        let mut tx = self.pool.begin().await?;
        plan_in(&mut tx, id).await
    }

    pub async fn cancel_production(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
    ) -> Result<ProductionPlan, LedgerError> {
        if !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("basis required"));
        }
        let payload =
            serde_json::json!({"version":1,"production_cancellation":{"id":id,"request":request}});
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
        // The DB trigger serializes cancellation with capacity and input reservations.
        sqlx::query("INSERT INTO production_cancellations(plan_id,basis) VALUES ($1,$2)")
            .bind(id)
            .bind(&request.basis)
            .execute(&mut *tx)
            .await?;
        let result = plan_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}

pub(crate) fn normalize(request: &mut PlanRequest) -> Result<(), LedgerError> {
    if !valid_text(&request.basis, 1000) || request.inputs.len() > 32 {
        return Err(LedgerError::Invalid("basis and at most 32 inputs required"));
    }
    request.inputs.sort_by(|a, b| a.slot.cmp(&b.slot));
    let unique: BTreeSet<_> = request.inputs.iter().map(|i| i.resource).collect();
    if unique.len() != request.inputs.len()
        || request.inputs.windows(2).any(|w| w[0].slot == w[1].slot)
    {
        return Err(LedgerError::Invalid(
            "distinct slots and one distinct resource lot per slot required",
        ));
    }
    Ok(())
}
async fn require_unit(tx: &mut Tx<'_>, id: Uuid) -> Result<(), LedgerError> {
    if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM provisioning_units WHERE id=$1)")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?
    {
        return Err(LedgerError::UnitNotFound);
    }
    Ok(())
}
pub(crate) async fn lock_capacity(tx: &mut Tx<'_>, id: Uuid) -> Result<(), LedgerError> {
    if sqlx::query("SELECT id FROM production_capacities WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .is_none()
    {
        return Err(LedgerError::ProductionNotFound);
    }
    Ok(())
}
pub(crate) async fn capacity_in(tx: &mut Tx<'_>, id: Uuid) -> Result<Capacity, LedgerError> {
    let value=sqlx::query_scalar("SELECT jsonb_build_object('declaration',jsonb_build_object('id',id,'unit',unit_id,'period',to_char(period,'YYYY-MM-DD'),'capacity_unit',capacity_unit,'quantity',quantity::text,'basis',basis),'reserved',reserved::text,'consumed',consumed::text,'available',(quantity-reserved-consumed)::text) FROM production_capacity_totals WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ProductionNotFound)?;
    decode(value)
}
pub(crate) async fn recipe_in(tx: &mut Tx<'_>, id: Uuid) -> Result<RecipeRequest, LedgerError> {
    let value=sqlx::query_scalar("SELECT jsonb_build_object('id',r.id,'unit',r.unit_id,'name',r.name,'capacity_unit',r.capacity_unit,'capacity_per_batch',r.capacity_per_batch::text,'output_category',r.output_category,'output_unit',r.output_unit,'output_per_batch',r.output_per_batch::text,'basis',r.basis,'inputs',COALESCE((SELECT jsonb_agg(jsonb_build_object('slot',i.slot,'category',i.category,'unit',i.unit,'quantity_per_batch',i.quantity_per_batch::text) ORDER BY i.slot) FROM production_recipe_inputs i WHERE i.recipe_id=r.id),'[]'::jsonb)) FROM production_recipes r WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ProductionNotFound)?;
    decode(value)
}
pub(crate) async fn plan_in(tx: &mut Tx<'_>, id: Uuid) -> Result<ProductionPlan, LedgerError> {
    let value=sqlx::query_scalar("SELECT jsonb_build_object('id',p.id,'state',CASE WHEN EXISTS(SELECT 1 FROM production_cancellations x WHERE x.plan_id=p.id) THEN 'cancelled' WHEN EXISTS(SELECT 1 FROM production_completions x WHERE x.plan_id=p.id) THEN 'completed' ELSE 'reserved' END,'planned_output',(p.batches::numeric*r.output_per_batch)::text,'request',jsonb_build_object('recipe',p.recipe_id,'capacity',p.capacity_id,'batches',p.batches::text,'basis',p.basis,'inputs',COALESCE((SELECT jsonb_agg(jsonb_build_object('slot',i.slot,'resource',i.resource_id) ORDER BY i.slot) FROM production_plan_inputs i WHERE i.plan_id=p.id),'[]'::jsonb))) FROM production_plans p JOIN production_recipes r ON r.id=p.recipe_id WHERE p.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ProductionNotFound)?;
    decode(value)
}
fn integer(value: &str) -> Result<i128, LedgerError> {
    value.parse().map_err(|_| LedgerError::Overflow)
}
async fn assess_in(
    tx: &mut Tx<'_>,
    request: &PlanRequest,
) -> Result<ProductionAssessment, LedgerError> {
    assess_with_limits(
        tx,
        request,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
    )
    .await
}
pub(crate) async fn assess_with_limits(
    tx: &mut Tx<'_>,
    request: &PlanRequest,
    stocks: &std::collections::BTreeMap<Uuid, i128>,
    capacities: &std::collections::BTreeMap<Uuid, i128>,
) -> Result<ProductionAssessment, LedgerError> {
    let mut recipe = recipe_in(tx, request.recipe).await?;
    recipe.inputs.sort_by(|a, b| a.slot.cmp(&b.slot));
    let capacity = capacity_in(tx, request.capacity).await?;
    if recipe.unit != capacity.declaration.unit
        || recipe.capacity_unit != capacity.declaration.capacity_unit
    {
        return Err(LedgerError::Invalid(
            "recipe must match capacity unit and responsible unit",
        ));
    }
    if recipe.inputs.len() != request.inputs.len()
        || recipe
            .inputs
            .iter()
            .zip(&request.inputs)
            .any(|(a, b)| a.slot != b.slot)
    {
        return Err(LedgerError::Invalid(
            "select exactly one lot for every recipe input",
        ));
    }
    let batches = i128::from(request.batches.units());
    let capacity_cost = i128::from(recipe.capacity_per_batch.units());
    let output = i128::from(recipe.output_per_batch.units());
    let available =
        integer(&capacity.available)?.min(*capacities.get(&request.capacity).unwrap_or(&i128::MAX));
    let required = batches * capacity_cost;
    let limit = i128::from(i64::MAX);
    let mut max_batches = (available / capacity_cost).min(limit / output);
    let mut reasons = Vec::new();
    if required > available {
        reasons.push("capacity_shortage".into());
    }
    let mut overflow = required > limit || batches * output > limit;
    let mut inputs = Vec::new();
    for (input, selection) in recipe.inputs.iter().zip(&request.inputs) {
        let resource = resource_in(tx, selection.resource).await?;
        if resource.category != input.category.as_str() || resource.unit != input.unit {
            return Err(LedgerError::Invalid(
                "input lot category and physical unit must match recipe",
            ));
        }
        let row = sqlx::query("SELECT unit_id FROM unit_providers WHERE producer_id=$1")
            .bind(resource.producer)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(LedgerError::Invalid(
                "input provider must be assigned to a unit",
            ))?;
        let supplying_unit = row.try_get("unit_id")?;
        let per_batch = i128::from(input.quantity_per_batch.units());
        let required = batches * per_batch;
        let available =
            integer(&resource.available)?.min(*stocks.get(&resource.id).unwrap_or(&i128::MAX));
        max_batches = max_batches.min(available / per_batch);
        overflow |= required > limit;
        inputs.push(InputAssessment {
            slot: input.slot.clone(),
            resource: resource.id,
            supplying_unit,
            category: resource.category,
            unit: resource.unit,
            required: required.to_string(),
            available: available.to_string(),
            shortage: (required - available).max(0).to_string(),
        });
    }
    if inputs.iter().any(|i| i.shortage != "0") {
        reasons.push("input_shortage".into());
    }
    if overflow {
        reasons.push("quantity_limit".into());
    }
    Ok(ProductionAssessment {
        feasible: reasons.is_empty(),
        max_batches: max_batches.to_string(),
        planned_output: (batches * output).to_string(),
        capacity_required: required.to_string(),
        capacity_available: available.to_string(),
        reasons,
        inputs,
    })
}
