use crate::{
    coordination::*,
    ledger::{Ledger, LedgerError, Tx, decode, finish},
    planning::{PlanningRequest, ProvisioningProposal},
    provisioning::{ResolutionRequest, valid_text},
};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

struct Claim {
    job: Uuid,
    monitor: Uuid,
    token: Uuid,
    request: PlanningRequest,
}
fn comparable(proposal: &ProvisioningProposal) -> Value {
    let mut value = serde_json::json!(proposal);
    value.as_object_mut().unwrap().remove("observed_at");
    value
}
impl Ledger {
    pub async fn create_monitor(
        &self,
        key: &str,
        request: MonitorRequest,
    ) -> Result<Monitor, LedgerError> {
        let payload = serde_json::json!({"version":1,"coordination_monitor":request});
        let (tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        tx.rollback().await?;
        self.propose_provisioning(request.plan.clone()).await?;
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let inserted = sqlx::query(
            "INSERT INTO coordination_monitors(id,request) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(request.id)
        .bind(serde_json::json!(request.plan))
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if inserted == 0 {
            return Err(LedgerError::MonitorConflict);
        }
        sqlx::query("INSERT INTO coordination_jobs(monitor_id) VALUES ($1)")
            .bind(request.id)
            .execute(&mut *tx)
            .await?;
        let result = monitor_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }
    pub async fn monitor(&self, id: Uuid) -> Result<Monitor, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        monitor_in(&mut tx, id).await
    }
    pub async fn stop_monitor(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
    ) -> Result<Monitor, LedgerError> {
        if !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("stop basis required"));
        }
        let payload = serde_json::json!({"version":1,"coordination_stop":id,"request":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        sqlx::query("SELECT id FROM coordination_monitors WHERE id=$1 FOR UPDATE")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let monitor = monitor_in(&mut tx, id).await?;
        if monitor.state == "stopped" {
            return Err(LedgerError::MonitorStopped);
        }
        sqlx::query("INSERT INTO coordination_stops(monitor_id,basis) VALUES ($1,$2)")
            .bind(id)
            .bind(request.basis)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE coordination_jobs SET state='cancelled',lease_token=NULL,lease_until=NULL WHERE monitor_id=$1 AND state IN ('queued','running')")
            .bind(id).execute(&mut *tx).await?;
        let result = monitor_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
    /// Dispatch committed commands, claim one job and perform exact calculations.
    /// This method never applies a draft or calls an external model.
    pub async fn coordinate_once(&self) -> Result<Option<CoordinationResult>, LedgerError> {
        self.dispatch_events().await?;
        let Some(claim) = self.claim_job().await? else {
            return Ok(None);
        };
        let calculation = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            self.propose_provisioning(claim.request.clone()),
        )
        .await;
        match calculation {
            Ok(Ok(proposal)) => self.complete_job(claim, proposal).await.map(Some),
            other => {
                let code = match other {
                    Err(_) => "calculation_timeout",
                    Ok(Err(LedgerError::Database(_))) => "storage_unavailable",
                    _ => "calculation_invalid",
                };
                sqlx::query("UPDATE coordination_jobs SET state=CASE WHEN attempts>=3 THEN 'failed' ELSE 'queued' END,lease_token=NULL,lease_until=NULL,error_code=$3,available_at=clock_timestamp()+interval '1 second' WHERE id=$1 AND lease_token=$2 AND state='running'")
                    .bind(claim.job).bind(claim.token).bind(code).execute(&self.pool).await?;
                Err(LedgerError::CalculationFailed)
            }
        }
    }
    async fn dispatch_events(&self) -> Result<(), LedgerError> {
        let mut tx = self.pool.begin().await?;
        let keys:Vec<String>=sqlx::query_scalar("SELECT command_key FROM coordination_events WHERE NOT dispatched ORDER BY created_at,command_key LIMIT 64 FOR UPDATE SKIP LOCKED")
            .fetch_all(&mut *tx).await?;
        for key in keys {
            // Serialize against stop/finalization before a fresh stopped-state read.
            // A query snapshot taken before waiting on an FK lock is insufficient.
            sqlx::query("SELECT id FROM coordination_monitors ORDER BY id FOR KEY SHARE")
                .fetch_all(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO coordination_jobs(monitor_id,command_key) SELECT m.id,$1 FROM coordination_monitors m WHERE NOT EXISTS(SELECT 1 FROM coordination_stops s WHERE s.monitor_id=m.id) ON CONFLICT DO NOTHING")
                .bind(&key).execute(&mut *tx).await?;
            sqlx::query("UPDATE coordination_events SET dispatched=true WHERE command_key=$1")
                .bind(key)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    async fn claim_job(&self) -> Result<Option<Claim>, LedgerError> {
        let mut tx = self.pool.begin().await?;
        // Monitor before job is also the stop/finalization order. Skip monitors
        // with live leases so multiple processes can work on different scopes.
        let row=sqlx::query("SELECT m.id,m.request FROM coordination_monitors m WHERE NOT EXISTS(SELECT 1 FROM coordination_stops s WHERE s.monitor_id=m.id) AND EXISTS(SELECT 1 FROM coordination_jobs j WHERE j.monitor_id=m.id AND ((j.state='queued' AND j.available_at<=clock_timestamp()) OR (j.state='running' AND j.lease_until<=clock_timestamp()))) AND NOT EXISTS(SELECT 1 FROM coordination_jobs j WHERE j.monitor_id=m.id AND j.state='running' AND j.lease_until>clock_timestamp()) ORDER BY m.created_at,m.id LIMIT 1 FOR UPDATE OF m SKIP LOCKED")
            .fetch_optional(&mut *tx).await?;
        let Some(row) = row else { return Ok(None) };
        let monitor: Uuid = row.try_get("id")?;
        let stopped:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM coordination_stops WHERE monitor_id=$1) OR EXISTS(SELECT 1 FROM coordination_jobs WHERE monitor_id=$1 AND state='running' AND lease_until>clock_timestamp())")
            .bind(monitor).fetch_one(&mut *tx).await?;
        if stopped {
            return Ok(None);
        }
        sqlx::query("UPDATE coordination_jobs SET state='failed',lease_token=NULL,lease_until=NULL,error_code='lease_expired' WHERE monitor_id=$1 AND state='running' AND lease_until<=clock_timestamp() AND attempts>=3")
            .bind(monitor).execute(&mut *tx).await?;
        let job:Option<Uuid>=sqlx::query_scalar("SELECT id FROM coordination_jobs WHERE monitor_id=$1 AND attempts<3 AND ((state='queued' AND available_at<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp())) ORDER BY created_at,id LIMIT 1 FOR UPDATE")
            .bind(monitor).fetch_optional(&mut *tx).await?;
        let Some(job) = job else {
            tx.commit().await?;
            return Ok(None);
        };
        let token = Uuid::new_v4();
        sqlx::query("UPDATE coordination_jobs SET state='running',attempts=attempts+1,lease_token=$2,lease_until=clock_timestamp()+interval '30 seconds',error_code=NULL WHERE id=$1")
            .bind(job).bind(token).execute(&mut *tx).await?;
        let request = decode(row.try_get("request")?)?;
        tx.commit().await?;
        Ok(Some(Claim {
            job,
            monitor,
            token,
            request,
        }))
    }
    async fn complete_job(
        &self,
        claim: Claim,
        proposal: ProvisioningProposal,
    ) -> Result<CoordinationResult, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM coordination_monitors WHERE id=$1 FOR UPDATE")
            .bind(claim.monitor)
            .execute(&mut *tx)
            .await?;
        let changed=sqlx::query("UPDATE coordination_jobs SET state='completed',lease_token=NULL,lease_until=NULL WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp() AND NOT EXISTS(SELECT 1 FROM coordination_stops WHERE monitor_id=$3)")
            .bind(claim.job).bind(claim.token).bind(claim.monitor).execute(&mut *tx).await?.rows_affected();
        if changed == 0 {
            return Err(LedgerError::CalculationLeaseLost);
        }
        let previous=sqlx::query("SELECT r.proposal,r.draft_id FROM coordination_results r JOIN coordination_jobs j ON j.id=r.job_id WHERE j.monitor_id=$1 ORDER BY r.sequence DESC LIMIT 1")
            .bind(claim.monitor).fetch_optional(&mut *tx).await?;
        let reusable = if let Some(row) = previous {
            let old: ProvisioningProposal = decode(row.try_get("proposal")?)?;
            if comparable(&old) == comparable(&proposal) {
                Some(row.try_get::<Uuid, _>("draft_id")?)
            } else {
                None
            }
        } else {
            None
        };
        let draft = if let Some(id) = reusable {
            id
        } else {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO planning_drafts(id,request,proposal) VALUES ($1,$2,$3)")
                .bind(id)
                .bind(serde_json::json!(claim.request))
                .bind(serde_json::json!(proposal))
                .execute(&mut *tx)
                .await?;
            id
        };
        sqlx::query("INSERT INTO coordination_results(job_id,engine,proposal,draft_id,changed) VALUES ($1,'ordered-planner-v2',$2,$3,$4)")
            .bind(claim.job).bind(serde_json::json!(proposal)).bind(draft).bind(reusable.is_none()).execute(&mut *tx).await?;
        let result = result_in(&mut tx, claim.job).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn coordination_result(&self, id: Uuid) -> Result<CoordinationResult, LedgerError> {
        let mut tx = self.pool.begin().await?;
        result_in(&mut tx, id).await
    }
    pub async fn record_ai_review(
        &self,
        key: &str,
        job: Uuid,
        request: ReviewRequest,
    ) -> Result<AiReview, LedgerError> {
        let r = &request.report;
        if !valid_text(&request.model, 100)
            || !valid_text(&r.summary, 2000)
            || !r.requires_operator_approval
            || r.findings.len() > 96
            || r.questions.len() > 16
            || r.questions.iter().any(|s| !valid_text(s, 1000))
        {
            return Err(LedgerError::Invalid(
                "bounded advisory report with operator approval required",
            ));
        }
        let payload = serde_json::json!({"version":1,"coordination_review":job,"request":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let calculation = result_in(&mut tx, job).await?;
        validate_report(&request.report, &calculation.proposal)?;
        let inserted=sqlx::query("INSERT INTO coordination_reviews(id,job_id,provider,model,report) VALUES ($1,$2,'codex-chatgpt',$3,$4) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(job).bind(&request.model).bind(serde_json::json!(request.report)).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ReviewConflict);
        }
        finish(
            tx,
            key,
            &payload,
            AiReview {
                id: request.id,
                job,
                provider: "codex-chatgpt".into(),
                model: request.model,
                report: request.report,
            },
        )
        .await
    }
    pub async fn ai_reviews(&self, job: Uuid) -> Result<Vec<AiReview>, LedgerError> {
        let mut tx = self.pool.begin().await?;
        result_in(&mut tx, job).await?;
        let rows=sqlx::query("SELECT id,provider,model,report FROM coordination_reviews WHERE job_id=$1 ORDER BY created_at,id").bind(job).fetch_all(&mut *tx).await?;
        rows.into_iter()
            .map(|r| {
                Ok(AiReview {
                    id: r.try_get("id")?,
                    job,
                    provider: r.try_get("provider")?,
                    model: r.try_get("model")?,
                    report: decode(r.try_get("report")?)?,
                })
            })
            .collect()
    }
}
async fn monitor_in(tx: &mut Tx<'_>, id: Uuid) -> Result<Monitor, LedgerError> {
    let row=sqlx::query("SELECT request,EXISTS(SELECT 1 FROM coordination_stops WHERE monitor_id=$1) AS stopped FROM coordination_monitors WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::MonitorNotFound)?;
    let counts=sqlx::query("SELECT count(*) FILTER(WHERE state='queued') AS queued,count(*) FILTER(WHERE state='running') AS running,count(*) FILTER(WHERE state='failed') AS failed FROM coordination_jobs WHERE monitor_id=$1")
        .bind(id).fetch_one(&mut **tx).await?;
    let latest:Option<Uuid>=sqlx::query_scalar("SELECT r.job_id FROM coordination_results r JOIN coordination_jobs j ON j.id=r.job_id WHERE j.monitor_id=$1 ORDER BY r.sequence DESC LIMIT 1")
        .bind(id).fetch_optional(&mut **tx).await?;
    let latest = if let Some(job) = latest {
        Some(result_in(tx, job).await?)
    } else {
        None
    };
    Ok(Monitor {
        id,
        state: if row.try_get("stopped")? {
            "stopped"
        } else {
            "active"
        }
        .into(),
        plan: decode(row.try_get("request")?)?,
        queued: counts.try_get("queued")?,
        running: counts.try_get("running")?,
        failed: counts.try_get("failed")?,
        latest,
    })
}
async fn result_in(tx: &mut Tx<'_>, id: Uuid) -> Result<CoordinationResult, LedgerError> {
    let row=sqlx::query("SELECT j.monitor_id,j.command_key,r.engine,r.proposal,r.draft_id,r.changed,to_char(r.created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS created_at FROM coordination_results r JOIN coordination_jobs j ON j.id=r.job_id WHERE r.job_id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::CalculationNotFound)?;
    Ok(CoordinationResult {
        job: id,
        monitor: row.try_get("monitor_id")?,
        trigger_command: row.try_get("command_key")?,
        engine: row.try_get("engine")?,
        proposal: decode(row.try_get("proposal")?)?,
        draft: row.try_get("draft_id")?,
        changed: row.try_get("changed")?,
        created_at: row.try_get("created_at")?,
    })
}

fn validate_report(report: &AiReport, proposal: &ProvisioningProposal) -> Result<(), LedgerError> {
    let mut seen = std::collections::BTreeSet::new();
    for finding in &report.findings {
        let expected = match finding.signal.as_str() {
            "physical_residual" => proposal
                .needs
                .iter()
                .find(|n| n.need == finding.subject)
                .map(|n| &n.residual),
            "production_forecast" => proposal
                .needs
                .iter()
                .find(|n| n.need == finding.subject)
                .map(|n| &n.expected_from_production),
            "accounting_shortage" => proposal
                .accounting
                .iter()
                .find(|g| g.resident == finding.subject)
                .map(|g| &g.shortage),
            _ => None,
        };
        if expected.is_none_or(|quantity| quantity == "0" || quantity != &finding.quantity)
            || !valid_text(&finding.explanation, 1000)
            || !seen.insert((finding.signal.clone(), finding.subject))
        {
            return Err(LedgerError::Invalid(
                "AI finding must cite a distinct nonzero signal with its exact recorded quantity",
            ));
        }
    }
    if serde_json::to_vec(report)
        .map_err(|_| LedgerError::InvalidStoredResponse)?
        .len()
        > 12000
    {
        return Err(LedgerError::Invalid("AI report exceeds storage contract"));
    }
    Ok(())
}
