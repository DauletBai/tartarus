use crate::{
    dated::DatedFlow,
    ledger::{Ledger, LedgerError, Tx, decode, finish},
    scenarios::*,
};
use serde_json::json;
use sqlx::Row;
use std::collections::BTreeSet;
use uuid::Uuid;

fn basis(s: &str) -> Result<(), LedgerError> {
    if s.trim().is_empty() || s.len() > 1000 {
        return Err(LedgerError::Invalid("scenario basis must be 1..1000 bytes"));
    }
    Ok(())
}
fn encode(flow: &DatedFlow) -> Result<String, LedgerError> {
    let bytes = flow
        .checkpoint()
        .map_err(|_| LedgerError::Invalid("invalid scenario state"))?;
    if bytes.len() > MAX_CHECKPOINT_BYTES {
        return Err(LedgerError::Invalid("scenario checkpoint exceeds 256 KiB"));
    }
    String::from_utf8(bytes).map_err(|_| LedgerError::InvalidStoredResponse)
}
fn replay(
    snapshot: &ScenarioSnapshot,
    request: &ScenarioProposal,
) -> Result<ScenarioSnapshot, LedgerError> {
    basis(&request.basis)?;
    if request.commands.is_empty() || request.commands.len() > MAX_COMMANDS {
        return Err(LedgerError::Invalid("scenario requires 1..64 commands"));
    }
    if request.expected_generation != snapshot.generation {
        return Err(LedgerError::ScenarioStale);
    }
    // Both inputs here are read from the approved database anchor, never supplied
    // by the requesting client. New states can only be derived by command replay.
    let bytes = snapshot.checkpoint.as_bytes();
    let mut flow = DatedFlow::restore(
        bytes,
        bytes,
        &snapshot.definition.id.to_string(),
        snapshot.flow_revision,
    )
    .map_err(|_| LedgerError::InvalidStoredResponse)?;
    for command in &request.commands {
        command
            .execute(&mut flow)
            .map_err(|_| LedgerError::Invalid("infeasible dated scenario command"))?;
    }
    let revision = flow.balance().revision;
    i64::try_from(revision).map_err(|_| LedgerError::Overflow)?;
    Ok(ScenarioSnapshot {
        definition: snapshot.definition.clone(),
        generation: snapshot
            .generation
            .checked_add(1)
            .ok_or(LedgerError::Overflow)?,
        flow_revision: revision,
        checkpoint: encode(&flow)?,
    })
}
impl Ledger {
    pub async fn create_dated_scenario(
        &self,
        key: &str,
        definition: ScenarioDefinition,
    ) -> Result<ScenarioSnapshot, LedgerError> {
        basis(&definition.basis)?;
        let unique: BTreeSet<_> = definition.resources.iter().collect();
        if definition.format_version != 1
            || unique.len() != definition.resources.len()
            || definition
                .resources
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 128 || s != s.trim())
        {
            return Err(LedgerError::Invalid("scenario version or resource names"));
        }
        let flow = DatedFlow::new(
            &definition.id.to_string(),
            &definition.unit,
            definition.resources.len(),
        )
        .map_err(|_| LedgerError::Invalid("scenario unit or resource count"))?;
        let initial = encode(&flow)?;
        let payload = json!({"version":1,"create_dated_scenario":definition});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        let inserted = sqlx::query("INSERT INTO dated_scenarios(id,definition,initial) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(definition.id).bind(json!(definition)).bind(initial.as_bytes()).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ScenarioConflict);
        }
        finish(
            tx,
            key,
            &payload,
            ScenarioSnapshot {
                definition,
                generation: 0,
                flow_revision: 0,
                checkpoint: initial,
            },
        )
        .await
    }
    /// None selects the authoritative current generation; explicit generations
    /// are historical reads and never an authorization to roll the head backward.
    pub async fn dated_scenario(
        &self,
        id: Uuid,
        generation: Option<i64>,
    ) -> Result<ScenarioSnapshot, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        snapshot_in(&mut tx, id, generation).await
    }
    pub async fn propose_dated_scenario(
        &self,
        key: &str,
        id: Uuid,
        request: ScenarioProposal,
    ) -> Result<ScenarioDraft, LedgerError> {
        let payload = json!({"version":1,"propose_dated_scenario":id,"request":request});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        lock(&mut tx, id).await?;
        let before = snapshot_in(&mut tx, id, None).await?;
        let candidate = replay(&before, &request)?;
        let inserted = sqlx::query("INSERT INTO dated_scenario_drafts(id,scenario_id,base_generation,request,candidate,flow_revision) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(id).bind(request.expected_generation).bind(json!(request))
            .bind(candidate.checkpoint.as_bytes()).bind(candidate.flow_revision as i64).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ScenarioConflict);
        }
        let draft = draft_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, draft).await
    }
    pub async fn dated_scenario_draft(&self, id: Uuid) -> Result<ScenarioDraft, LedgerError> {
        let mut tx = self.pool.begin().await?;
        draft_in(&mut tx, id).await
    }
    pub async fn resolve_dated_scenario(
        &self,
        key: &str,
        id: Uuid,
        decision: ScenarioDecision,
    ) -> Result<ScenarioDraft, LedgerError> {
        basis(&decision.basis)?;
        let payload = json!({"version":1,"resolve_dated_scenario":id,"decision":decision});
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
            let current = snapshot_in(&mut tx, draft.scenario_id, None).await?;
            let checked = replay(&current, &draft.request)?;
            if checked != draft.candidate {
                return Err(LedgerError::InvalidStoredResponse);
            }
            Some(checked.generation)
        } else {
            None
        };
        sqlx::query("INSERT INTO dated_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,$3,$4,$5)")
            .bind(id).bind(draft.scenario_id).bind(generation).bind(if decision.apply { "applied" } else { "discarded" })
            .bind(&decision.basis).execute(&mut *tx).await?;
        let result = draft_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}
