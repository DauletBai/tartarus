//! Real PostgreSQL tests. Run via scripts/check.py; never silently skip a missing DB.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use qazna::{
    api,
    domain::{Amount, Receipt, Transfer},
    ledger::{Ledger, LedgerError, MIGRATOR},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

#[path = "cases/needs.rs"]
mod needs;
#[path = "cases/provisioning.rs"]
mod provisioning;

struct Fixture {
    pool: PgPool,
    ledger: Ledger,
    treasury: Uuid,
    resident: Uuid,
    issuer: Uuid,
    genesis: Uuid,
}

impl Fixture {
    async fn new() -> Self {
        Self::new_at(false).await
    }

    async fn new_at(legacy: bool) -> Self {
        Self::new_with_migrations(if legacy { 1 } else { 3 }).await
    }

    async fn new_with_migrations(count: usize) -> Self {
        let url = std::env::var("QAZNA_TEST_DATABASE_URL").expect("use python3 scripts/check.py");
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE SCHEMA {schema}"))
            .execute(&admin)
            .await
            .unwrap();
        admin.close().await;
        let pool = PgPoolOptions::new()
            .max_connections(12)
            .after_connect(move |connection, _| {
                let query = format!("SET search_path TO {schema}");
                Box::pin(async move {
                    sqlx::query(&query).execute(connection).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        if count < 3 {
            let path = std::env::temp_dir().join(format!("qazna-migrations-{}", Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            for (name, contents) in [
                (
                    "0001_ledger.sql",
                    include_str!("../migrations/0001_ledger.sql"),
                ),
                (
                    "0002_provisioning.sql",
                    include_str!("../migrations/0002_provisioning.sql"),
                ),
            ]
            .into_iter()
            .take(count)
            {
                std::fs::write(path.join(name), contents).unwrap();
            }
            let migrator = sqlx::migrate::Migrator::new(path.clone()).await.unwrap();
            migrator.run(&pool).await.unwrap();
            std::fs::remove_dir_all(path).unwrap();
        } else {
            MIGRATOR.run(&pool).await.unwrap();
        }
        let treasury = Uuid::new_v4();
        let resident = Uuid::new_v4();
        let issuer = Uuid::new_v4();
        let genesis = Uuid::new_v4();
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("INSERT INTO accounts(id, kind) VALUES ($1, 'issuer'), ($2, 'treasury'), ($3, 'resident')")
            .bind(issuer).bind(treasury).bind(resident).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO journal_transactions(id, idempotency_key, kind, request) VALUES ($1, 'fixture-genesis', 'genesis', '{\"fixture\":true}')")
            .bind(genesis).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO postings(transaction_id, account_id, delta) VALUES ($1, $2, -1000), ($1, $3, 1000)")
            .bind(genesis).bind(issuer).bind(treasury).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        Self {
            ledger: Ledger::new(pool.clone()),
            pool,
            treasury,
            resident,
            issuer,
            genesis,
        }
    }

    fn transfer(&self, amount: &str) -> Transfer {
        Transfer {
            from: self.treasury,
            to: self.resident,
            amount: Amount::parse(amount).unwrap(),
        }
    }

    async fn balance(&self, account: Uuid) -> String {
        self.ledger.account(account).await.unwrap().balance
    }

    async fn transaction_count(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM journal_transactions")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn transfer_replay_conflict_and_persistence() {
    let f = Fixture::new().await;
    let first = f
        .ledger
        .transfer("payment-1", f.transfer("100"))
        .await
        .unwrap();
    let second = f
        .ledger
        .transfer("payment-1", f.transfer("100"))
        .await
        .unwrap();
    assert_eq!(first, second);
    assert!(matches!(
        f.ledger.transfer("payment-1", f.transfer("101")).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    assert_eq!(f.balance(f.treasury).await, "900");
    assert_eq!(f.balance(f.resident).await, "100");
    assert_eq!(f.transaction_count().await, 2);
    // A fresh service has no in-process ledger state to recover.
    let reloaded = Ledger::new(f.pool.clone());
    assert_eq!(
        reloaded
            .transfer("payment-1", f.transfer("100"))
            .await
            .unwrap(),
        first
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn failed_operations_roll_back_and_do_not_consume_key() {
    let f = Fixture::new().await;
    assert!(matches!(
        f.ledger.transfer("retry", f.transfer("1001")).await,
        Err(LedgerError::InsufficientFunds)
    ));
    assert_eq!(f.transaction_count().await, 1);
    f.ledger.transfer("retry", f.transfer("100")).await.unwrap();
    let mut missing = f.transfer("1");
    missing.to = Uuid::new_v4();
    assert!(matches!(
        f.ledger.transfer("missing", missing).await,
        Err(LedgerError::NotFound)
    ));
    let mut issuance = f.transfer("1");
    issuance.from = f.issuer;
    assert!(matches!(
        f.ledger.transfer("mint", issuance).await,
        Err(LedgerError::Invalid(_))
    ));
    assert_eq!(f.transaction_count().await, 2);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_spending_cannot_overdraw() {
    let f = Fixture::new().await;
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..10 {
        let ledger = f.ledger.clone();
        let request = f.transfer("300");
        tasks.spawn(async move { ledger.transfer(&format!("concurrent-{i}"), request).await });
    }
    let mut successes = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(_) => successes += 1,
            Err(LedgerError::InsufficientFunds) => {}
            Err(other) => panic!("unexpected result: {other}"),
        }
    }
    assert_eq!(successes, 3);
    assert_eq!(f.balance(f.treasury).await, "100");
    assert_eq!(f.balance(f.resident).await, "900");
    assert_eq!(f.transaction_count().await, 4);
    let sum: String = sqlx::query_scalar("SELECT sum(delta)::text FROM postings")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(sum, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_identical_requests_post_once() {
    let f = Fixture::new().await;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let ledger = f.ledger.clone();
        let request = f.transfer("100");
        tasks.spawn(async move { ledger.transfer("same-key", request).await });
    }
    let mut ids = std::collections::HashSet::new();
    while let Some(result) = tasks.join_next().await {
        ids.insert(result.unwrap().unwrap().transaction_id);
    }
    assert_eq!(ids.len(), 1);
    assert_eq!(f.balance(f.resident).await, "100");
    assert_eq!(f.transaction_count().await, 2);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn database_rejects_unbalanced_and_mutated_history() {
    let f = Fixture::new().await;
    let mut tx = f.pool.begin().await.unwrap();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO journal_transactions(id, idempotency_key, kind, request) VALUES ($1, 'bad', 'transfer', '{}')")
        .bind(id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO postings(transaction_id, account_id, delta) VALUES ($1, $2, 1)")
        .bind(id)
        .bind(f.resident)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(tx.commit().await.is_err());
    assert_eq!(f.transaction_count().await, 1);
    for query in [
        "DELETE FROM postings",
        "UPDATE postings SET delta = 1",
        "TRUNCATE postings",
        "UPDATE accounts SET kind = 'issuer'",
        "DELETE FROM journal_transactions",
    ] {
        assert!(
            sqlx::query(query).execute(&f.pool).await.is_err(),
            "{query}"
        );
    }
    // A new posting cannot retroactively change a committed transaction.
    assert!(
        sqlx::query("INSERT INTO postings(transaction_id, account_id, delta) VALUES ($1, $2, 1)")
            .bind(f.genesis)
            .bind(f.resident)
            .execute(&f.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn retained_money_exposes_treasury_liquidity_gap() {
    let f = Fixture::new().await;
    f.ledger
        .transfer("day-1", f.transfer("1000"))
        .await
        .unwrap();
    let return_flow = Transfer {
        from: f.resident,
        to: f.treasury,
        amount: Amount::parse("900").unwrap(),
    };
    f.ledger.transfer("return", return_flow).await.unwrap();
    assert!(matches!(
        f.ledger.transfer("day-2", f.transfer("1000")).await,
        Err(LedgerError::InsufficientFunds)
    ));
    assert_eq!(f.balance(f.treasury).await, "900");
    assert_eq!(f.balance(f.resident).await, "100");
    assert_eq!(f.balance(f.issuer).await, "-1000");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn http_authorization_validation_and_transfer() {
    let f = Fixture::new().await;
    let token = "local-test-operator-token-32-characters";
    let app = api::router(f.ledger.clone(), token.into());
    let path = format!("/v1/accounts/{}", f.treasury);
    for supplied in [None, Some("Bearer wrong")] {
        let mut request = Request::builder().uri(&path);
        if let Some(value) = supplied {
            request = request.header("authorization", value);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let payload = serde_json::to_string(&f.transfer("100")).unwrap();
    let request = |body: String, key: Option<&str>| {
        let mut r = Request::builder()
            .method("POST")
            .uri("/v1/transfers")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"));
        if let Some(key) = key {
            r = r.header("idempotency-key", key);
        }
        r.body(Body::from(body)).unwrap()
    };
    let response = app
        .clone()
        .oneshot(request(payload.clone(), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let mut bad = serde_json::to_value(f.transfer("100")).unwrap();
    bad["amount"] = serde_json::json!(100);
    let response = app
        .clone()
        .oneshot(request(bad.to_string(), Some("number")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app.oneshot(request(payload, Some("http-1"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let receipt: Receipt = serde_json::from_slice(&body).unwrap();
    assert_eq!(receipt.transfer.amount.units(), 100);
    assert_eq!(f.balance(f.resident).await, "100");
}
