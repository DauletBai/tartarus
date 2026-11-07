use super::*;
use qazna::{
    domain::{AccountKind, NewAccount},
    needs::*,
    provisioning::*,
};

fn amount(value: &str) -> Amount {
    Amount::parse(value).unwrap()
}
fn resolution() -> ResolutionRequest {
    ResolutionRequest {
        basis: "Synthetic confirmation".into(),
    }
}
async fn unit(f: &Fixture, parent: Option<Uuid>) -> Unit {
    let id = Uuid::new_v4();
    f.ledger
        .create_unit(
            &format!("unit-{id}"),
            UnitRequest {
                id,
                parent,
                name: "Provisioning unit".into(),
                function: "Meet declared needs".into(),
                basis: "Synthetic hierarchy".into(),
            },
        )
        .await
        .unwrap()
}
struct Context {
    root: Uuid,
    child: Uuid,
    resource: Uuid,
    producer: Uuid,
}
async fn context(f: &Fixture, quantity: &str, measure: &str) -> Context {
    let root = unit(f, None).await;
    let child = unit(f, Some(root.id)).await;
    let producer = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: producer,
            kind: AccountKind::Producer,
        })
        .await
        .unwrap();
    f.ledger
        .link_provider(
            "provider",
            child.id,
            ProviderRequest {
                producer,
                basis: "Synthetic responsibility".into(),
            },
        )
        .await
        .unwrap();
    let resource = Uuid::new_v4();
    f.ledger
        .create_resource(
            "stock",
            ResourceRequest {
                id: resource,
                producer,
                category: Category::Food,
                unit: "meal".into(),
                quantity: amount(quantity),
                unit_measure: amount(measure),
                basis: "Synthetic stock and norm".into(),
            },
        )
        .await
        .unwrap();
    Context {
        root: root.id,
        child: child.id,
        resource,
        producer,
    }
}
fn need_request(f: &Fixture, unit: Uuid, quantity: &str) -> NeedRequest {
    NeedRequest {
        id: Uuid::new_v4(),
        resident: f.resident,
        assigned_unit: unit,
        period: "2026-11-02".into(),
        category: Category::Food,
        unit: "meal".into(),
        quantity: amount(quantity),
        basis: "Declared daily need".into(),
    }
}
fn request(resource: Uuid, quantity: &str) -> NeedReservationRequest {
    NeedReservationRequest {
        resource,
        quantity: amount(quantity),
        basis: "Provision attempt".into(),
    }
}
async fn fund(f: &Fixture, value: &str) {
    f.ledger
        .allocate(
            "fund",
            AllocationRequest {
                treasury: f.treasury,
                resident: f.resident,
                period: "2026-11-02".into(),
                category: Category::Food,
                amount: amount(value),
                basis: "Synthetic provision allocation".into(),
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn need_registration_does_not_depend_on_money_or_stock() {
    let f = Fixture::new().await;
    let root = unit(&f, None).await;
    let req = need_request(&f, root.id, "5");
    let n = f.ledger.create_need("need", req.clone()).await.unwrap();
    assert_eq!(f.balance(f.resident).await, "0");
    assert_eq!(
        (n.requested.as_str(), n.unmet.as_str(), n.uncovered.as_str()),
        ("5", "5", "5")
    );
    assert_eq!(
        f.ledger.create_need("need", req.clone()).await.unwrap().id,
        n.id
    );
    let mut duplicate = req.clone();
    duplicate.id = Uuid::new_v4();
    assert!(matches!(
        f.ledger.create_need("duplicate", duplicate).await,
        Err(LedgerError::NeedConflict)
    ));
    let mut changed = req;
    changed.basis = "Another source".into();
    assert!(matches!(
        f.ledger.create_need("need", changed).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    assert_eq!(f.transaction_count().await, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn blocked_attempt_keeps_need_and_records_both_shortages() {
    let f = Fixture::new().await;
    let c = context(&f, "2", "30").await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, c.root, "5"))
        .await
        .unwrap();
    let blocked = f
        .ledger
        .reserve_need("try", n.id, request(c.resource, "3"))
        .await
        .unwrap();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(
        blocked.reasons,
        vec!["physical_shortage", "accounting_shortage"]
    );
    assert_eq!(blocked.snapshot.resource_available, "2");
    assert_eq!(blocked.snapshot.accounting_available, "0");
    assert_eq!(f.ledger.need(n.id).await.unwrap().unmet, "5");
    assert_eq!(f.transaction_count().await, 1);
    fund(&f, "100").await;
    assert_eq!(
        f.ledger
            .reserve_need("try", n.id, request(c.resource, "3"))
            .await
            .unwrap()
            .id,
        blocked.id
    );
    let held = f
        .ledger
        .reserve_need("retry-new-state", n.id, request(c.resource, "2"))
        .await
        .unwrap();
    assert_eq!(held.status, "reserved");
    let n = f.ledger.need(n.id).await.unwrap();
    assert_eq!(
        (
            n.reserved.as_str(),
            n.received.as_str(),
            n.uncovered.as_str(),
            n.unmet.as_str()
        ),
        ("2", "0", "3", "5")
    );
    f.ledger
        .settle("deliver", held.reservation_id.unwrap(), resolution())
        .await
        .unwrap();
    let n = f.ledger.need(n.id).await.unwrap();
    assert_eq!(
        (
            n.reserved.as_str(),
            n.received.as_str(),
            n.uncovered.as_str(),
            n.unmet.as_str()
        ),
        ("0", "2", "3", "3")
    );
    let attempts = f.ledger.need_attempts(n.id).await.unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[1].id, blocked.id);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn cancellation_reopens_uncovered_need_without_claiming_delivery() {
    let f = Fixture::new().await;
    let c = context(&f, "10", "1").await;
    fund(&f, "100").await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, c.root, "5"))
        .await
        .unwrap();
    let held = f
        .ledger
        .reserve_need("r", n.id, request(c.resource, "5"))
        .await
        .unwrap();
    assert_eq!(f.ledger.need(n.id).await.unwrap().uncovered, "0");
    assert_eq!(f.ledger.need(n.id).await.unwrap().unmet, "5");
    f.ledger
        .cancel("c", held.reservation_id.unwrap(), resolution())
        .await
        .unwrap();
    let n = f.ledger.need(n.id).await.unwrap();
    assert_eq!(
        (
            n.received.as_str(),
            n.reserved.as_str(),
            n.uncovered.as_str()
        ),
        ("0", "0", "5")
    );
    assert_eq!(f.ledger.account(f.resident).await.unwrap().available, "100");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_needs_reservations_cannot_exceed_declared_quantity() {
    let f = Fixture::new().await;
    let c = context(&f, "100", "1").await;
    fund(&f, "1000").await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, c.root, "10"))
        .await
        .unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..8 {
        let ledger = f.ledger.clone();
        let req = request(c.resource, "3");
        tasks.spawn(async move {
            ledger
                .reserve_need(&format!("attempt-{i}"), n.id, req)
                .await
        });
    }
    let mut successes = 0;
    while let Some(result) = tasks.join_next().await {
        let attempt = result.unwrap().unwrap();
        if attempt.status == "reserved" {
            successes += 1;
        } else {
            assert_eq!(attempt.reasons, vec!["coverage_limit"]);
        }
    }
    assert_eq!(successes, 3);
    let n = f.ledger.need(n.id).await.unwrap();
    assert_eq!(n.reserved, "9");
    assert_eq!(n.uncovered, "1");
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "9");
    assert_eq!(f.ledger.need_attempts(n.id).await.unwrap().len(), 8);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn hierarchy_scopes_providers_and_aggregates_without_mixing_units() {
    let f = Fixture::new().await;
    let c = context(&f, "10", "1").await;
    fund(&f, "100").await;
    let sibling = unit(&f, Some(c.root)).await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, sibling.id, "2"))
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .reserve_need("outside", n.id, request(c.resource, "1"))
            .await,
        Err(LedgerError::ProviderOutOfScope)
    ));
    let mut water = need_request(&f, c.child, "1000");
    water.category = Category::Water;
    water.unit = "ml".into();
    let water = f.ledger.create_need("water", water).await.unwrap();
    assert!(matches!(
        f.ledger
            .reserve_need("wrong-unit", water.id, request(c.resource, "1"))
            .await,
        Err(LedgerError::Invalid(_))
    ));
    let report = f.ledger.unit_coverage(c.root, "2026-11-02").await.unwrap();
    assert_eq!(report.lines.len(), 2);
    assert_eq!(
        (
            report.lines[0].category.as_str(),
            report.lines[0].unit.as_str(),
            report.lines[0].requested.as_str()
        ),
        ("food", "meal", "2")
    );
    assert_eq!(
        (
            report.lines[1].category.as_str(),
            report.lines[1].unit.as_str(),
            report.lines[1].requested.as_str()
        ),
        ("water", "ml", "1000")
    );
    assert_eq!(
        f.ledger
            .unit_coverage(c.child, "2026-11-02")
            .await
            .unwrap()
            .lines
            .len(),
        1
    );
    assert!(
        f.ledger
            .unit_coverage(c.root, "2026-11-03")
            .await
            .unwrap()
            .lines
            .is_empty()
    );
    assert_eq!(
        f.ledger.unit(c.child).await.unwrap().providers,
        vec![c.producer]
    );
    assert!(matches!(
        f.ledger
            .link_provider(
                "reassign",
                sibling.id,
                ProviderRequest {
                    producer: c.producer,
                    basis: "Not versioned yet".into()
                }
            )
            .await,
        Err(LedgerError::ProviderConflict)
    ));
    assert!(
        sqlx::query("UPDATE provisioning_units SET parent_id=$1 WHERE id=$2")
            .bind(c.child)
            .bind(c.root)
            .execute(&f.pool)
            .await
            .is_err()
    );
    let id = Uuid::new_v4();
    assert!(
        f.ledger
            .create_unit(
                "cycle",
                UnitRequest {
                    id,
                    parent: Some(id),
                    name: "Cycle".into(),
                    function: "Bad".into(),
                    basis: "Bad".into()
                }
            )
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn failed_attempt_record_rolls_back_reservation_and_key() {
    let f = Fixture::new().await;
    let c = context(&f, "10", "1").await;
    fund(&f, "100").await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, c.root, "2"))
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_attempt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END; $$; CREATE TRIGGER fail_attempt BEFORE INSERT ON need_attempts FOR EACH ROW EXECUTE FUNCTION fail_attempt();").execute(&f.pool).await.unwrap();
    assert!(matches!(
        f.ledger
            .reserve_need("r", n.id, request(c.resource, "2"))
            .await,
        Err(LedgerError::Database(_))
    ));
    assert_eq!(f.ledger.need(n.id).await.unwrap().reserved, "0");
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "0");
    assert_eq!(f.ledger.resource(c.resource).await.unwrap().available, "10");
    sqlx::query("DROP TRIGGER fail_attempt ON need_attempts")
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .reserve_need("r", n.id, request(c.resource, "2"))
            .await
            .unwrap()
            .status,
        "reserved"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn raw_sql_cannot_overcover_need_or_attach_old_reservation() {
    let f = Fixture::new().await;
    let c = context(&f, "10", "1").await;
    fund(&f, "100").await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, c.root, "1"))
        .await
        .unwrap();
    let mut tx = f.pool.begin().await.unwrap();
    let rid = Uuid::new_v4();
    sqlx::query("INSERT INTO reservations(id,resident_id,resource_id,quantity,amount,basis) VALUES ($1,$2,$3,2,2,'raw test')")
        .bind(rid).bind(f.resident).bind(c.resource).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO need_attempts(id,need_id,resource_id,quantity,reservation_id,reasons,uncovered_before,resource_available,accounting_available,required_measure,basis) VALUES ($1,$2,$3,2,$4,ARRAY[]::text[],1,10,100,2,'raw test')")
        .bind(Uuid::new_v4()).bind(n.id).bind(c.resource).bind(rid).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    let old = f
        .ledger
        .reserve(
            "old",
            ReservationRequest {
                resident: f.resident,
                resource: c.resource,
                quantity: amount("1"),
                basis: "Unattributed reservation".into(),
            },
        )
        .await
        .unwrap();
    assert!(sqlx::query("INSERT INTO need_attempts(id,need_id,resource_id,quantity,reservation_id,reasons,uncovered_before,resource_available,accounting_available,required_measure,basis) VALUES ($1,$2,$3,1,$4,ARRAY[]::text[],1,9,99,1,'late attribution')")
        .bind(Uuid::new_v4()).bind(n.id).bind(c.resource).bind(old.id).execute(&f.pool).await.is_err());
    assert_eq!(f.ledger.need(n.id).await.unwrap().reserved, "0");
    assert_eq!(f.ledger.need(n.id).await.unwrap().unmet, "1");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn numeric_limit_is_feedback_and_does_not_create_a_hold() {
    let f = Fixture::new().await;
    let c = context(&f, "10", &i64::MAX.to_string()).await;
    let n = f
        .ledger
        .create_need("n", need_request(&f, c.root, "2"))
        .await
        .unwrap();
    let a = f
        .ledger
        .reserve_need("too-large", n.id, request(c.resource, "2"))
        .await
        .unwrap();
    assert_eq!(a.reasons, vec!["quantity_limit"]);
    assert_eq!(a.snapshot.required_measure, None);
    assert_eq!(f.ledger.need(n.id).await.unwrap().uncovered, "2");
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn migration_keeps_existing_reservations_unattributed_and_operational() {
    let f = Fixture::new_with_migrations(2).await;
    fund(&f, "100").await;
    let producer = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: producer,
            kind: AccountKind::Producer,
        })
        .await
        .unwrap();
    let stock = f
        .ledger
        .create_resource(
            "stock",
            ResourceRequest {
                id: Uuid::new_v4(),
                producer,
                category: Category::Food,
                unit: "meal".into(),
                quantity: amount("10"),
                unit_measure: amount("10"),
                basis: "Version 0.2 synthetic lot".into(),
            },
        )
        .await
        .unwrap();
    let r = f
        .ledger
        .reserve(
            "old-reservation",
            ReservationRequest {
                resident: f.resident,
                resource: stock.id,
                quantity: amount("4"),
                basis: "Before needs registry".into(),
            },
        )
        .await
        .unwrap();
    MIGRATOR.run(&f.pool).await.unwrap();
    let root = unit(&f, None).await;
    f.ledger
        .link_provider(
            "provider",
            root.id,
            ProviderRequest {
                producer,
                basis: "After migration".into(),
            },
        )
        .await
        .unwrap();
    let n = f
        .ledger
        .create_need("need", need_request(&f, root.id, "4"))
        .await
        .unwrap();
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "40");
    assert_eq!(n.reserved, "0");
    f.ledger
        .settle("deliver-old", r.id, resolution())
        .await
        .unwrap();
    assert_eq!(f.ledger.need(n.id).await.unwrap().received, "0");
    assert_eq!(f.balance(f.resident).await, "60");
    let next = f
        .ledger
        .reserve_need("explicit-need", n.id, request(stock.id, "4"))
        .await
        .unwrap();
    assert_eq!(next.status, "reserved");
    assert_eq!(f.ledger.need(n.id).await.unwrap().reserved, "4");
}
