//! Durable declarations and links to independently reviewed physical/monetary
//! plans. Neither journal entry nor plan link posts money or certifies recovery.
use crate::incident_revisions::IncidentDisposition;
use crate::{
    coupled, coupled_service,
    ledger::{Ledger, LedgerError, Tx, decode, finish},
    model_scenario_service,
    model_scenarios::{checkpoint, text},
    services::ServiceRevision,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncidentDeclaration {
    pub id: Uuid,
    pub expected_generation: i64,
    pub revision: ServiceRevision,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncidentPlan {
    pub model_generation: i64,
    pub coupled_id: Uuid,
    pub coupled_generation: i64,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncidentRecord {
    pub id: Uuid,
    pub sequence: i64,
    pub model_id: Uuid,
    pub base_generation: i64,
    pub revision: ServiceRevision,
    pub state: String,
    pub plan: Option<IncidentPlan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<IncidentDisposition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<Uuid>,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncidentQuery {
    pub after: Option<i64>,
    pub limit: Option<usize>,
}
fn invalid() -> LedgerError {
    LedgerError::Invalid("invalid service incident declaration or plan link")
}
impl Ledger {
    pub async fn declare_service_incident(
        &self,
        key: &str,
        model_id: Uuid,
        request: IncidentDeclaration,
    ) -> Result<IncidentRecord, LedgerError> {
        if request.revision.changes.is_empty()
            || request.revision.changes.len() > 128
            || serde_json::to_vec(&request).map_err(|_| invalid())?.len() > 16 * 1024
        {
            return Err(invalid());
        }
        let payload = json!({"version":1,"declare_service_incident":model_id,"request":request});
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
        if request.revision.boundary < c.input.boundary {
            return Err(invalid());
        }
        let old = c.input.services.as_ref().ok_or_else(invalid)?;
        old.with_revision(request.revision.clone())
            .map_err(|_| invalid())?;
        // Deliberately do not require a feasible physical schedule here.
        let count=sqlx::query("INSERT INTO service_incidents(id,model_id,base_generation,revision) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(model_id).bind(before.generation).bind(json!(request.revision)).execute(&mut *tx).await?.rows_affected();
        if count == 0 {
            return Err(LedgerError::ScenarioConflict);
        }
        let record = record(&mut tx, request.id).await?;
        finish(tx, key, &payload, record).await
    }
    pub async fn service_incident(&self, id: Uuid) -> Result<IncidentRecord, LedgerError> {
        let mut tx = self.pool.begin().await?;
        record(&mut tx, id).await
    }
    pub async fn service_incidents(
        &self,
        model_id: Uuid,
        query: IncidentQuery,
    ) -> Result<Vec<IncidentRecord>, LedgerError> {
        let limit = query.limit.unwrap_or(50);
        let after = query.after.unwrap_or(0);
        if limit == 0 || limit > 100 || after < 0 {
            return Err(invalid());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM model_scenarios WHERE id=$1)")
                .bind(model_id)
                .fetch_one(&mut *tx)
                .await?;
        if !exists {
            return Err(LedgerError::ScenarioNotFound);
        }
        let ids:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM service_incidents WHERE model_id=$1 AND sequence>$2 ORDER BY sequence LIMIT $3")
            .bind(model_id).bind(after).bind(limit as i64).fetch_all(&mut *tx).await?;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            result.push(record(&mut tx, id).await?);
        }
        Ok(result)
    }
    pub async fn link_service_incident_plan(
        &self,
        key: &str,
        id: Uuid,
        request: IncidentPlan,
    ) -> Result<IncidentRecord, LedgerError> {
        text(&request.basis, 1000)?;
        let payload = json!({"version":1,"link_service_incident_plan":id,"request":request});
        let (mut tx, previous) = self.start(key, &payload).await?;
        if let Some(value) = previous {
            return decode(value);
        }
        let incident = record(&mut tx, id).await?;
        coupled_service::lock(&mut tx, request.coupled_id).await?;
        model_scenario_service::lock(&mut tx, incident.model_id).await?;
        if record(&mut tx, id).await?.state != "open" {
            return Err(LedgerError::ScenarioClosed);
        }
        let physical = model_scenario_service::snapshot(&mut tx, incident.model_id, None).await?;
        let money = coupled_service::snapshot(&mut tx, request.coupled_id, None).await?;
        if physical.generation != request.model_generation
            || money.generation != request.coupled_generation
        {
            return Err(LedgerError::ScenarioStale);
        }
        if physical.generation <= incident.base_generation
            || money.definition.model_id != incident.model_id
        {
            return Err(invalid());
        }
        let baseline = model_scenario_service::snapshot(
            &mut tx,
            incident.model_id,
            Some(incident.base_generation),
        )
        .await?;
        let b = checkpoint(&baseline)?.ok_or_else(invalid)?;
        let p = checkpoint(&physical)?.ok_or_else(invalid)?;
        let anchors = model_scenario_service::anchors(&mut tx, &physical.definition).await?;
        if p.dated != anchors {
            return Err(LedgerError::ScenarioStale);
        }
        let old = &b.input.services.as_ref().ok_or_else(invalid)?.revisions;
        let revisions = &p.input.services.as_ref().ok_or_else(invalid)?.revisions;
        let c = coupled::checkpoint(&money)?.ok_or_else(invalid)?;
        if revisions.get(..old.len()) != Some(old.as_slice())
            || revisions.get(old.len()) != Some(&incident.revision)
            || json!(c.evidence.physical) != json!(physical)
            || !c.evidence.all_scheduled_payments_settled
            || !c.evidence.failed.is_empty()
        {
            return Err(invalid());
        }
        sqlx::query("INSERT INTO service_incident_plans(incident_id,model_id,model_generation,coupled_id,coupled_generation,basis) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(id).bind(incident.model_id).bind(request.model_generation).bind(request.coupled_id).bind(request.coupled_generation).bind(&request.basis).execute(&mut *tx).await?;
        let result = record(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}
pub(crate) async fn record(tx: &mut Tx<'_>, id: Uuid) -> Result<IncidentRecord, LedgerError> {
    let row=sqlx::query("SELECT i.id,i.sequence,i.model_id,i.base_generation,i.revision,p.model_generation,p.coupled_id,p.coupled_generation,p.basis,s.kind,s.model_generation AS disposition_generation,s.replacement_id,s.basis AS disposition_basis,ARRAY(SELECT x.incident_id FROM service_incident_supersessions x WHERE x.replacement_id=i.id ORDER BY x.incident_id) AS supersedes FROM service_incidents i LEFT JOIN service_incident_plans p ON p.incident_id=i.id LEFT JOIN service_incident_supersessions s ON s.incident_id=i.id WHERE i.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ScenarioNotFound)?;
    let generation: Option<i64> = row.try_get("model_generation")?;
    let plan = if let Some(model_generation) = generation {
        Some(IncidentPlan {
            model_generation,
            coupled_id: row.try_get("coupled_id")?,
            coupled_generation: row.try_get("coupled_generation")?,
            basis: row.try_get("basis")?,
        })
    } else {
        None
    };
    let kind: Option<String> = row.try_get("kind")?;
    let disposition = kind
        .map(|kind| -> Result<IncidentDisposition, LedgerError> {
            Ok(IncidentDisposition {
                kind,
                model_generation: row.try_get("disposition_generation")?,
                replacement_id: row.try_get("replacement_id")?,
                basis: row.try_get("disposition_basis")?,
            })
        })
        .transpose()?;
    let state = if let Some(d) = &disposition {
        if d.replacement_id.is_some() {
            "superseded"
        } else {
            "withdrawn"
        }
    } else if plan.is_some() {
        "plan_linked"
    } else {
        "open"
    };
    Ok(IncidentRecord {
        id: row.try_get("id")?,
        sequence: row.try_get("sequence")?,
        model_id: row.try_get("model_id")?,
        base_generation: row.try_get("base_generation")?,
        revision: decode(row.try_get("revision")?)?,
        state: state.into(),
        disposition,
        supersedes: row.try_get("supersedes")?,
        plan,
    })
}
