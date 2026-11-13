//! Append-only incident correction and bounded exact combination.
use crate::{
    ledger::{Ledger, LedgerError, decode, finish},
    model_scenario_service,
    model_scenarios::{checkpoint, text},
    service_feedback::combine_service_revisions,
    service_incidents::{IncidentRecord, record},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncidentCorrection {
    pub expected_generation: i64,
    pub replacement_id: Option<Uuid>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncidentDisposition {
    pub kind: String,
    pub model_generation: i64,
    pub replacement_id: Option<Uuid>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncidentCombination {
    pub id: Uuid,
    pub expected_generation: i64,
    pub incidents: Vec<Uuid>,
    pub basis: String,
}
fn invalid() -> LedgerError {
    LedgerError::Invalid("invalid incident correction or combination")
}
impl Ledger {
    pub async fn correct_service_incident(
        &self,
        key: &str,
        id: Uuid,
        request: IncidentCorrection,
    ) -> Result<IncidentRecord, LedgerError> {
        text(&request.basis, 1000)?;
        let payload = json!({"version":1,"correct_service_incident":id,"request":request});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        let first = record(&mut tx, id).await?;
        model_scenario_service::lock(&mut tx, first.model_id).await?;
        let old = record(&mut tx, id).await?;
        if old.disposition.is_some() {
            return Err(LedgerError::ScenarioClosed);
        }
        let current = model_scenario_service::snapshot(&mut tx, old.model_id, None).await?;
        if current.generation != request.expected_generation {
            return Err(LedgerError::ScenarioStale);
        }
        if let Some(replacement) = request.replacement_id {
            let new = record(&mut tx, replacement).await?;
            if new.model_id != old.model_id
                || new.sequence <= old.sequence
                || new.base_generation != current.generation
                || new.state != "open"
                || !new.supersedes.is_empty()
            {
                return Err(invalid());
            }
        }
        sqlx::query("INSERT INTO service_incident_supersessions(incident_id,model_id,model_generation,replacement_id,kind,basis) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(id).bind(old.model_id).bind(current.generation).bind(request.replacement_id)
            .bind(if request.replacement_id.is_some(){"replaced"}else{"withdrawn"}).bind(&request.basis).execute(&mut *tx).await?;
        let result = record(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
    pub async fn combine_service_incidents(
        &self,
        key: &str,
        model_id: Uuid,
        request: IncidentCombination,
    ) -> Result<IncidentRecord, LedgerError> {
        text(&request.basis, 1000)?;
        if !(2..=16).contains(&request.incidents.len())
            || request.incidents.iter().collect::<BTreeSet<_>>().len() != request.incidents.len()
        {
            return Err(invalid());
        }
        let payload = json!({"version":1,"combine_service_incidents":model_id,"request":request});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        model_scenario_service::lock(&mut tx, model_id).await?;
        let before = model_scenario_service::snapshot(&mut tx, model_id, None).await?;
        if before.generation != request.expected_generation {
            return Err(LedgerError::ScenarioStale);
        }
        let c = checkpoint(&before)?.ok_or_else(invalid)?;
        let mut revisions = Vec::new();
        for &id in &request.incidents {
            let member = record(&mut tx, id).await?;
            if member.model_id != model_id
                || member.base_generation != before.generation
                || member.state != "open"
            {
                return Err(invalid());
            }
            revisions.push(member.revision);
        }
        let revision = combine_service_revisions(
            c.input.services.as_ref().ok_or_else(invalid)?,
            &revisions,
            request.basis.clone(),
            format!("qazna:incident-combination:{}", request.id),
        )?;
        if revision.boundary < c.input.boundary
            || serde_json::to_vec(&revision).map_err(|_| invalid())?.len() > 15 * 1024
        {
            return Err(invalid());
        }
        let n=sqlx::query("INSERT INTO service_incidents(id,model_id,base_generation,revision) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(model_id).bind(before.generation).bind(json!(revision)).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(LedgerError::ScenarioConflict);
        }
        for &id in &request.incidents {
            sqlx::query("INSERT INTO service_incident_supersessions(incident_id,model_id,model_generation,replacement_id,kind,basis) VALUES($1,$2,$3,$4,'merged',$5)")
                .bind(id).bind(model_id).bind(before.generation).bind(request.id).bind(&request.basis).execute(&mut *tx).await?;
        }
        let result = record(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }
}
