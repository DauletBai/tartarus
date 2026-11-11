use crate::model_scenario_service::CALCULATIONS;
use crate::{
    coupled::*,
    ledger::{Ledger, LedgerError, Tx, decode, finish},
    model_scenario_service,
    model_scenarios::{ModelVersion, checkpoint as physical_checkpoint, text},
    scenarios::ScenarioDecision,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

async fn checked(
    before: CoupledVersion,
    request: CoupledProposal,
    physical: ModelVersion,
) -> Result<CoupledVersion, LedgerError> {
    let permit = CALCULATIONS
        .try_acquire()
        .map_err(|_| LedgerError::ScenarioBusy)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        calculate(before, request, physical)
    })
    .await
    .map_err(|_| LedgerError::InvalidStoredResponse)?
}
impl Ledger {
    pub async fn create_coupled_scenario(
        &self,
        key: &str,
        definition: CoupledDefinition,
    ) -> Result<CoupledVersion, LedgerError> {
        let payload = json!({"version":1,"create_coupled_scenario":definition});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        let physical = current_physical(&mut tx, definition.model_id).await?;
        definition.validate(&physical)?;
        let inserted = sqlx::query(
            "INSERT INTO coupled_scenarios(id,definition,model_id) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(definition.id)
        .bind(json!(definition))
        .bind(definition.model_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ScenarioConflict);
        }
        finish(
            tx,
            key,
            &payload,
            CoupledVersion {
                definition,
                generation: 0,
                checkpoint: None,
            },
        )
        .await
    }
    pub async fn coupled_scenario(
        &self,
        id: Uuid,
        generation: Option<i64>,
    ) -> Result<CoupledVersion, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        snapshot(&mut tx, id, generation).await
    }
    pub async fn propose_coupled_scenario(
        &self,
        key: &str,
        id: Uuid,
        request: CoupledProposal,
    ) -> Result<CoupledDraft, LedgerError> {
        if serde_json::to_vec(&request)
            .map_err(|_| LedgerError::InvalidStoredResponse)?
            .len()
            > (16 * 1024)
        {
            return Err(LedgerError::Invalid("coupled request exceeds 16 KiB"));
        }
        let payload = json!({"version":1,"propose_coupled_scenario":id,"request":request});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        lock(&mut tx, id).await?;
        let before = snapshot(&mut tx, id, None).await?;
        let physical = current_physical(&mut tx, before.definition.model_id).await?;
        let candidate = checked(before, request.clone(), physical).await?;
        let inserted=sqlx::query("INSERT INTO coupled_scenario_drafts(id,scenario_id,base_generation,request,candidate,model_id,model_generation) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(id).bind(request.expected_generation).bind(json!(request))
            .bind(candidate.checkpoint.as_ref().ok_or(LedgerError::InvalidStoredResponse)?.as_bytes())
            .bind(candidate.definition.model_id).bind(request.model_generation)
            .execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ScenarioConflict);
        }
        let draft = draft_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, draft).await
    }
    pub async fn coupled_scenario_draft(&self, id: Uuid) -> Result<CoupledDraft, LedgerError> {
        let mut tx = self.pool.begin().await?;
        draft_in(&mut tx, id).await
    }
    pub async fn resolve_coupled_scenario(
        &self,
        key: &str,
        id: Uuid,
        decision: ScenarioDecision,
    ) -> Result<CoupledDraft, LedgerError> {
        text(&decision.basis, 1000)?;
        let payload = json!({"version":1,"resolve_coupled_scenario":id,"decision":decision});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        let initial = draft_in(&mut tx, id).await?;
        lock(&mut tx, initial.scenario_id).await?;
        let draft = draft_in(&mut tx, id).await?;
        if draft.state != "pending" {
            return Err(LedgerError::ScenarioClosed);
        }
        let generation = if decision.apply {
            let before = snapshot(&mut tx, draft.scenario_id, None).await?;
            let physical = current_physical(&mut tx, before.definition.model_id).await?;
            let validated = checked(before, draft.request.clone(), physical).await?;
            if validated.checkpoint != draft.candidate.checkpoint {
                return Err(LedgerError::InvalidStoredResponse);
            }
            Some(validated.generation)
        } else {
            None
        };
        sqlx::query("INSERT INTO coupled_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,$3,$4,$5)")
            .bind(id).bind(draft.scenario_id).bind(generation).bind(if decision.apply {"applied"}else{"discarded"})
            .bind(&decision.basis).execute(&mut *tx).await?;
        let result = draft_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}
