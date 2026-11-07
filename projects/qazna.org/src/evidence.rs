//! Recorded attestations constrain use; they are not proof against collusion.
use crate::{
    domain::Amount,
    ledger::{Ledger, LedgerError, decode, finish},
    provisioning::valid_text,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipalRequest {
    pub id: Uuid,
    pub account: Uuid,
    pub role: String,
    pub basis: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementRequest {
    pub id: Uuid,
    pub decision: String,
    pub quantity: Amount,
    pub evidence_sha256: String,
    pub basis: String,
}
impl Ledger {
    /// Administrator CLI operation; never exposed to model or operator tokens.
    pub async fn revoke_principal(&self, principal: Uuid, basis: &str) -> Result<(), LedgerError> {
        if !valid_text(basis, 1000) {
            return Err(LedgerError::Invalid("revocation basis required"));
        }
        let mut tx = self.pool.begin().await?;
        let found: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM evidence_principals WHERE id=$1 FOR UPDATE")
                .bind(principal)
                .fetch_optional(&mut *tx)
                .await?;
        if found.is_none() {
            return Err(LedgerError::AccessDenied);
        }
        let old: Option<String> = sqlx::query_scalar(
            "SELECT basis FROM evidence_principal_revocations WHERE principal_id=$1",
        )
        .bind(principal)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(old) = old {
            if old != basis {
                return Err(LedgerError::EvidenceConflict);
            }
        } else {
            sqlx::query(
                "INSERT INTO evidence_principal_revocations(principal_id,basis) VALUES($1,$2)",
            )
            .bind(principal)
            .bind(basis)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn enable_evidence(&self, basis: &str) -> Result<(), LedgerError> {
        if !valid_text(basis, 1000) {
            return Err(LedgerError::Invalid("activation basis required"));
        }
        sqlx::query("INSERT INTO evidence_enforcement(basis) VALUES($1) ON CONFLICT DO NOTHING")
            .bind(basis)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn register_principal(
        &self,
        key: &str,
        request: PrincipalRequest,
    ) -> Result<PrincipalRequest, LedgerError> {
        if !matches!(
            request.role.as_str(),
            "inspector" | "custodian" | "recipient"
        ) || !valid_text(&request.basis, 1000)
        {
            return Err(LedgerError::Invalid(
                "valid principal role and basis required",
            ));
        }
        let payload = json!({"version":1,"evidence_principal":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(v) = replay {
            return decode(v);
        }
        let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM accounts WHERE id=$1")
            .bind(request.account)
            .fetch_optional(&mut *tx)
            .await?;
        if kind.as_deref().is_none_or(|k| {
            k == "issuer"
                || (request.role == "recipient" && k != "resident")
                || (request.role == "custodian" && k != "producer")
        }) {
            return Err(LedgerError::Invalid(
                "principal account does not match its role",
            ));
        }
        let n=sqlx::query("INSERT INTO evidence_principals(id,account_id,role,basis) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(request.id).bind(request.account).bind(&request.role).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if n == 0 {
            return Err(LedgerError::EvidenceConflict);
        }
        finish(tx, key, &payload, request).await
    }
    pub async fn record_evidence(
        &self,
        key: &str,
        principal: Uuid,
        kind: &str,
        id: Uuid,
        request: StatementRequest,
    ) -> Result<Value, LedgerError> {
        if !matches!(kind, "resource" | "delivery")
            || !matches!(request.decision.as_str(), "confirm" | "dispute")
            || !valid_text(&request.basis, 1000)
            || request.evidence_sha256.len() != 64
            || !request
                .evidence_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(LedgerError::Invalid(
                "valid subject, decision, evidence digest and basis required",
            ));
        }
        let payload = json!({"version":1,"evidence_statement":{"principal":principal,"kind":kind,"subject":id,"request":request}});
        let (mut tx, replay) = self.start(key, &payload).await?;
        active_principal(&mut tx, principal).await?;
        if let Some(v) = replay {
            return decode(v);
        }
        let eligible: bool = if kind == "resource" {
            sqlx::query("SELECT id FROM resources WHERE id=$1 FOR UPDATE")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_principals p,resources r WHERE p.id=$1 AND r.id=$2 AND p.role='inspector' AND p.account_id<>r.producer_id AND r.initial_quantity=$3)").bind(principal).bind(id).bind(request.quantity.units()).fetch_one(&mut *tx).await?
        } else {
            sqlx::query("SELECT id FROM reservations WHERE id=$1 FOR UPDATE")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_principals p,reservations r,resources s WHERE p.id=$1 AND r.id=$2 AND s.id=r.resource_id AND r.quantity=$3 AND ((p.role='recipient' AND p.account_id=r.resident_id) OR (p.role='custodian' AND p.account_id=s.producer_id)))").bind(principal).bind(id).bind(request.quantity.units()).fetch_one(&mut *tx).await?
        };
        if !eligible {
            return Err(LedgerError::Invalid(
                "principal or quantity does not match subject",
            ));
        }
        let n=sqlx::query("INSERT INTO evidence_statements(id,principal_id,resource_id,reservation_id,decision,quantity,evidence_sha256,basis) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(principal).bind(if kind=="resource" {Some(id)} else {None}).bind(if kind=="delivery" {Some(id)} else {None}).bind(&request.decision).bind(request.quantity.units()).bind(&request.evidence_sha256).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if n == 0 {
            return Err(LedgerError::EvidenceConflict);
        }
        finish(tx,key,&payload,json!({"id":request.id,"principal":principal,"kind":kind,"subject":id,"statement":request})).await
    }
    pub async fn evidence(&self, kind: &str, id: Uuid) -> Result<Value, LedgerError> {
        self.scoped_evidence(None, kind, id).await
    }
    pub async fn scoped_evidence(
        &self,
        actor: Option<Uuid>,
        kind: &str,
        id: Uuid,
    ) -> Result<Value, LedgerError> {
        if !matches!(kind, "resource" | "delivery") {
            return Err(LedgerError::Invalid("resource or delivery required"));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        if let Some(actor) = actor {
            let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_principals p WHERE p.id=$1 AND NOT EXISTS(SELECT 1 FROM evidence_principal_revocations v WHERE v.principal_id=p.id) AND (($2='resource' AND EXISTS(SELECT 1 FROM resources r WHERE r.id=$3 AND (p.role='inspector' OR (p.role='custodian' AND p.account_id=r.producer_id) OR (p.role='recipient' AND EXISTS(SELECT 1 FROM reservations h WHERE h.resource_id=r.id AND h.resident_id=p.account_id))))) OR ($2='delivery' AND EXISTS(SELECT 1 FROM reservations h JOIN resources r ON r.id=h.resource_id WHERE h.id=$3 AND ((p.role='recipient' AND p.account_id=h.resident_id) OR (p.role='custodian' AND p.account_id=r.producer_id))))))")
                .bind(actor).bind(kind).bind(id).fetch_one(&mut *tx).await?;
            if !allowed {
                return Err(LedgerError::AccessDenied);
            }
        }
        let exists: bool = if kind == "resource" {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM resources WHERE id=$1)")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
        } else {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reservations WHERE id=$1)")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
        };
        if !exists {
            return Err(LedgerError::ResourceNotFound);
        }
        let attested:bool=sqlx::query_scalar("SELECT CASE WHEN $1='resource' THEN resource_attested($2) ELSE delivery_attested($2) END").bind(kind).bind(id).fetch_one(&mut *tx).await?;
        let statements:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(jsonb_build_object('id',s.id,'principal',s.principal_id,'role',p.role,'decision',s.decision,'quantity',s.quantity::text,'evidence_sha256',s.evidence_sha256,'basis',s.basis,'created_at',s.created_at) ORDER BY s.created_at,s.id),'[]'::jsonb) FROM evidence_statements s JOIN evidence_principals p ON p.id=s.principal_id WHERE (s.resource_id=$2 AND $1='resource') OR (s.reservation_id=$2 AND $1='delivery')").bind(kind).bind(id).fetch_one(&mut *tx).await?;
        let disputed = statements
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["decision"] == "dispute");
        Ok(
            json!({"kind":kind,"subject":id,"state":if disputed {"disputed"} else if attested {"attested"} else {"pending"},"statements":statements,"limitation":"credential separation and matching statements do not prove physical truth or exclude collusion"}),
        )
    }
}

