use super::*;
use qazna::{
    domain::{AccountKind, NewAccount},
    provisioning::*,
};

fn amount(value: &str) -> Amount {
    Amount::parse(value).unwrap()
}
fn allocation(f: &Fixture, value: &str) -> AllocationRequest {
    AllocationRequest {
        treasury: f.treasury,
        resident: f.resident,
        period: "2026-11-02".into(),
        category: Category::Food,
        amount: amount(value),
        basis: "Synthetic day 1 entitlement".into(),
    }
}
async fn resource(f: &Fixture, quantity: &str, price: &str) -> Resource {
    let producer = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: producer,
            kind: AccountKind::Producer,
        })
        .await
        .unwrap();
    f.ledger
        .create_resource(
            &format!("stock-{producer}"),
            ResourceRequest {
                id: Uuid::new_v4(),
                producer,
                category: Category::Food,
                unit: "meal".into(),
                quantity: amount(quantity),
                unit_measure: amount(price),
                basis: "Synthetic stock declaration".into(),
            },
        )
        .await
        .unwrap()
}
fn reservation(f: &Fixture, stock: &Resource, quantity: &str) -> ReservationRequest {
    ReservationRequest {
        resident: f.resident,
        resource: stock.id,
        quantity: amount(quantity),
        basis: "Resident request".into(),
    }
}
fn resolution() -> ResolutionRequest {
    ResolutionRequest {
        basis: "Operator confirmation".into(),
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn allocation_reservation_settlement_complete_cycle() {
    let f = Fixture::new().await;
    let allocation = f
        .ledger
        .allocate("allocation", allocation(&f, "300"))
        .await
        .unwrap();
    assert_eq!(f.balance(f.treasury).await, "700");
    assert_eq!(f.balance(f.resident).await, "300");
    let stock = resource(&f, "10", "20").await;
    let held = f
        .ledger
        .reserve("reserve", reservation(&f, &stock, "4"))
        .await
        .unwrap();
    let account = f.ledger.account(f.resident).await.unwrap();
    assert_eq!(
        (
            account.balance.as_str(),
            account.reserved.as_str(),
            account.available.as_str()
        ),
        ("300", "80", "220")
    );
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().available, "6");
    let done = f
        .ledger
        .settle("settle", held.id, resolution())
        .await
        .unwrap();
    assert_eq!(done.state, "settled");
    assert!(done.transaction_id.is_some());
    assert_ne!(done.transaction_id.unwrap(), allocation.transaction_id);
    assert_eq!(f.balance(stock.producer).await, "80");
    let account = f.ledger.account(f.resident).await.unwrap();
    assert_eq!(
        (
            account.balance.as_str(),
            account.reserved.as_str(),
            account.available.as_str()
        ),
        ("220", "0", "220")
    );
    let stock = f.ledger.resource(stock.id).await.unwrap();
    assert_eq!(
        (
            stock.on_hand.as_str(),
            stock.reserved.as_str(),
            stock.available.as_str()
        ),
        ("6", "0", "6")
    );
    let replay = f
        .ledger
        .settle("settle", held.id, resolution())
        .await
        .unwrap();
    assert_eq!(replay.transaction_id, done.transaction_id);
    assert!(matches!(
        f.ledger.cancel("late-cancel", held.id, resolution()).await,
        Err(LedgerError::ReservationClosed)
    ));
    assert_eq!(f.transaction_count().await, 3);
    // Allocation and settlement leave total issuance unchanged.
    assert_eq!(f.balance(f.issuer).await, "-1000");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn cancellation_releases_both_holds_without_payment() {
    let f = Fixture::new().await;
    f.ledger.allocate("a", allocation(&f, "100")).await.unwrap();
    let stock = resource(&f, "2", "50").await;
    let held = f
        .ledger
        .reserve("r", reservation(&f, &stock, "2"))
        .await
        .unwrap();
    let spend = Transfer {
        from: f.resident,
        to: stock.producer,
        amount: amount("1"),
    };
    assert!(matches!(
        f.ledger.transfer("spend", spend.clone()).await,
        Err(LedgerError::InsufficientFunds)
    ));
    f.ledger.cancel("c", held.id, resolution()).await.unwrap();
    assert_eq!(f.ledger.account(f.resident).await.unwrap().available, "100");
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().available, "2");
    assert_eq!(f.balance(stock.producer).await, "0");
    assert!(matches!(
        f.ledger.settle("late", held.id, resolution()).await,
        Err(LedgerError::ReservationClosed)
    ));
    f.ledger.transfer("spend", spend).await.unwrap();
    assert_eq!(f.balance(f.resident).await, "99");
    // A replay returns the original receipt; GET gives current state.
    assert_eq!(
        f.ledger
            .reserve("r", reservation(&f, &stock, "2"))
            .await
            .unwrap()
            .state,
        "reserved"
    );
    assert_eq!(
        f.ledger.reservation(held.id).await.unwrap().state,
        "cancelled"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn allocation_unique_under_different_keys_and_concurrent_requests() {
    let f = Fixture::new().await;
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..8 {
        let ledger = f.ledger.clone();
        let req = allocation(&f, "100");
        tasks.spawn(async move { ledger.allocate(&format!("a-{i}"), req).await });
    }
    let mut count = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(_) => count += 1,
            Err(LedgerError::AllocationConflict) => {}
            Err(e) => panic!("{e}"),
        }
    }
    assert_eq!(count, 1);
    assert_eq!(f.balance(f.resident).await, "100");
    let mut next_day = allocation(&f, "100");
    next_day.period = "2026-11-03".into();
    f.ledger.allocate("next", next_day.clone()).await.unwrap();
    f.ledger.allocate("next", next_day).await.unwrap();
    assert_eq!(f.balance(f.resident).await, "200");
    let mut invalid = allocation(&f, "100");
    invalid.period = "2026-02-29".into();
    assert!(matches!(
        f.ledger.allocate("bad", invalid).await,
        Err(LedgerError::Invalid(_))
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_reservations_cannot_overcommit_money_or_stock() {
    for (quantity, funds, expected) in [("100", "100", 3), ("2", "1000", 2)] {
        let f = Fixture::new().await;
        f.ledger.allocate("a", allocation(&f, funds)).await.unwrap();
        let stock = resource(&f, quantity, "30").await;
        let mut tasks = tokio::task::JoinSet::new();
        for i in 0..10 {
            let ledger = f.ledger.clone();
            let req = reservation(&f, &stock, "1");
            tasks.spawn(async move { ledger.reserve(&format!("r-{i}"), req).await });
        }
        let mut count = 0;
        while let Some(result) = tasks.join_next().await {
            match result.unwrap() {
                Ok(_) => count += 1,
                Err(LedgerError::InsufficientFunds | LedgerError::InsufficientStock) => {}
                Err(e) => panic!("{e}"),
            }
        }
        assert_eq!(count, expected);
        assert_eq!(
            f.ledger.account(f.resident).await.unwrap().reserved,
            (expected * 30).to_string()
        );
        assert_eq!(
            f.ledger.resource(stock.id).await.unwrap().reserved,
            expected.to_string()
        );
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn simultaneous_settlement_and_cancellation_have_one_winner() {
    let f = Fixture::new().await;
    f.ledger.allocate("a", allocation(&f, "100")).await.unwrap();
    let stock = resource(&f, "1", "100").await;
    let held = f
        .ledger
        .reserve("r", reservation(&f, &stock, "1"))
        .await
        .unwrap();
    let (settled, cancelled) = tokio::join!(
        f.ledger.settle("s", held.id, resolution()),
        f.ledger.cancel("c", held.id, resolution())
    );
    assert_ne!(settled.is_ok(), cancelled.is_ok());
    let loser = if settled.is_ok() { cancelled } else { settled };
    assert!(matches!(loser, Err(LedgerError::ReservationClosed)));
    let terminal = f.ledger.reservation(held.id).await.unwrap();
    let delivered = terminal.state == "settled";
    assert_eq!(
        f.balance(stock.producer).await,
        if delivered { "100" } else { "0" }
    );
    assert_eq!(
        f.ledger.resource(stock.id).await.unwrap().on_hand,
        if delivered { "0" } else { "1" }
    );
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn settlement_failure_rolls_back_stock_payment_and_outcome() {
    let f = Fixture::new().await;
    f.ledger.allocate("a", allocation(&f, "100")).await.unwrap();
    let stock = resource(&f, "1", "100").await;
    let held = f
        .ledger
        .reserve("r", reservation(&f, &stock, "1"))
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_settlement() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END; $$; CREATE TRIGGER fail_settlement BEFORE INSERT ON journal_transactions FOR EACH ROW WHEN (NEW.kind = 'settlement') EXECUTE FUNCTION reject_settlement();")
        .execute(&f.pool).await.unwrap();
    assert!(matches!(
        f.ledger.settle("s", held.id, resolution()).await,
        Err(LedgerError::Database(_))
    ));
    assert_eq!(
        f.ledger.reservation(held.id).await.unwrap().state,
        "reserved"
    );
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "100");
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().on_hand, "1");
    assert_eq!(f.balance(stock.producer).await, "0");
    sqlx::query("DROP TRIGGER fail_settlement ON journal_transactions")
        .execute(&f.pool)
        .await
        .unwrap();
    f.ledger.settle("s", held.id, resolution()).await.unwrap();
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().on_hand, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn direct_sql_cannot_spend_reserved_money_or_overbook_resource() {
    let f = Fixture::new().await;
    f.ledger.allocate("a", allocation(&f, "100")).await.unwrap();
    let stock = resource(&f, "1", "80").await;
    f.ledger
        .reserve("r", reservation(&f, &stock, "1"))
        .await
        .unwrap();
    let mut tx = f.pool.begin().await.unwrap();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO journal_transactions(id,idempotency_key,kind,request) VALUES ($1,'bypass','transfer','{}')").bind(id).execute(&mut *tx).await.unwrap();
    sqlx::query(
        "INSERT INTO postings(transaction_id,account_id,delta) VALUES ($1,$2,-50),($1,$3,50)",
    )
    .bind(id)
    .bind(f.resident)
    .bind(stock.producer)
    .execute(&mut *tx)
    .await
    .unwrap();
    assert!(tx.commit().await.is_err());
    let result = sqlx::query("INSERT INTO reservations(id,resident_id,resource_id,quantity,amount,basis) VALUES ($1,$2,$3,1,80,'bypass')")
        .bind(Uuid::new_v4()).bind(f.resident).bind(stock.id).execute(&f.pool).await;
    assert!(result.is_err());
    assert_eq!(f.balance(f.resident).await, "100");
    assert_eq!(f.ledger.resource(stock.id).await.unwrap().reserved, "1");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn failed_allocation_and_cross_route_key_conflicts_are_atomic() {
    let f = Fixture::new().await;
    assert!(matches!(
        f.ledger.allocate("a", allocation(&f, "1001")).await,
        Err(LedgerError::InsufficientFunds)
    ));
    let a = f.ledger.allocate("a", allocation(&f, "100")).await.unwrap();
    assert_eq!(
        f.ledger
            .allocate("a", allocation(&f, "100"))
            .await
            .unwrap()
            .id,
        a.id
    );
    assert!(matches!(
        f.ledger.transfer("a", f.transfer("1")).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    let stock = resource(&f, "1", "100").await;
    let mut request = reservation(&f, &stock, "1");
    let held = f.ledger.reserve("r", request.clone()).await.unwrap();
    request.basis = "different intent".into();
    assert!(matches!(
        f.ledger.reserve("r", request).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    assert!(matches!(
        f.ledger.settle("r", held.id, resolution()).await,
        Err(LedgerError::IdempotencyConflict)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn migration_preserves_legacy_balances_and_idempotency() {
    let f = Fixture::new_at(true).await;
    let request = f.transfer("100");
    let payload = serde_json::json!({"version":1,"transfer":request});
    let id = Uuid::new_v4();
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO journal_transactions(id,idempotency_key,kind,request) VALUES ($1,'legacy','transfer',$2)")
        .bind(id).bind(payload).execute(&mut *tx).await.unwrap();
    sqlx::query(
        "INSERT INTO postings(transaction_id,account_id,delta) VALUES ($1,$2,-100),($1,$3,100)",
    )
    .bind(id)
    .bind(f.treasury)
    .bind(f.resident)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    MIGRATOR.run(&f.pool).await.unwrap();
    assert_eq!(
        f.ledger
            .transfer("legacy", request)
            .await
            .unwrap()
            .transaction_id,
        id
    );
    assert_eq!(f.balance(f.treasury).await, "900");
    assert_eq!(f.balance(f.resident).await, "100");
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "0");
    assert!(matches!(
        f.ledger.allocate("legacy", allocation(&f, "100")).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    let stock = resource(&f, "10", "20").await;
    f.ledger
        .reserve("r", reservation(&f, &stock, "4"))
        .await
        .unwrap();
    assert_eq!(f.ledger.account(f.resident).await.unwrap().available, "20");
    assert_eq!(f.transaction_count().await, 2);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn simultaneous_reserve_and_transfer_share_available_balance() {
    let f = Fixture::new().await;
    f.ledger.allocate("a", allocation(&f, "100")).await.unwrap();
    let stock = resource(&f, "10", "70").await;
    let spend = Transfer {
        from: f.resident,
        to: stock.producer,
        amount: amount("70"),
    };
    let (hold, payment) = tokio::join!(
        f.ledger.reserve("r", reservation(&f, &stock, "1")),
        f.ledger.transfer("t", spend)
    );
    assert_ne!(hold.is_ok(), payment.is_ok());
    if hold.is_ok() {
        assert!(matches!(payment, Err(LedgerError::InsufficientFunds)));
    } else {
        assert!(matches!(hold, Err(LedgerError::InsufficientFunds)));
    }
    assert_eq!(f.ledger.account(f.resident).await.unwrap().available, "30");
}
