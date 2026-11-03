use crate::{
    domain::Amount,
    drafts::{DraftAction, DraftRequest, PlanningDraft},
    ledger::{Ledger, LedgerError, Tx, decode, finish, lock_accounts},
    needs::NeedReservationRequest,
    needs_service::reserve_need_in,
    planning::PlanningRequest,
    planning_service::propose_in,
    production_service::plan_production_in,
    provisioning::{ResolutionRequest, valid_text},
};
use sqlx::Row;
use std::collections::BTreeSet;
use uuid::Uuid;

impl Ledger {
    pub async fn create_planning_draft(
        &self,
        key: &str,
        request: DraftRequest,
    ) -> Result<PlanningDraft, LedgerError> {
        let payload = serde_json::json!({"version":1,"planning_draft":request});
        let (tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        // Release the command connection before acquiring a snapshot connection.
        // Recheck the key after assessment to handle simultaneous identical requests.
        tx.rollback().await?;
        let proposal = self.propose_provisioning(request.plan.clone()).await?;
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let inserted = sqlx::query("INSERT INTO planning_drafts(id,request,proposal) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(serde_json::json!(request.plan)).bind(serde_json::json!(proposal))
            .execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::DraftConflict);
        }
        let result = draft_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn planning_draft(&self, id: Uuid) -> Result<PlanningDraft, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        draft_in(&mut tx, id).await
    }

