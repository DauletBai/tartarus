use super::*;
use qazna::{
    domain::{AccountKind, NewAccount},
    needs::{ProviderRequest, UnitRequest},
    production::*,
    provisioning::*,
};
fn amount(s: &str) -> Amount {
    Amount::parse(s).unwrap()
}
fn reason() -> ResolutionRequest {
    ResolutionRequest {
        basis: "Synthetic withdrawal".into(),
    }
}
struct Context {
    unit: Uuid,
    capacity: Uuid,
    recipe: RecipeRequest,
    grain: Uuid,
    water: Uuid,
}
async fn setup(f: &Fixture, cap: &str, grain: &str, water: &str) -> Context {
    let mut units = Vec::new();
    for name in ["Kitchen", "Supplies"] {
        let id = Uuid::new_v4();
        f.ledger
            .create_unit(
                name,
                UnitRequest {
                    id,
                    parent: None,
                    name: name.into(),
                    function: "Synthetic provision".into(),
                    basis: "Test".into(),
                },
            )
            .await
            .unwrap();
        units.push(id);
    }
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
            units[1],
            ProviderRequest {
                producer,
                basis: "Test".into(),
            },
        )
        .await
        .unwrap();
    let mut resources = Vec::new();
    for (name, category, unit, quantity) in [
        ("grain", Category::Food, "gram", grain),
        ("water", Category::Water, "ml", water),
    ] {
        let id = Uuid::new_v4();
        f.ledger
            .create_resource(
                name,
                ResourceRequest {
                    id,
                    producer,
                    category,
                    unit: unit.into(),
                    quantity: amount(quantity),
                    unit_measure: amount("1"),
                    basis: "Synthetic stock".into(),
                },
            )
            .await
            .unwrap();
        resources.push(id);
    }
    let capacity = Uuid::new_v4();
    f.ledger
        .create_capacity(
            "capacity",
            CapacityRequest {
                id: capacity,
                unit: units[0],
                period: "2026-11-02".into(),
                capacity_unit: "kitchen_minutes".into(),
                quantity: amount(cap),
                basis: "Declared experimental capacity".into(),
            },
        )
        .await
        .unwrap();
    let recipe = f
        .ledger
        .create_recipe(
            "recipe",
            RecipeRequest {
                id: Uuid::new_v4(),
                unit: units[0],
                name: "Meals".into(),
                capacity_unit: "kitchen_minutes".into(),
                capacity_per_batch: amount("2"),
                output_category: Category::Food,
                output_unit: "meal".into(),
                output_per_batch: amount("3"),
                inputs: vec![
                    RecipeInput {
                        slot: "grain".into(),
                        category: Category::Food,
                        unit: "gram".into(),
                        quantity_per_batch: amount("2"),
                    },
                    RecipeInput {
                        slot: "water".into(),
                        category: Category::Water,
                        unit: "ml".into(),
                        quantity_per_batch: amount("5"),
                    },
                ],
                basis: "Experimental recipe, not a verified nutrition norm".into(),
            },
        )
        .await
        .unwrap();
    Context {
        unit: units[0],
        capacity,
        recipe,
        grain: resources[0],
        water: resources[1],
    }
}
fn plan(c: &Context, batches: &str) -> PlanRequest {
    PlanRequest {
        recipe: c.recipe.id,
        capacity: c.capacity,
        batches: amount(batches),
        inputs: vec![
            InputSelection {
                slot: "grain".into(),
                resource: c.grain,
            },
            InputSelection {
                slot: "water".into(),
                resource: c.water,
            },
        ],
        basis: "Proposed physical production".into(),
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn production_assessment_and_durable_shortage_feedback() {
    let f = Fixture::new().await;
    let c = setup(&f, "6", "10", "20").await;
    let a = f.ledger.assess_production(plan(&c, "5")).await.unwrap();
    assert!(!a.feasible);
    assert_eq!(a.max_batches, "3");
    assert_eq!(a.planned_output, "15");
    assert_eq!(a.reasons, vec!["capacity_shortage", "input_shortage"]);
    assert_eq!(a.inputs[0].shortage, "0");
    assert_eq!(a.inputs[1].shortage, "5");
    assert_ne!(a.inputs[0].supplying_unit, c.unit);
    assert_eq!(f.ledger.resource(c.water).await.unwrap().reserved, "0");
    let held = f
        .ledger
        .plan_production("hold", plan(&c, "3"))
        .await
        .unwrap();
    assert_eq!(held.status, "reserved");
    let blocked = f
        .ledger
        .plan_production("blocked", plan(&c, "1"))
        .await
        .unwrap();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.assessment.max_batches, "0");
    let id = held.plan_id.unwrap();
    f.ledger
        .cancel_production("cancel", id, reason())
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .plan_production("blocked", plan(&c, "1"))
            .await
            .unwrap()
            .status,
        "blocked"
    );
    assert_eq!(
        f.ledger
            .plan_production("new-attempt", plan(&c, "1"))
            .await
            .unwrap()
            .status,
        "reserved"
    );
    assert_eq!(f.balance(f.resident).await, "0");
    assert_eq!(f.transaction_count().await, 1);
    let lots: i64 = sqlx::query_scalar("SELECT count(*) FROM resources")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(lots, 2, "planning must not manufacture stock");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn production_holds_interoperate_with_resident_provision_and_replay() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let req = plan(&c, "4");
    let held = f.ledger.plan_production("hold", req.clone()).await.unwrap();
    let mut reordered = req;
    reordered.inputs.reverse();
    assert_eq!(
        f.ledger
            .plan_production("hold", reordered)
            .await
            .unwrap()
            .plan_id,
        held.plan_id
    );
    assert!(matches!(
        f.ledger.plan_production("hold", plan(&c, "3")).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().available, "12");
    assert_eq!(f.ledger.resource(c.water).await.unwrap().available, "0");
    let resident_req = ReservationRequest {
        resident: f.resident,
        resource: c.water,
        quantity: amount("1"),
        basis: "Drinking water".into(),
    };
    assert!(matches!(
        f.ledger.reserve("drink", resident_req.clone()).await,
        Err(LedgerError::InsufficientStock)
    ));
    let id = held.plan_id.unwrap();
    assert_eq!(
        Ledger::new(f.pool.clone())
            .production_plan(id)
            .await
            .unwrap()
            .state,
        "reserved"
    );
    let cancelled = f
        .ledger
        .cancel_production("cancel", id, reason())
        .await
        .unwrap();
    assert_eq!(cancelled.state, "cancelled");
    assert_eq!(
        f.ledger
            .cancel_production("cancel", id, reason())
            .await
            .unwrap()
            .state,
        "cancelled"
    );
    assert!(matches!(
        f.ledger.cancel_production("again", id, reason()).await,
        Err(LedgerError::ProductionClosed)
    ));
    let r = f.ledger.reserve("drink", resident_req).await.unwrap();
    f.ledger.settle("received", r.id, reason()).await.unwrap();
    assert_eq!(
        f.ledger
            .assess_production(plan(&c, "4"))
            .await
            .unwrap()
            .max_batches,
        "3"
    );
    assert_eq!(f.ledger.resource(c.water).await.unwrap().on_hand, "19");
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().available, "20");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_recipes_share_capacity_without_overbooking() {
    let f = Fixture::new().await;
    let c = setup(&f, "6", "100", "100").await;
    let mut alternative = c.recipe.clone();
    alternative.id = Uuid::new_v4();
    alternative.output_per_batch = amount("4");
    f.ledger
        .create_recipe("alternative", alternative.clone())
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for i in 0..8 {
        let l = f.ledger.clone();
        let mut req = plan(&c, "1");
        if i % 2 == 0 {
            req.recipe = alternative.id;
        }
        tasks.push(tokio::spawn(async move {
            l.plan_production(&format!("parallel-{i}"), req)
                .await
                .unwrap()
        }));
    }
    let mut count = 0;
    for task in tasks {
        let a = task.await.unwrap();
        if a.status == "reserved" {
            count += 1
        } else {
            assert_eq!(a.assessment.reasons, vec!["capacity_shortage"]);
        }
    }
    assert_eq!(count, 3);
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().available, "0");
    assert_eq!(f.ledger.resource(c.grain).await.unwrap().reserved, "6");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn resident_and_production_race_for_same_physical_lot() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "5").await;
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let l = f.ledger.clone();
    let req = plan(&c, "1");
    let producer = tokio::spawn(async move { l.plan_production("production", req).await.unwrap() });
    let l = f.ledger.clone();
    let resident = f.resident;
    let resource = c.water;
    let consumer = tokio::spawn(async move {
        l.reserve(
            "resident",
            ReservationRequest {
                resident,
                resource,
                quantity: amount("5"),
                basis: "Resident water".into(),
            },
        )
        .await
    });
    let p = producer.await.unwrap();
    let r = consumer.await.unwrap();
    assert_eq!(usize::from(p.plan_id.is_some()) + usize::from(r.is_ok()), 1);
    if let Err(e) = r {
        assert!(matches!(e, LedgerError::InsufficientStock));
    }
    assert_eq!(f.ledger.resource(c.water).await.unwrap().reserved, "5");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn periods_share_stock_and_invalid_inputs_do_not_claim_capacity() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let mut wrong = plan(&c, "1");
    wrong.inputs[1].resource = c.grain;
    assert!(matches!(
        f.ledger.plan_production("invalid", wrong).await,
        Err(LedgerError::Invalid(_))
    ));
    let mut wrong = plan(&c, "1");
    wrong.inputs.swap(0, 1);
    wrong.inputs[0].slot = "grain".into();
    wrong.inputs[1].slot = "water".into();
    assert!(matches!(
        f.ledger.plan_production("invalid", wrong).await,
        Err(LedgerError::Invalid(_))
    ));
    let cap = f.ledger.capacity(c.capacity).await.unwrap().declaration;
    let mut duplicate = cap.clone();
    duplicate.id = Uuid::new_v4();
    assert!(matches!(
        f.ledger.create_capacity("dup", duplicate).await,
        Err(LedgerError::ProductionConflict)
    ));
    let mut next = cap;
    next.id = Uuid::new_v4();
    next.period = "2026-11-03".into();
    f.ledger
        .create_capacity("tomorrow", next.clone())
        .await
        .unwrap();
    f.ledger
        .plan_production("invalid", plan(&c, "3"))
        .await
        .unwrap();
    let mut req = plan(&c, "2");
    req.capacity = next.id;
    let a = f
        .ledger
        .plan_production("tomorrow-plan", req)
        .await
        .unwrap();
    assert_eq!(a.status, "blocked");
    assert_eq!(a.assessment.max_batches, "1");
    assert_eq!(f.ledger.capacity(next.id).await.unwrap().reserved, "0");
    let mut wrong_recipe = c.recipe.clone();
    wrong_recipe.id = Uuid::new_v4();
    wrong_recipe.capacity_unit = "another_machine_minutes".into();
    f.ledger
        .create_recipe("wrong-machine", wrong_recipe.clone())
        .await
        .unwrap();
    let mut req = plan(&c, "1");
    req.recipe = wrong_recipe.id;
    assert!(matches!(
        f.ledger.plan_production("wrong-machine-plan", req).await,
        Err(LedgerError::Invalid(_))
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn production_failure_rolls_back_all_holds_and_command() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    sqlx::raw_sql("CREATE FUNCTION fail_input() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.slot='water' THEN RAISE EXCEPTION 'injected input failure'; END IF; RETURN NEW; END; $$; CREATE TRIGGER fail_input BEFORE INSERT ON production_plan_inputs FOR EACH ROW EXECUTE FUNCTION fail_input();").execute(&f.pool).await.unwrap();
    assert!(
        f.ledger
            .plan_production("retry", plan(&c, "2"))
            .await
            .is_err()
    );
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "0");
    assert_eq!(f.ledger.resource(c.grain).await.unwrap().reserved, "0");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM commands WHERE key='retry'")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER fail_input ON production_plan_inputs")
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .plan_production("retry", plan(&c, "2"))
            .await
            .unwrap()
            .status,
        "reserved"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn raw_sql_cannot_overbook_omit_inputs_or_rewrite_recipes() {
    let f = Fixture::new().await;
    let c = setup(&f, "2", "10", "20").await;
    for (batches, inputs) in [(2, true), (1, false)] {
        let mut tx = f.pool.begin().await.unwrap();
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO production_plans(id,recipe_id,capacity_id,batches,basis) VALUES ($1,$2,$3,$4,'raw test')").bind(id).bind(c.recipe.id).bind(c.capacity).bind(batches as i64).execute(&mut *tx).await.unwrap();
        if inputs {
            for (slot, resource, quantity) in [
                ("grain", c.grain, 2 * batches),
                ("water", c.water, 5 * batches),
            ] {
                sqlx::query("INSERT INTO production_plan_inputs(plan_id,slot,resource_id,quantity) VALUES ($1,$2,$3,$4)").bind(id).bind(slot).bind(resource).bind(quantity as i64).execute(&mut *tx).await.unwrap();
            }
        }
        assert!(tx.commit().await.is_err());
    }
    assert!(sqlx::query("INSERT INTO production_recipe_inputs(recipe_id,slot,category,unit,quantity_per_batch) VALUES ($1,'late','food','gram',1)").bind(c.recipe.id).execute(&f.pool).await.is_err());
    assert!(
        sqlx::query("UPDATE production_capacities SET quantity=100 WHERE id=$1")
            .bind(c.capacity)
            .execute(&f.pool)
            .await
            .is_err()
    );
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "0");
    // A resident hold is also protected against bypassing the production service.
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    f.ledger
        .reserve(
            "resident",
            ReservationRequest {
                resident: f.resident,
                resource: c.water,
                quantity: amount("20"),
                basis: "Water".into(),
            },
        )
        .await
        .unwrap();
    let mut tx = f.pool.begin().await.unwrap();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO production_plans(id,recipe_id,capacity_id,batches,basis) VALUES ($1,$2,$3,1,'raw stock test')").bind(id).bind(c.recipe.id).bind(c.capacity).execute(&mut *tx).await.unwrap();
    for (slot, resource, quantity) in [("grain", c.grain, 2i64), ("water", c.water, 5)] {
        sqlx::query("INSERT INTO production_plan_inputs(plan_id,slot,resource_id,quantity) VALUES ($1,$2,$3,$4)").bind(id).bind(slot).bind(resource).bind(quantity).execute(&mut *tx).await.unwrap();
    }
    assert!(tx.commit().await.is_err());
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn production_uses_exact_wide_assessment_and_caps_storable_output() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "20").await;
    let mut recipe = c.recipe.clone();
    recipe.id = Uuid::new_v4();
    recipe.output_per_batch = amount("9223372036854775807");
    f.ledger
        .create_recipe("large", recipe.clone())
        .await
        .unwrap();
    let mut req = plan(&c, "2");
    req.recipe = recipe.id;
    let a = f.ledger.plan_production("overflow", req).await.unwrap();
    assert_eq!(a.status, "blocked");
    assert_eq!(a.assessment.reasons, vec!["quantity_limit"]);
    assert_eq!(a.assessment.planned_output, "18446744073709551614");
    assert_eq!(a.assessment.max_batches, "1");
    let a = f
        .ledger
        .assess_production(plan(&c, "9223372036854775807"))
        .await
        .unwrap();
    assert!(a.reasons.contains(&"quantity_limit".into()));
    assert_eq!(a.inputs[1].required, "46116860184273879035");
    assert_eq!(f.ledger.resource(c.water).await.unwrap().reserved, "0");
    // Source processes may explicitly have no modeled inputs; this is an assumption, not verified growth.
    recipe.id = Uuid::new_v4();
    recipe.inputs.clear();
    recipe.output_per_batch = amount("1");
    f.ledger
        .create_recipe("source", recipe.clone())
        .await
        .unwrap();
    let mut req = plan(&c, "2");
    req.recipe = recipe.id;
    req.inputs.clear();
    assert_eq!(
        f.ledger
            .plan_production("source-plan", req)
            .await
            .unwrap()
            .status,
        "reserved"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn upgrade_from_needs_preserves_existing_reservations() {
    let f = Fixture::new_with_migrations(3).await;
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let producer = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: producer,
            kind: AccountKind::Producer,
        })
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
                quantity: amount("10"),
                unit_measure: amount("2"),
                basis: "Old lot".into(),
            },
        )
        .await
        .unwrap();
    let r = f
        .ledger
        .reserve(
            "hold",
            ReservationRequest {
                resident: f.resident,
                resource,
                quantity: amount("3"),
                basis: "Old reservation".into(),
            },
        )
        .await
        .unwrap();
    MIGRATOR.run(&f.pool).await.unwrap();
    f.ledger.ready().await.unwrap();
    assert_eq!(f.ledger.resource(resource).await.unwrap().available, "7");
    f.ledger.settle("settle", r.id, reason()).await.unwrap();
    assert_eq!(f.ledger.resource(resource).await.unwrap().on_hand, "7");
    assert_eq!(f.balance(f.resident).await, "94");
    let c = setup(&f, "20", "10", "20").await;
    assert_eq!(
        f.ledger
            .plan_production("new-plan", plan(&c, "2"))
            .await
            .unwrap()
            .status,
        "reserved"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_periods_cannot_double_promise_shared_inputs() {
    let f = Fixture::new().await;
    let c = setup(&f, "20", "10", "5").await;
    let mut next = f.ledger.capacity(c.capacity).await.unwrap().declaration;
    next.id = Uuid::new_v4();
    next.period = "2026-11-03".into();
    f.ledger
        .create_capacity("tomorrow", next.clone())
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for (key, capacity) in [("today", c.capacity), ("tomorrow-plan", next.id)] {
        let l = f.ledger.clone();
        let mut request = plan(&c, "1");
        request.capacity = capacity;
        tasks.push(tokio::spawn(async move {
            l.plan_production(key, request).await.unwrap()
        }));
    }
    let mut held = 0;
    for task in tasks {
        let result = task.await.unwrap();
        if result.plan_id.is_some() {
            held += 1;
        } else {
            assert_eq!(result.assessment.reasons, vec!["input_shortage"]);
        }
    }
    assert_eq!(held, 1);
    assert_eq!(f.ledger.resource(c.water).await.unwrap().reserved, "5");
    let first = f
        .ledger
        .capacity(c.capacity)
        .await
        .unwrap()
        .reserved
        .parse::<i64>()
        .unwrap();
    let second = f
        .ledger
        .capacity(next.id)
        .await
        .unwrap()
        .reserved
        .parse::<i64>()
        .unwrap();
    assert_eq!(first + second, 2);
}
