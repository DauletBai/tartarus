use crate::{
    domain::Amount,
    ledger::{Ledger, LedgerError, Tx, available, decode, finish},
    needs::NeedRequest,
    needs_service::{create_need_in, need_in},
    planning::*,
    production::PlanRequest,
    production_service::{assess_with_limits, capacity_in, normalize, recipe_in},
    provisioning::{valid_period, valid_text},
    provisioning_service::resource_in,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

impl Ledger {
    pub async fn create_need_norm(
        &self,
        key: &str,
        request: NeedNorm,
    ) -> Result<NeedNorm, LedgerError> {
        if !valid_text(&request.code, 64)
            || !request
                .code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
            || !valid_text(&request.unit, 40)
            || !valid_text(&request.applicability, 1000)
            || !valid_text(&request.basis, 1000)
            || request.revision < 1
            || (request.revision == 1) != request.supersedes.is_none()
        {
            return Err(LedgerError::Invalid(
                "valid norm code, revision, dimensions, applicability and basis required",
            ));
        }
        let payload = serde_json::json!({"version":1,"need_norm":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(v) = replay {
            return decode(v);
        }
        if let Some(id) = request.supersedes {
            let previous = norm_in(&mut tx, id).await?;
            if previous.code != request.code
                || previous.revision.checked_add(1) != Some(request.revision)
                || previous.category.as_str() != request.category.as_str()
                || previous.unit != request.unit
            {
                return Err(LedgerError::Invalid(
                    "revision must continue the same norm code and dimension",
                ));
            }
        }
        let inserted=sqlx::query("INSERT INTO need_norms(id,code,revision,supersedes,category,unit,quantity,applicability,basis) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(&request.code).bind(request.revision).bind(request.supersedes).bind(request.category.as_str()).bind(&request.unit).bind(request.quantity.units()).bind(&request.applicability).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::NormConflict);
        }
        finish(tx, key, &payload, request).await
    }
    pub async fn need_norm(&self, id: Uuid) -> Result<NeedNorm, LedgerError> {
        let mut tx = self.pool.begin().await?;
        norm_in(&mut tx, id).await
    }
    pub async fn create_need_from_norm(
        &self,
        key: &str,
        request: NormNeedRequest,
    ) -> Result<NormNeed, LedgerError> {
        if !valid_period(&request.period) || !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid(
                "canonical period and application basis required",
            ));
        }
        let payload = serde_json::json!({"version":1,"norm_need":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(v) = replay {
            return decode(v);
        }
        let norm = norm_in(&mut tx, request.norm).await?;
        let need = create_need_in(
            &mut tx,
            NeedRequest {
                id: request.id,
                resident: request.resident,
                assigned_unit: request.assigned_unit,
                period: request.period,
                category: norm.category.clone(),
                unit: norm.unit.clone(),
                quantity: norm.quantity,
                basis: request.basis,
            },
        )
        .await?;
        sqlx::query("INSERT INTO need_norm_sources(need_id,norm_id) VALUES ($1,$2)")
            .bind(need.id)
            .bind(norm.id)
            .execute(&mut *tx)
            .await?;
        finish(tx, key, &payload, NormNeed { need, norm }).await
    }
    pub async fn need_norm_source(&self, id: Uuid) -> Result<Option<NeedNorm>, LedgerError> {
        let mut tx = self.pool.begin().await?;
        need_in(&mut tx, id).await?;
        let norm: Option<Uuid> =
            sqlx::query_scalar("SELECT norm_id FROM need_norm_sources WHERE need_id=$1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(norm) = norm {
            Ok(Some(norm_in(&mut tx, norm).await?))
        } else {
            Ok(None)
        }
    }

    /// Deterministic advice in caller-supplied priority order. No holds or money writes.
    pub async fn propose_provisioning(
        &self,
        request: PlanningRequest,
    ) -> Result<ProvisioningProposal, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        propose_in(&mut tx, request).await
    }
}
fn integer(value: &str) -> Result<i128, LedgerError> {
    value.parse().map_err(|_| LedgerError::Overflow)
}
fn amount(value: i128) -> Result<Amount, LedgerError> {
    Amount::parse(&value.to_string()).map_err(|_| LedgerError::Overflow)
}
async fn in_scope(tx: &mut Tx<'_>, root: Uuid, unit: Option<Uuid>) -> Result<bool, LedgerError> {
    Ok(sqlx::query_scalar("WITH RECURSIVE scope AS (SELECT id FROM provisioning_units WHERE id=$1 UNION ALL SELECT u.id FROM provisioning_units u JOIN scope p ON u.parent_id=p.id) SELECT EXISTS(SELECT 1 FROM scope WHERE id=$2)")
        .bind(root).bind(unit).fetch_one(&mut **tx).await?)
}
async fn norm_in(tx: &mut Tx<'_>, id: Uuid) -> Result<NeedNorm, LedgerError> {
    let value=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'code',code,'revision',revision,'supersedes',supersedes,'category',category,'unit',unit,'quantity',quantity::text,'applicability',applicability,'basis',basis) FROM need_norms WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::NormNotFound)?;
    decode(value)
}