    pub async fn apply_planning_draft(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
    ) -> Result<PlanningDraft, LedgerError> {
        validate_basis(&request)?;
        let payload = serde_json::json!({"version":1,"apply_planning_draft":id,"request":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let draft = pending_in(&mut tx, id).await?;
        lock_observations(&mut tx, &draft.request).await?;
        let fresh = propose_in(&mut tx, draft.request.clone()).await?;
        let mut saved = serde_json::json!(draft.proposal);
        let mut observed = serde_json::json!(fresh);
        saved.as_object_mut().unwrap().remove("observed_at");
        observed.as_object_mut().unwrap().remove("observed_at");
        if saved != observed {
            return Err(LedgerError::DraftStale);
        }
        if fresh.accounting.iter().any(|gap| gap.shortage != "0") {
            return Err(LedgerError::InsufficientFunds);
        }
        if fresh
            .needs
            .iter()
            .all(|n| n.stock.is_empty() && n.production.is_empty())
        {
            return Err(LedgerError::DraftEmpty);
        }
        close_in(&mut tx, id, "applied", &request.basis).await?;
        for (ni, line) in draft.proposal.needs.into_iter().enumerate() {
            for (ai, stock) in line.stock.into_iter().enumerate() {
                let attempt = reserve_need_in(
                    &mut tx,
                    line.need,
                    NeedReservationRequest {
                        resource: stock.resource,
                        quantity: Amount::parse(&stock.quantity)
                            .map_err(|_| LedgerError::InvalidStoredResponse)?,
                        basis: request.basis.clone(),
                    },
                )
                .await?;
                let reservation = attempt.reservation_id.ok_or(LedgerError::DraftStale)?;
                sqlx::query("INSERT INTO planning_actions(draft_id,need_position,action_position,kind,need_id,reservation_id,attempt_id) VALUES ($1,$2,$3,'stock',$4,$5,$6)")
                    .bind(id).bind(ni as i32).bind(ai as i32).bind(line.need).bind(reservation).bind(attempt.id)
                    .execute(&mut *tx).await?;
            }
            for (ai, production) in line.production.into_iter().enumerate() {
                let attempt = plan_production_in(&mut tx, production.request).await?;
                let plan = attempt.plan_id.ok_or(LedgerError::DraftStale)?;
                sqlx::query("INSERT INTO planning_actions(draft_id,need_position,action_position,kind,need_id,production_plan_id) VALUES ($1,$2,$3,'production',$4,$5)")
                    .bind(id).bind(ni as i32).bind(ai as i32).bind(line.need).bind(plan)
                    .execute(&mut *tx).await?;
            }
        }
        let result = draft_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn discard_planning_draft(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
    ) -> Result<PlanningDraft, LedgerError> {
        validate_basis(&request)?;
        let payload =
            serde_json::json!({"version":1,"discard_planning_draft":id,"request":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        pending_in(&mut tx, id).await?;
        close_in(&mut tx, id, "discarded", &request.basis).await?;
        let result = draft_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}
fn validate_basis(request: &ResolutionRequest) -> Result<(), LedgerError> {
    if !valid_text(&request.basis, 1000) {
        return Err(LedgerError::Invalid("decision basis required"));
    }
    Ok(())
}
async fn pending_in(tx: &mut Tx<'_>, id: Uuid) -> Result<PlanningDraft, LedgerError> {
    sqlx::query("SELECT id FROM planning_drafts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    let draft = draft_in(tx, id).await?;
    if draft.state != "pending" {
        return Err(LedgerError::DraftClosed);
    }
    Ok(draft)
}
async fn close_in(tx: &mut Tx<'_>, id: Uuid, state: &str, basis: &str) -> Result<(), LedgerError> {
    sqlx::query("INSERT INTO planning_outcomes(draft_id,state,basis) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(state)
        .bind(basis)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn draft_in(tx: &mut Tx<'_>, id: Uuid) -> Result<PlanningDraft, LedgerError> {
    let row = sqlx::query("SELECT d.format_version,d.request,d.proposal,to_char(d.created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS created_at,COALESCE(o.state,'pending') AS state,o.basis FROM planning_drafts d LEFT JOIN planning_outcomes o ON o.draft_id=d.id WHERE d.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::DraftNotFound)?;
    let rows = sqlx::query("SELECT need_id,kind,reservation_id,attempt_id,production_plan_id FROM planning_actions WHERE draft_id=$1 ORDER BY need_position,CASE kind WHEN 'stock' THEN 0 ELSE 1 END,action_position")
        .bind(id).fetch_all(&mut **tx).await?;
    let actions = rows
        .into_iter()
        .map(|r| {
            Ok(DraftAction {
                need: r.try_get("need_id")?,
                kind: r.try_get("kind")?,
                reservation: r.try_get("reservation_id")?,
                attempt: r.try_get("attempt_id")?,
                production_plan: r.try_get("production_plan_id")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(PlanningDraft {
        id,
        format_version: row.try_get("format_version")?,
        state: row.try_get("state")?,
        created_at: row.try_get("created_at")?,
        request: decode(row.try_get("request")?)?,
        proposal: decode(row.try_get("proposal")?)?,
        outcome_basis: row.try_get("basis")?,
        actions,
    })
}

async fn lock_observations(tx: &mut Tx<'_>, request: &PlanningRequest) -> Result<(), LedgerError> {
    let needs: Vec<_> = request.needs.iter().map(|n| n.need).collect();
    let mut capacities = BTreeSet::new();
    let mut resources = BTreeSet::new();
    for need in &request.needs {
        resources.extend(need.resources.iter().copied());
        for process in &need.production {
            capacities.insert(process.capacity);
            resources.extend(process.inputs.iter().map(|i| i.resource));
        }
    }
    // Batch order extends existing single-operation orders: needs, capacities,
    // accounts, resources. Sort each class globally, including unused candidates.
    let mut accounts: Vec<Uuid> =
        sqlx::query_scalar("SELECT resident_id FROM needs WHERE id=ANY($1) ORDER BY id FOR UPDATE")
            .bind(&needs)
            .fetch_all(&mut **tx)
            .await?;
    // Terminal production operations also lock capacity. Include outstanding
    // commitments even when a new request omits their production candidates.
    capacities.extend(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT capacity_id FROM need_production_forecasts WHERE need_id=ANY($1)",
        )
        .bind(&needs)
        .fetch_all(&mut **tx)
        .await?,
    );
    sqlx::query("SELECT id FROM production_capacities WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(capacities.into_iter().collect::<Vec<_>>())
        .fetch_all(&mut **tx)
        .await?;
    let resources: Vec<_> = resources.into_iter().collect();
    accounts.extend(
        sqlx::query_scalar::<_, Uuid>("SELECT producer_id FROM resources WHERE id=ANY($1)")
            .bind(&resources)
            .fetch_all(&mut **tx)
            .await?,
    );
    lock_accounts(tx, &accounts).await?;
    sqlx::query("SELECT id FROM resources WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(&resources)
        .fetch_all(&mut **tx)
        .await?;
    Ok(())
}
