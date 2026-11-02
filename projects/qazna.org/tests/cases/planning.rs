use super::*;
use qazna::{needs::NeedRequest, planning::*};
fn norm(quantity: &str) -> NeedNorm {
    NeedNorm {
        id: Uuid::new_v4(),
        code: "experimental-meals".into(),
        revision: 1,
        supersedes: None,
        category: Category::Food,
        unit: "meal".into(),
        quantity: amount(quantity),
        applicability: "Explicit operator selection for a synthetic resident/day".into(),
        basis: "Experimental assumption, not a physiological standard".into(),
    }
}
async fn need(
    f: &Fixture,
    resident: Uuid,
    unit: Uuid,
    category: Category,
    physical_unit: &str,
    quantity: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    f.ledger
        .create_need(
            &format!("need-{id}"),
            NeedRequest {
                id,
                resident,
                assigned_unit: unit,
                period: "2026-11-02".into(),
                category,
                unit: physical_unit.into(),
                quantity: amount(quantity),
                basis: "Synthetic need".into(),
            },
        )
        .await
        .unwrap();
    id
}
fn option(c: &Context) -> ProcessOption {
    ProcessOption {
        recipe: c.recipe.id,
        capacity: c.capacity,
        inputs: plan(c, "1").inputs,
    }
}
fn proposal(needs: Vec<NeedOptions>) -> PlanningRequest {
    PlanningRequest {
        period: "2026-11-02".into(),
        needs,
        basis: "Explicit operator priority and candidate order".into(),
    }
}
async fn meal_stock(f: &Fixture, c: &Context, quantity: &str, measure: &str) -> Uuid {
    let producer = executor(f, c.unit).await;
    let id = Uuid::new_v4();
    f.ledger
        .create_resource(
            &format!("meals-{id}"),
            ResourceRequest {
                id,
                producer,
                category: Category::Food,
                unit: "meal".into(),
                quantity: amount(quantity),
                unit_measure: amount(measure),
                basis: "Declared test meals".into(),
            },
        )
        .await
        .unwrap();
    id
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn norm_revisions_preserve_existing_needs_and_do_not_require_funds() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let first = norm("3");
    f.ledger
        .create_need_norm("norm", first.clone())
        .await
        .unwrap();
    let req = NormNeedRequest {
        id: Uuid::new_v4(),
        resident: f.resident,
        assigned_unit: c.unit,
        period: "2026-11-02".into(),
        norm: first.id,
        basis: "Resident-specific application".into(),
    };
    let a = f
        .ledger
        .create_need_from_norm("apply", req.clone())
        .await
        .unwrap();
    assert_eq!(a.need.requested, "3");
    assert_eq!(f.balance(f.resident).await, "0");
    assert_eq!(
        f.ledger
            .create_need_from_norm("apply", req)
            .await
            .unwrap()
            .need
            .id,
        a.need.id
    );
    let mut second = first.clone();
    second.id = Uuid::new_v4();
    second.revision = 2;
    second.supersedes = Some(first.id);
    second.quantity = amount("5");
    f.ledger
        .create_need_norm("revise", second.clone())
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .need_norm_source(a.need.id)
            .await
            .unwrap()
            .unwrap()
            .id,
        first.id
    );
    assert_eq!(f.ledger.need(a.need.id).await.unwrap().requested, "3");
    let b = f
        .ledger
        .create_need_from_norm(
            "next-day",
            NormNeedRequest {
                id: Uuid::new_v4(),
                resident: f.resident,
                assigned_unit: c.unit,
                period: "2026-11-03".into(),
                norm: second.id,
                basis: "Next-day selection".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(b.need.requested, "5");
    let mut changed = first.clone();
    changed.quantity = amount("4");
    assert!(matches!(
        f.ledger.create_need_norm("norm", changed).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    second.id = Uuid::new_v4();
    second.revision = 3;
    second.unit = "gram".into();
    assert!(matches!(
        f.ledger.create_need_norm("bad-dimension", second).await,
        Err(LedgerError::Invalid(_))
    ));
    assert_eq!(f.transaction_count().await, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn competing_norm_revisions_have_one_successor() {
    let f = Fixture::new().await;
    let first = norm("3");
    f.ledger
        .create_need_norm("first", first.clone())
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for i in 0..6 {
        let l = f.ledger.clone();
        let mut r = first.clone();
        r.id = Uuid::new_v4();
        r.revision = 2;
        r.supersedes = Some(first.id);
        r.basis = format!("Candidate {i}");
        tasks.push(tokio::spawn(async move {
            l.create_need_norm(&format!("revision-{i}"), r).await
        }));
    }
    let mut successes = 0;
    for t in tasks {
        match t.await.unwrap() {
            Ok(_) => successes += 1,
            Err(e) => assert!(matches!(e, LedgerError::NormConflict)),
        }
    }
    assert_eq!(successes, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn norm_application_failure_rolls_back_need_and_sql_rejects_relabeling() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let spec = norm("3");
    f.ledger
        .create_need_norm("norm", spec.clone())
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_norm_link() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected source failure'; END; $$; CREATE TRIGGER fail_norm_link BEFORE INSERT ON need_norm_sources FOR EACH ROW EXECUTE FUNCTION fail_norm_link();").execute(&f.pool).await.unwrap();
    let req = NormNeedRequest {
        id: Uuid::new_v4(),
        resident: f.resident,
        assigned_unit: c.unit,
        period: "2026-11-02".into(),
        norm: spec.id,
        basis: "Apply norm".into(),
    };
    assert!(
        f.ledger
            .create_need_from_norm("retry", req.clone())
            .await
            .is_err()
    );
    assert!(matches!(
        f.ledger.need(req.id).await,
        Err(LedgerError::NeedNotFound)
    ));
    sqlx::query("DROP TRIGGER fail_norm_link ON need_norm_sources")
        .execute(&f.pool)
        .await
        .unwrap();
    f.ledger.create_need_from_norm("retry", req).await.unwrap();
    let other = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: other,
            kind: AccountKind::Resident,
        })
        .await
        .unwrap();
    let manual = need(&f, other, c.unit, Category::Food, "meal", "3").await;
    assert!(f.ledger.need_norm_source(manual).await.unwrap().is_none());
    assert!(
        sqlx::query("INSERT INTO need_norm_sources(need_id,norm_id) VALUES ($1,$2)")
            .bind(manual)
            .bind(spec.id)
            .execute(&f.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE need_norms SET quantity=10 WHERE id=$1")
            .bind(spec.id)
            .execute(&f.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn proposal_separates_physical_stock_from_accounting_shortage_without_writes() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let stock = meal_stock(&f, &c, "5", "20").await;
    let id = need(&f, f.resident, c.unit, Category::Food, "meal", "5").await;
    let req = proposal(vec![NeedOptions {
        need: id,
        resources: vec![stock],
        production: vec![],
    }]);
    let a = f.ledger.propose_provisioning(req.clone()).await.unwrap();
    assert!(a.advisory);
    assert_eq!(a.needs[0].stock_selected, "5");
    assert_eq!(a.needs[0].residual, "0");
    assert_eq!(a.accounting[0].shortage, "100");
    let b = f.ledger.propose_provisioning(req).await.unwrap();
    assert_eq!(
        serde_json::to_value(&a.needs).unwrap(),
        serde_json::to_value(&b.needs).unwrap()
    );
    assert_eq!(f.ledger.resource(stock).await.unwrap().reserved, "0");
    assert_eq!(f.ledger.need(id).await.unwrap().uncovered, "5");
    assert_eq!(f.transaction_count().await, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn proposal_shares_capacity_and_inputs_across_needs() {
    let f = Fixture::new().await;
    let c = setup(&f, "4", "10", "20").await;
    let stock = meal_stock(&f, &c, "2", "10").await;
    let first = need(&f, f.resident, c.unit, Category::Food, "meal", "5").await;
    let other = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: other,
            kind: AccountKind::Resident,
        })
        .await
        .unwrap();
    let second = need(&f, other, c.unit, Category::Food, "meal", "5").await;
    let req = proposal(vec![
        NeedOptions {
            need: first,
            resources: vec![stock],
            production: vec![option(&c)],
        },
        NeedOptions {
            need: second,
            resources: vec![stock],
            production: vec![option(&c)],
        },
    ]);
    let result = f.ledger.propose_provisioning(req).await.unwrap();
    assert_eq!(result.needs[0].stock_selected, "2");
    assert_eq!(result.needs[0].expected_from_production, "3");
    assert_eq!(result.needs[0].residual, "0");
    assert_eq!(result.needs[1].stock_selected, "0");
    assert_eq!(result.needs[1].expected_from_production, "3");
    assert_eq!(result.needs[1].residual, "2");
    assert_eq!(result.needs[1].evaluations[0].assessment.max_batches, "1");
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "0");
    // Suggested production commands are executable while the observation remains fresh.
    for (i, line) in result.needs.iter().enumerate() {
        for suggestion in &line.production {
            assert_eq!(
                f.ledger
                    .plan_production(&format!("apply-{i}"), suggestion.request.clone())
                    .await
                    .unwrap()
                    .status,
                "reserved"
            );
        }
    }
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().available, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn caller_order_controls_stock_competition_between_needs_and_production() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let producer = f.ledger.resource(c.water).await.unwrap().producer;
    let supplier: Uuid =
        sqlx::query_scalar("SELECT unit_id FROM unit_providers WHERE producer_id=$1")
            .bind(producer)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let water = need(&f, f.resident, supplier, Category::Water, "ml", "20").await;
    let meals = need(&f, f.resident, c.unit, Category::Food, "meal", "6").await;
    let a = NeedOptions {
        need: water,
        resources: vec![c.water],
        production: vec![],
    };
    let b = NeedOptions {
        need: meals,
        resources: vec![],
        production: vec![option(&c)],
    };
    let first = f
        .ledger
        .propose_provisioning(proposal(vec![a.clone(), b.clone()]))
        .await
        .unwrap();
    assert_eq!(first.needs[1].residual, "6");
    assert!(
        first.needs[1].evaluations[0]
            .assessment
            .reasons
            .contains(&"input_shortage".into())
    );
    let reverse = f
        .ledger
        .propose_provisioning(proposal(vec![b, a]))
        .await
        .unwrap();
    assert_eq!(reverse.needs[0].residual, "0");
    assert_eq!(reverse.needs[1].stock_selected, "10");
    assert_eq!(reverse.needs[1].residual, "10");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn projected_surplus_is_not_reused_as_real_stock_or_coverage() {
    let f = Fixture::new().await;
    let c = setup(&f, "2", "10", "20").await;
    let first = need(&f, f.resident, c.unit, Category::Food, "meal", "1").await;
    let other = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: other,
            kind: AccountKind::Resident,
        })
        .await
        .unwrap();
    let second = need(&f, other, c.unit, Category::Food, "meal", "2").await;
    let result = f
        .ledger
        .propose_provisioning(proposal(vec![
            NeedOptions {
                need: first,
                resources: vec![],
                production: vec![option(&c)],
            },
            NeedOptions {
                need: second,
                resources: vec![],
                production: vec![option(&c)],
            },
        ]))
        .await
        .unwrap();
    assert_eq!(result.needs[0].production[0].surplus, "2");
    assert_eq!(result.needs[1].residual, "2");
    assert_eq!(f.ledger.need(first).await.unwrap().received, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn proposal_revalidates_scope_period_duplicates_and_current_holds() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let id = need(&f, f.resident, c.unit, Category::Food, "meal", "6").await;
    let entry = NeedOptions {
        need: id,
        resources: vec![],
        production: vec![option(&c)],
    };
    assert!(matches!(
        f.ledger
            .propose_provisioning(proposal(vec![entry.clone(), entry.clone()]))
            .await,
        Err(LedgerError::Invalid(_))
    ));
    let mut req = proposal(vec![entry.clone()]);
    req.period = "2026-11-03".into();
    assert!(matches!(
        f.ledger.propose_provisioning(req).await,
        Err(LedgerError::Invalid(_))
    ));
    let mut bad = entry.clone();
    bad.resources = vec![c.grain];
    assert!(matches!(
        f.ledger.propose_provisioning(proposal(vec![bad])).await,
        Err(LedgerError::Invalid(_))
    ));
    f.ledger
        .plan_production("existing", plan(&c, "4"))
        .await
        .unwrap();
    let after = f
        .ledger
        .propose_provisioning(proposal(vec![entry]))
        .await
        .unwrap();
    assert_eq!(after.needs[0].residual, "6");
    // The supplying unit is outside the executing unit's responsibility subtree.
    let water = need(&f, f.resident, c.unit, Category::Water, "ml", "1").await;
    assert!(matches!(
        f.ledger
            .propose_provisioning(proposal(vec![NeedOptions {
                need: water,
                resources: vec![c.water],
                production: vec![]
            }]))
            .await,
        Err(LedgerError::ProviderOutOfScope)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn proposal_respects_numeric_limits_and_already_covered_need() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let stock = meal_stock(&f, &c, "2", "9223372036854775807").await;
    let id = need(&f, f.resident, c.unit, Category::Food, "meal", "2").await;
    let r = f
        .ledger
        .propose_provisioning(proposal(vec![NeedOptions {
            need: id,
            resources: vec![stock],
            production: vec![],
        }]))
        .await
        .unwrap();
    assert_eq!(r.needs[0].stock_selected, "1");
    assert_eq!(r.needs[0].residual, "1");
    assert_eq!(
        r.accounting[0].required_for_selected_stock,
        "9223372036854775807"
    );
    let cheap = meal_stock(&f, &c, "2", "1").await;
    f.ledger.transfer("fund", f.transfer("10")).await.unwrap();
    f.ledger
        .reserve_need(
            "hold",
            id,
            qazna::needs::NeedReservationRequest {
                resource: cheap,
                quantity: amount("2"),
                basis: "Existing promise".into(),
            },
        )
        .await
        .unwrap();
    let r = f
        .ledger
        .propose_provisioning(proposal(vec![NeedOptions {
            need: id,
            resources: vec![stock],
            production: vec![option(&c)],
        }]))
        .await
        .unwrap();
    assert_eq!(r.needs[0].uncovered, "0");
    assert!(r.needs[0].stock.is_empty());
    assert!(r.needs[0].production.is_empty());
    assert_eq!(f.ledger.need(id).await.unwrap().unmet, "2");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn upgrade_from_completion_keeps_manual_needs_unattributed() {
    let f = Fixture::new_with_migrations(5).await;
    let c = setup(&f, "20", "10", "20").await;
    let old = need(&f, f.resident, c.unit, Category::Food, "meal", "3").await;
    MIGRATOR.run(&f.pool).await.unwrap();
    f.ledger.ready().await.unwrap();
    assert!(f.ledger.need_norm_source(old).await.unwrap().is_none());
    assert_eq!(f.ledger.need(old).await.unwrap().requested, "3");
    let n = norm("4");
    f.ledger.create_need_norm("norm", n.clone()).await.unwrap();
    assert!(
        sqlx::query("INSERT INTO need_norm_sources(need_id,norm_id) VALUES ($1,$2)")
            .bind(old)
            .bind(n.id)
            .execute(&f.pool)
            .await
            .is_err()
    );
    let new = f
        .ledger
        .create_need_from_norm(
            "new",
            NormNeedRequest {
                id: Uuid::new_v4(),
                resident: f.resident,
                assigned_unit: c.unit,
                period: "2026-11-03".into(),
                norm: n.id,
                basis: "Explicit new application".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(new.need.requested, "4");
}

#[path = "drafts.rs"]
mod drafts;

#[path = "coordination.rs"]
mod coordination;
