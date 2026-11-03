//! Read-only, transaction-consistent telemetry. No inference authorizes a write.
use crate::ledger::{Ledger, LedgerError};
use serde_json::{Value, json};

impl Ledger {
    pub async fn system_status(&self) -> Result<Value, LedgerError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='3s'")
            .execute(&mut *tx)
            .await?;
        let mut status: Value = sqlx::query_scalar(r#"
SELECT jsonb_build_object(
 'schema_version',1,
 'observed_at',to_char(transaction_timestamp() AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS.US"Z"'),
 'invariants',jsonb_build_object(
   'ledger_net',(SELECT COALESCE(sum(delta),0)::text FROM postings),
   'unbalanced_transactions',(SELECT count(*)::text FROM (SELECT t.id FROM journal_transactions t LEFT JOIN postings p ON p.transaction_id=t.id GROUP BY t.id HAVING COALESCE(sum(p.delta),0)<>0 OR count(p.account_id)<2) x),
   'account_violations',(SELECT count(*)::text FROM account_totals WHERE reserved<0 OR (kind<>'issuer' AND (balance<0 OR balance>9223372036854775807 OR reserved>balance))),
   'resource_violations',(SELECT count(*)::text FROM resource_totals WHERE reserved<0 OR consumed<0 OR reserved+consumed>initial_quantity),
   'capacity_violations',(SELECT count(*)::text FROM production_capacity_totals WHERE reserved<0 OR consumed<0 OR reserved+consumed>quantity),
   'need_violations',(SELECT count(*)::text FROM need_totals WHERE reserved<0 OR received<0 OR reserved+received>quantity)),
 'needs',COALESCE((SELECT jsonb_agg(x ORDER BY x.period,x.category,x.unit) FROM
   (SELECT period,category,unit,count(*)::text AS count,sum(quantity)::text AS requested,sum(reserved)::text AS reserved,sum(received)::text AS received,sum(quantity-received)::text AS unmet,sum(quantity-received-reserved)::text AS uncovered FROM need_totals GROUP BY period,category,unit) x),'[]'::jsonb),
 'coordination',jsonb_build_object(
   'pending_events',(SELECT count(*)::text FROM coordination_events WHERE NOT dispatched),
   'queued',(SELECT count(*)::text FROM coordination_jobs WHERE state='queued'),
   'running',(SELECT count(*)::text FROM coordination_jobs WHERE state='running'),
   'failed',(SELECT count(*)::text FROM coordination_jobs WHERE state='failed'),
   'expired_leases',(SELECT count(*)::text FROM coordination_jobs WHERE state='running' AND lease_until<transaction_timestamp()),
   'completed',(SELECT count(*)::text FROM coordination_jobs WHERE state='completed'),
   'oldest_pending_seconds',(SELECT COALESCE(GREATEST(0,extract(epoch FROM transaction_timestamp()-min(created_at))),0)::text FROM (
      SELECT created_at FROM coordination_jobs WHERE state IN ('queued','running') UNION ALL SELECT created_at FROM coordination_events WHERE NOT dispatched) q)),
 'physical_evidence',jsonb_build_object(
   'declared_lots',(SELECT count(*)::text FROM resources r WHERE NOT EXISTS(SELECT 1 FROM production_completions c WHERE c.output_resource_id=r.id)),
   'reported_completions',(SELECT count(*)::text FROM production_completions),
   'independently_verified',false),
 'authority','observation_only')
"#).fetch_one(&mut *tx).await?;
        let healthy = status["invariants"]
            .as_object()
            .is_some_and(|m| m.values().all(|v| v == "0"));
        status["invariants_passed"] = json!(healthy);
        let mut alerts = Vec::new();
        if !healthy {
            alerts.push("invariant_violation");
        }
        let queue = &status["coordination"];
        if queue["failed"] != "0" {
            alerts.push("calculation_failed");
        }
        if queue["expired_leases"] != "0" {
            alerts.push("worker_lease_expired");
        }
        let age = queue["oldest_pending_seconds"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        if age >= 5.0 {
            alerts.push("coordination_backlog_older_than_5s");
        }
        status["alerts"] = json!(alerts);
        status["limitations"] = json!([
            "physical_reports_are_not_independently_verified",
            "no_resident_identity_authorization",
            "snapshot_is_not_a_backup_or_a_proof_of_future_safety"
        ]);
        tx.commit().await?;
        Ok(status)
    }
}
