use crate::{
    ledger::{Ledger, LedgerError, Tx, decode, finish},
    model_scenarios::*,
    scenario_service,
    scenarios::{ScenarioDecision, ScenarioSnapshot},
};
use serde_json::json;
use sqlx::Row;
use tokio::sync::Semaphore;
use uuid::Uuid;

// Fixed process-wide CPU admission: no unbounded queue of matrix jobs.
pub(crate) static CALCULATIONS: Semaphore = Semaphore::const_new(4);
async fn checked(
    before: ModelVersion,
    request: ModelProposal,
    dated: Vec<ScenarioSnapshot>,
) -> Result<ModelVersion, LedgerError> {
    let permit = CALCULATIONS
        .try_acquire()
        .map_err(|_| LedgerError::ScenarioBusy)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        calculate(before, request, dated)
    })
    .await
    .map_err(|_| LedgerError::InvalidStoredResponse)?
}
impl Ledger {
    pub async fn create_model_scenario(
        &self,
        key: &str,
        definition: ModelDefinition,
    ) -> Result<ModelVersion, LedgerError> {
        definition.validate()?;
        let payload = json!({"version":1,"create_model_scenario":definition});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        for b in &definition.dated_bindings {
            let anchor = scenario_service::snapshot_in(&mut tx, b.scenario_id, Some(0)).await?;
            if b.model_resources.len() != anchor.definition.resources.len()
                || b.model_resources
                    .iter()
                    .any(|&r| definition.resources[r].unit != anchor.definition.unit)
            {
                return Err(LedgerError::Invalid("dated binding dimensions or units"));
            }
        }
        let inserted = sqlx::query(
            "INSERT INTO model_scenarios(id,definition) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(definition.id)
        .bind(json!(definition))
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
            ModelVersion {
                definition,
                generation: 0,
                checkpoint: None,
            },
        )
        .await
    }
    pub async fn model_scenario(
        &self,
        id: Uuid,
        generation: Option<i64>,
    ) -> Result<ModelVersion, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        snapshot(&mut tx, id, generation).await
    }
    pub async fn propose_model_scenario(
        &self,
        key: &str,
        id: Uuid,
        request: ModelProposal,
    ) -> Result<ModelDraft, LedgerError> {
        if serde_json::to_vec(&request)
            .map_err(|_| LedgerError::InvalidStoredResponse)?
            .len()
            > MAX_MODEL_REQUEST_BYTES
        {
            return Err(LedgerError::Invalid("model request exceeds 1 MiB"));
        }
        let payload = json!({"version":1,"propose_model_scenario":id,"request":request});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        lock(&mut tx, id).await?;
        let before = snapshot(&mut tx, id, None).await?;
        let dated = anchors(&mut tx, &before.definition).await?;
        let candidate = checked(before, request.clone(), dated).await?;
        let inserted=sqlx::query("INSERT INTO model_scenario_drafts(id,scenario_id,base_generation,request,candidate) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(id).bind(request.expected_generation).bind(json!(request))
            .bind(candidate.checkpoint.as_ref().ok_or(LedgerError::InvalidStoredResponse)?.as_bytes())
            .execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ScenarioConflict);
        }
        let draft = draft_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, draft).await
    }
    pub async fn model_scenario_draft(&self, id: Uuid) -> Result<ModelDraft, LedgerError> {
        let mut tx = self.pool.begin().await?;
        draft_in(&mut tx, id).await
    }
    pub async fn resolve_model_scenario(
        &self,
        key: &str,
        id: Uuid,
        decision: ScenarioDecision,
    ) -> Result<ModelDraft, LedgerError> {
        text(&decision.basis, 1000)?;
        let payload = json!({"version":1,"resolve_model_scenario":id,"decision":decision});
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
            let dated = anchors(&mut tx, &before.definition).await?;
            let validated = checked(before, draft.request.clone(), dated).await?;
            if validated.checkpoint != draft.candidate.checkpoint {
                return Err(LedgerError::InvalidStoredResponse);
            }
            Some(validated.generation)
        } else {
            None
        };
        sqlx::query("INSERT INTO model_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,$3,$4,$5)")
            .bind(id).bind(draft.scenario_id).bind(generation).bind(if decision.apply {"applied"}else{"discarded"})
            .bind(&decision.basis).execute(&mut *tx).await?;
        let result = draft_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}
pub(crate) async fn lock(tx: &mut Tx<'_>, id: Uuid) -> Result<(), LedgerError> {
    sqlx::query("SELECT id FROM model_scenarios WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(LedgerError::ScenarioNotFound)?;
    Ok(())
}
pub(crate) async fn anchors(
    tx: &mut Tx<'_>,
    definition: &ModelDefinition,
) -> Result<Vec<ScenarioSnapshot>, LedgerError> {
    let mut ids: Vec<_> = definition
        .dated_bindings
        .iter()
        .map(|b| b.scenario_id)
        .collect();
    ids.sort();
    for id in ids {
        scenario_service::lock(tx, id).await?;
    }
    let mut snapshots = Vec::new();
    for b in &definition.dated_bindings {
        snapshots.push(scenario_service::snapshot_in(tx, b.scenario_id, None).await?);
    }
    Ok(snapshots)
}
fn version(
    definition: ModelDefinition,
    generation: i64,
    bytes: Option<Vec<u8>>,
) -> Result<ModelVersion, LedgerError> {
    let v = ModelVersion {
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
) -> Result<ModelVersion, LedgerError> {
    let definition = decode(
        sqlx::query("SELECT definition FROM model_scenarios WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(LedgerError::ScenarioNotFound)?
            .try_get("definition")?,
    )?;
    if generation == Some(0) {
        return version(definition, 0, None);
    }
    let row=sqlx::query("SELECT o.generation,d.candidate FROM model_scenario_outcomes o JOIN model_scenario_drafts d ON d.id=o.draft_id WHERE o.scenario_id=$1 AND o.state='applied' AND ($2::bigint IS NULL OR o.generation=$2) ORDER BY o.generation DESC LIMIT 1")
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
async fn draft_in(tx: &mut Tx<'_>, id: Uuid) -> Result<ModelDraft, LedgerError> {
    let r=sqlx::query("SELECT d.scenario_id,d.request,d.base_generation,d.candidate,s.definition,COALESCE(o.state,'pending') AS state,o.basis FROM model_scenario_drafts d JOIN model_scenarios s ON s.id=d.scenario_id LEFT JOIN model_scenario_outcomes o ON o.draft_id=d.id WHERE d.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ScenarioNotFound)?;
    let generation: i64 = r.try_get("base_generation")?;
    Ok(ModelDraft {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn saturated_calculation_admission_rejects_without_queueing() {
        let permits = CALCULATIONS.acquire_many(4).await.unwrap();
        let before = ModelVersion {
            definition: ModelDefinition {
                id: Uuid::new_v4(),
                format_version: 1,
                resources: vec![],
                period_unit: "day".into(),
                time_origin: "fixture".into(),
                dated_bindings: vec![],
                basis: "fixture".into(),
            },
            generation: 0,
            checkpoint: None,
        };
        let request = ModelProposal {
            id: Uuid::new_v4(),
            expected_generation: 0,
            input: serde_json::from_str(include_str!("../fixtures/model-scenario.json")).unwrap(),
            basis: "fixture".into(),
        };
        assert!(matches!(
            checked(before, request, vec![]).await,
            Err(LedgerError::ScenarioBusy)
        ));
        drop(permits);
        assert_eq!(CALCULATIONS.available_permits(), 4);
    }
}
