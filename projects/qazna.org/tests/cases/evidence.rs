use super::*;
use qazna::evidence::{PrincipalRequest, StatementRequest};

async fn actor(f: &Fixture, account: Uuid, role: &str) -> Uuid {
    let id = Uuid::new_v4();
    f.ledger
        .register_principal(
            &id.to_string(),
            PrincipalRequest {
                id,
                account,
                role: role.into(),
                basis: "Synthetic enrollment".into(),
            },
        )
        .await
        .unwrap();
    id
}
async fn inspector(f: &Fixture) -> Uuid {
    let id = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id,
            kind: AccountKind::Resident,
        })
        .await
        .unwrap();
    actor(f, id, "inspector").await
}
fn statement(quantity: &str, decision: &str) -> StatementRequest {
    StatementRequest {
        id: Uuid::new_v4(),
        quantity: amount(quantity),
        decision: decision.into(),
        evidence_sha256: "a".repeat(64),
        basis: "Synthetic evidence document digest, not physical truth".into(),
    }
}
async fn attest(f: &Fixture, who: Uuid, kind: &str, id: Uuid, quantity: &str, decision: &str) {
    f.ledger
        .record_evidence(
            &Uuid::new_v4().to_string(),
            who,
            kind,
            id,
            statement(quantity, decision),
        )
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn strict_lot_and_delivery_need_independent_credentials() {
    let f = Fixture::new().await;
    f.ledger
        .enable_evidence("Synthetic strict pilot")
        .await
        .unwrap();
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let stock = resource(&f, "10", "2").await;
    assert_eq!(stock.available, "0");
    assert!(matches!(
        f.ledger
            .reserve("reserve", reservation(&f, &stock, "2"))
            .await,
        Err(LedgerError::InsufficientStock)
    ));
    let a = inspector(&f).await;
    let b = inspector(&f).await;
    attest(&f, a, "resource", stock.id, "10", "confirm").await;
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().available, "0");
    assert!(matches!(
        f.ledger
            .record_evidence(
                "duplicate",
                a,
                "resource",
                stock.id,
                statement("10", "confirm")
            )
            .await,
        Err(LedgerError::EvidenceConflict)
    ));
    attest(&f, b, "resource", stock.id, "10", "confirm").await;
    assert_eq!(
        f.ledger.evidence("resource", stock.id).await.unwrap()["state"],
        "attested"
    );
    let held = f
        .ledger
        .reserve("reserve", reservation(&f, &stock, "2"))
        .await
        .unwrap();
    assert!(matches!(
        f.ledger.settle("settle", held.id, resolution()).await,
        Err(LedgerError::EvidenceRequired)
    ));
    let sender = actor(&f, stock.producer, "custodian").await;
    let receiver = actor(&f, f.resident, "recipient").await;
    attest(&f, sender, "delivery", held.id, "2", "confirm").await;
    assert!(matches!(
        f.ledger.settle("settle", held.id, resolution()).await,
        Err(LedgerError::EvidenceRequired)
    ));
    attest(&f, receiver, "delivery", held.id, "2", "confirm").await;
    f.ledger
        .settle("settle", held.id, resolution())
        .await
        .unwrap();
    assert_eq!(f.balance(f.resident).await, "96");
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().on_hand, "8");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn self_attestation_wrong_quantity_disputes_and_raw_sql_are_rejected() {
    let f = Fixture::new().await;
    f.ledger.enable_evidence("Strict test").await.unwrap();
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let stock = resource(&f, "10", "2").await;
    let self_actor = actor(&f, stock.producer, "inspector").await;
    assert!(matches!(
        f.ledger
            .record_evidence(
                "self",
                self_actor,
                "resource",
                stock.id,
                statement("10", "confirm")
            )
            .await,
        Err(LedgerError::Invalid(_))
    ));
    let a = inspector(&f).await;
    let b = inspector(&f).await;
    assert!(matches!(
        f.ledger
            .record_evidence("wrong", a, "resource", stock.id, statement("9", "confirm"))
            .await,
        Err(LedgerError::Invalid(_))
    ));
    // Direct inserts cannot bypass the physical gate or role checks.
    assert!(sqlx::query("INSERT INTO reservations(id,resident_id,resource_id,quantity,amount,basis) VALUES($1,$2,$3,1,2,'bypass')").bind(Uuid::new_v4()).bind(f.resident).bind(stock.id).execute(&f.pool).await.is_err());
    attest(&f, a, "resource", stock.id, "10", "confirm").await;
    attest(&f, b, "resource", stock.id, "10", "confirm").await;
    let hold = f
        .ledger
        .reserve("hold", reservation(&f, &stock, "1"))
        .await
        .unwrap();
    attest(&f, a, "resource", stock.id, "10", "dispute").await;
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().available, "0");
    assert_eq!(
        f.ledger.evidence("resource", stock.id).await.unwrap()["state"],
        "disputed"
    );
    assert!(matches!(
        f.ledger.settle("s", hold.id, resolution()).await,
        Err(LedgerError::EvidenceRequired)
    ));
    f.ledger
        .cancel("cancel", hold.id, resolution())
        .await
        .unwrap();
    assert!(
        sqlx::query("DELETE FROM evidence_statements")
            .execute(&f.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM evidence_enforcement")
            .execute(&f.pool)
            .await
            .is_err()
    );
    let status = f.ledger.system_status().await.unwrap();
    assert!(
        status["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "physical_evidence_disputed")
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn evidence_activation_requires_resolved_legacy_holds() {
    let f = Fixture::new().await;
    let stock = resource(&f, "10", "2").await;
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let held = f
        .ledger
        .reserve("hold", reservation(&f, &stock, "2"))
        .await
        .unwrap();
    assert!(
        f.ledger
            .enable_evidence("Cannot silently change open promises")
            .await
            .is_err()
    );
    f.ledger
        .cancel("cancel", held.id, resolution())
        .await
        .unwrap();
    f.ledger
        .enable_evidence("Explicit migration to strict evidence")
        .await
        .unwrap();
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().available, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn attestor_http_identity_cannot_be_forged_or_used_for_economic_actions() {
    let f = Fixture::new().await;
    let stock = resource(&f, "10", "2").await;
    let a = inspector(&f).await;
    let op = "operator-credential-over-thirty-two-bytes";
    let token = "attestor-credential-over-thirty-two-bytes";
    let app = api::router_with_access(
        f.ledger.clone(),
        api::AccessTokens::new(op.into(), None, None)
            .unwrap()
            .with_attestors(vec![api::AttestorCredential {
                principal: a,
                token: token.into(),
            }])
            .unwrap(),
    );
    let path = format!("/v1/evidence/resource/{}", stock.id);
    for (auth, route, body, status) in [
        (
            op,
            path.as_str(),
            serde_json::json!(statement("10", "confirm")),
            403,
        ),
        (token, "/v1/transfers", serde_json::json!({}), 403),
        (
            token,
            path.as_str(),
            serde_json::json!(statement("10", "confirm")),
            200,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(route)
                    .header("authorization", format!("Bearer {auth}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
    assert_eq!(
        f.ledger.evidence("resource", stock.id).await.unwrap()["statements"][0]["principal"],
        a.to_string()
    );
}