pub(crate) async fn propose_in(
    tx: &mut Tx<'_>,
    request: PlanningRequest,
) -> Result<ProvisioningProposal, LedgerError> {
    let count: usize = request
        .needs
        .iter()
        .map(|n| n.resources.len() + n.production.len())
        .sum();
    if !valid_period(&request.period)
        || !valid_text(&request.basis, 1000)
        || request.needs.is_empty()
        || request.needs.len() > 32
        || count > 64
    {
        return Err(LedgerError::Invalid(
            "period, basis, 1..32 needs and at most 64 candidates required",
        ));
    }
    if request
        .needs
        .iter()
        .map(|n| n.need)
        .collect::<BTreeSet<_>>()
        .len()
        != request.needs.len()
    {
        return Err(LedgerError::Invalid("duplicate need"));
    }
    let observed_at:String=sqlx::query_scalar("SELECT to_char(transaction_timestamp() AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')").fetch_one(&mut **tx).await?;
    let mut stocks = BTreeMap::<Uuid, i128>::new();
    let mut capacities = BTreeMap::<Uuid, i128>::new();
    let mut accounting = BTreeMap::<Uuid, (i128, i128)>::new();
    let mut lines = Vec::new();
    for options in &request.needs {
        let need = need_in(tx, options.need).await?;
        if need.period != request.period {
            return Err(LedgerError::Invalid(
                "need period must match planning period",
            ));
        }
        if options.resources.iter().collect::<BTreeSet<_>>().len() != options.resources.len() {
            return Err(LedgerError::Invalid("duplicate stock candidate"));
        }
        let mut remaining = integer(&need.uncovered)?;
        let mut line = NeedPlan {
            need: need.id,
            category: need.category.clone(),
            unit: need.unit.clone(),
            uncovered: need.uncovered.clone(),
            stock_selected: "0".into(),
            expected_from_production: "0".into(),
            residual: String::new(),
            stock: Vec::new(),
            production: Vec::new(),
            evaluations: Vec::new(),
        };
        let free = i128::from(available(tx, need.resident).await?);
        accounting.entry(need.resident).or_insert((free, 0));
        let mut selected = 0i128;
        for id in &options.resources {
            let resource = resource_in(tx, *id).await?;
            if resource.category != need.category || resource.unit != need.unit {
                return Err(LedgerError::Invalid(
                    "stock candidate must match need dimension",
                ));
            }
            let provider: Option<Uuid> =
                sqlx::query_scalar("SELECT unit_id FROM unit_providers WHERE producer_id=$1")
                    .bind(resource.producer)
                    .fetch_optional(&mut **tx)
                    .await?;
            if !in_scope(tx, need.assigned_unit, provider).await? {
                return Err(LedgerError::ProviderOutOfScope);
            }
            let stock = stocks.entry(*id).or_insert(integer(&resource.available)?);
            let measure = integer(&resource.unit_measure)?;
            // Each suggested resident reservation must fit existing exact-accounting limits.
            let quantity = remaining.min(*stock).min(i128::from(i64::MAX) / measure);
            if quantity > 0 {
                *stock -= quantity;
                remaining -= quantity;
                selected += quantity;
                let required = quantity * measure;
                accounting.get_mut(&need.resident).unwrap().1 += required;
                line.stock.push(StockSuggestion {
                    resource: *id,
                    quantity: quantity.to_string(),
                    required_measure: required.to_string(),
                });
            }
        }
        line.stock_selected = selected.to_string();
        let mut forecast = 0i128;
        for option in &options.production {
            let recipe = recipe_in(tx, option.recipe).await?;
            let capacity = capacity_in(tx, option.capacity).await?;
            if recipe.output_category.as_str() != need.category
                || recipe.output_unit != need.unit
                || capacity.declaration.period != request.period
            {
                return Err(LedgerError::Invalid(
                    "process output and capacity period must match need",
                ));
            }
            if !in_scope(tx, need.assigned_unit, Some(recipe.unit)).await? {
                return Err(LedgerError::ProviderOutOfScope);
            }
            let per_batch = i128::from(recipe.output_per_batch.units());
            let desired = ((remaining + per_batch - 1) / per_batch).max(1);
            let mut candidate = PlanRequest {
                recipe: option.recipe,
                capacity: option.capacity,
                batches: amount(desired)?,
                inputs: option.inputs.clone(),
                basis: request.basis.clone(),
            };
            normalize(&mut candidate)?;
            let assessment = assess_with_limits(tx, &candidate, &stocks, &capacities).await?;
            if remaining == 0 {
                continue;
            }
            let batches = desired.min(integer(&assessment.max_batches)?);
            line.evaluations.push(ProcessEvaluation {
                recipe: option.recipe,
                capacity: option.capacity,
                assessment,
            });
            if batches == 0 {
                continue;
            }
            candidate.batches = amount(batches)?;
            let chosen = assess_with_limits(tx, &candidate, &stocks, &capacities).await?;
            if !chosen.feasible {
                return Err(LedgerError::InvalidStoredResponse);
            }
            for input in chosen.inputs {
                stocks.insert(
                    input.resource,
                    integer(&input.available)? - integer(&input.required)?,
                );
            }
            capacities.insert(
                option.capacity,
                integer(&chosen.capacity_available)? - integer(&chosen.capacity_required)?,
            );
            let output = integer(&chosen.planned_output)?;
            let for_need = remaining.min(output);
            remaining -= for_need;
            forecast += for_need;
            line.production.push(ProcessSuggestion {
                request: candidate,
                expected_for_need: for_need.to_string(),
                surplus: (output - for_need).to_string(),
            });
            // Forecast output is deliberately not added to stock or another need's coverage.
        }
        line.expected_from_production = forecast.to_string();
        line.residual = remaining.to_string();
        lines.push(line);
    }
    let accounting = accounting
        .into_iter()
        .map(|(resident, (available, required))| AccountingGap {
            resident,
            available: available.to_string(),
            required_for_selected_stock: required.to_string(),
            shortage: (required - available).max(0).to_string(),
        })
        .collect();
    Ok(ProvisioningProposal {
        advisory: true,
        policy: "listed_need_order_stock_then_production".into(),
        observed_at,
        period: request.period,
        basis: request.basis,
        needs: lines,
        accounting,
    })
}