async fn lock(tx: &mut Tx<'_>, id: Uuid) -> Result<(), LedgerError> {
    sqlx::query("SELECT id FROM dated_scenarios WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(LedgerError::ScenarioNotFound)?;
    Ok(())
}
async fn snapshot_in(
    tx: &mut Tx<'_>,
    id: Uuid,
    generation: Option<i64>,
) -> Result<ScenarioSnapshot, LedgerError> {
    let row = sqlx::query("SELECT definition,initial FROM dated_scenarios WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(LedgerError::ScenarioNotFound)?;
    let definition = decode(row.try_get("definition")?)?;
    if generation == Some(0) {
        return snapshot(definition, 0, 0, row.try_get("initial")?);
    }
    let selected = sqlx::query("SELECT o.generation,d.flow_revision,d.candidate FROM dated_scenario_outcomes o JOIN dated_scenario_drafts d ON d.id=o.draft_id WHERE o.scenario_id=$1 AND o.state='applied' AND ($2::bigint IS NULL OR o.generation=$2) ORDER BY o.generation DESC LIMIT 1")
        .bind(id).bind(generation).fetch_optional(&mut **tx).await?;
    match selected {
        Some(r) => snapshot(
            definition,
            r.try_get("generation")?,
            r.try_get("flow_revision")?,
            r.try_get("candidate")?,
        ),
        None if generation.is_none() => snapshot(definition, 0, 0, row.try_get("initial")?),
        None => Err(LedgerError::ScenarioNotFound),
    }
}
fn snapshot(
    definition: ScenarioDefinition,
    generation: i64,
    revision: i64,
    bytes: Vec<u8>,
) -> Result<ScenarioSnapshot, LedgerError> {
    let flow_revision = u64::try_from(revision).map_err(|_| LedgerError::InvalidStoredResponse)?;
    if bytes.len() > MAX_CHECKPOINT_BYTES {
        return Err(LedgerError::InvalidStoredResponse);
    }
    let flow = DatedFlow::restore(&bytes, &bytes, &definition.id.to_string(), flow_revision)
        .map_err(|_| LedgerError::InvalidStoredResponse)?;
    // Binding is independently checked rather than trusting a matching scope alone.
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| LedgerError::InvalidStoredResponse)?;
    if value["unit"] != json!(definition.unit)
        || value["resources"] != json!(definition.resources.len())
        || definition.format_version != 1
    {
        return Err(LedgerError::InvalidStoredResponse);
    }
    Ok(ScenarioSnapshot {
        definition,
        generation,
        flow_revision,
        checkpoint: encode(&flow)?,
    })
}
async fn draft_in(tx: &mut Tx<'_>, id: Uuid) -> Result<ScenarioDraft, LedgerError> {
    let r = sqlx::query("SELECT d.scenario_id,d.request,d.base_generation,d.flow_revision,d.candidate,s.definition,COALESCE(o.state,'pending') AS state,o.basis FROM dated_scenario_drafts d JOIN dated_scenarios s ON s.id=d.scenario_id LEFT JOIN dated_scenario_outcomes o ON o.draft_id=d.id WHERE d.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ScenarioNotFound)?;
    let generation: i64 = r.try_get("base_generation")?;
    Ok(ScenarioDraft {
        scenario_id: r.try_get("scenario_id")?,
        request: decode(r.try_get("request")?)?,
        candidate: snapshot(
            decode(r.try_get("definition")?)?,
            generation.checked_add(1).ok_or(LedgerError::Overflow)?,
            r.try_get("flow_revision")?,
            r.try_get("candidate")?,
        )?,
        state: r.try_get("state")?,
        decision_basis: r.try_get("basis")?,
    })
}