pub(crate) async fn lock(tx: &mut Tx<'_>, id: Uuid) -> Result<(), LedgerError> {
    sqlx::query("SELECT id FROM coupled_scenarios WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(LedgerError::ScenarioNotFound)?;
    Ok(())
}
async fn current_physical(tx: &mut Tx<'_>, id: Uuid) -> Result<ModelVersion, LedgerError> {
    model_scenario_service::lock(tx, id).await?;
    let physical = model_scenario_service::snapshot(tx, id, None).await?;
    let c = physical_checkpoint(&physical)?
        .ok_or(LedgerError::Invalid("approved physical model required"))?;
    let current = model_scenario_service::anchors(tx, &physical.definition).await?;
    if c.dated != current {
        return Err(LedgerError::ScenarioStale);
    }
    Ok(physical)
}
fn version(
    definition: CoupledDefinition,
    generation: i64,
    bytes: Option<Vec<u8>>,
) -> Result<CoupledVersion, LedgerError> {
    let v = CoupledVersion {
        definition,
        generation,
        checkpoint: bytes
            .map(String::from_utf8)
            .transpose()
            .map_err(|_| LedgerError::InvalidStoredResponse)?,
    };
    checkpoint(&v)?;
    Ok(v)
}
pub(crate) async fn snapshot(
    tx: &mut Tx<'_>,
    id: Uuid,
    generation: Option<i64>,
) -> Result<CoupledVersion, LedgerError> {
    let definition = decode(
        sqlx::query("SELECT definition FROM coupled_scenarios WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(LedgerError::ScenarioNotFound)?
            .try_get("definition")?,
    )?;
    if generation == Some(0) {
        return version(definition, 0, None);
    }
    let row=sqlx::query("SELECT o.generation,d.candidate FROM coupled_scenario_outcomes o JOIN coupled_scenario_drafts d ON d.id=o.draft_id WHERE o.scenario_id=$1 AND o.state='applied' AND ($2::bigint IS NULL OR o.generation=$2) ORDER BY o.generation DESC LIMIT 1")
        .bind(id).bind(generation).fetch_optional(&mut **tx).await?;
    match row {
        Some(r) => version(
            definition,
            r.try_get("generation")?,
            Some(r.try_get("candidate")?),
        ),
        None if generation.is_none() => version(definition, 0, None),
        None => Err(LedgerError::ScenarioNotFound),
    }
}
async fn draft_in(tx: &mut Tx<'_>, id: Uuid) -> Result<CoupledDraft, LedgerError> {
    let r=sqlx::query("SELECT d.scenario_id,d.request,d.base_generation,d.candidate,s.definition,COALESCE(o.state,'pending') AS state,o.basis FROM coupled_scenario_drafts d JOIN coupled_scenarios s ON s.id=d.scenario_id LEFT JOIN coupled_scenario_outcomes o ON o.draft_id=d.id WHERE d.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ScenarioNotFound)?;
    let generation: i64 = r.try_get("base_generation")?;
    Ok(CoupledDraft {
        scenario_id: r.try_get("scenario_id")?,
        request: decode(r.try_get("request")?)?,
        candidate: version(
            decode(r.try_get("definition")?)?,
            generation.checked_add(1).ok_or(LedgerError::Overflow)?,
            Some(r.try_get("candidate")?),
        )?,
        state: r.try_get("state")?,
        decision_basis: r.try_get("basis")?,
    })
}