pub(crate) async fn enforced(tx: &mut crate::ledger::Tx<'_>) -> Result<bool, LedgerError> {
    // Older migration fixtures remain readable until the caller migrates.
    let exists: bool = sqlx::query_scalar("SELECT to_regclass('evidence_enforcement') IS NOT NULL")
        .fetch_one(&mut **tx)
        .await?;
    if !exists {
        return Ok(false);
    }
    Ok(
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM evidence_enforcement)")
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub(crate) async fn usable(tx: &mut crate::ledger::Tx<'_>, id: Uuid) -> Result<bool, LedgerError> {
    if !enforced(tx).await? {
        return Ok(true);
    }
    Ok(sqlx::query_scalar("SELECT resource_attested($1)")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?)
}

async fn active_principal(tx: &mut crate::ledger::Tx<'_>, actor: Uuid) -> Result<(), LedgerError> {
    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM evidence_principals WHERE id=$1 FOR SHARE")
            .bind(actor)
            .fetch_optional(&mut **tx)
            .await?;
    let revoked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM evidence_principal_revocations WHERE principal_id=$1)",
    )
    .bind(actor)
    .fetch_one(&mut **tx)
    .await?;
    if found.is_none() || revoked {
        return Err(LedgerError::AccessDenied);
    }
    Ok(())
}
