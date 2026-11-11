use super::*;
use qazna::{model_scenarios::*, scenarios::*};
use serde_json::json;
fn definition() -> ModelDefinition {
    ModelDefinition {
        id: Uuid::new_v4(),
        format_version: 1,
        resources: ["material", "machine", "food"]
            .into_iter()
            .map(|name| ModelResource {
                name: name.into(),
                unit: "test_unit".into(),
            })
            .collect(),
        period_unit: "day".into(),
        time_origin: "Synthetic day zero".into(),
        dated_bindings: vec![],
        basis: "Synthetic fixture".into(),
    }
}
fn proposal(generation: i64) -> ModelProposal {
    ModelProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        input: serde_json::from_str(include_str!("../../fixtures/model-scenario.json")).unwrap(),
        basis: "Reviewed synthetic input".into(),
    }
}
fn decision() -> ScenarioDecision {
    ScenarioDecision {
        apply: true,
        basis: "Operator approval".into(),
    }
}
async fn create(f: &Fixture) -> Uuid {
    f.ledger
        .create_model_scenario("create", definition())
        .await
        .unwrap()
        .definition
        .id
}
async fn apply(f: &Fixture, id: Uuid, key: &str, p: ModelProposal) -> ModelDraft {
    let d = f.ledger.propose_model_scenario(key, id, p).await.unwrap();
    f.ledger
        .resolve_model_scenario(&format!("apply-{key}"), d.request.id, decision())
        .await
        .unwrap()
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn approved_full_state_retains_work_cohorts_exact_mass_and_history() {
    let f = Fixture::new().await;
    let id = create(&f).await;
    let p = proposal(0);
    let d = f
        .ledger
        .propose_model_scenario("proposal", id, p.clone())
        .await
        .unwrap();
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        0
    );
    let c: ModelCheckpoint =
        serde_json::from_str(d.candidate.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(c.rolling.opening_stock, vec![97, 1, 0]);
    assert_eq!(c.rolling.pending.len(), 2);
    assert_eq!(c.rolling.opening_cohorts[0].expires, 3);
    assert_eq!(c.materials[0].initial, 102);
    assert_eq!(c.materials[0].work_in_progress[1], 2);
    assert_eq!(
        json!(
            f.ledger
                .propose_model_scenario("proposal", id, p.clone())
                .await
                .unwrap()
        ),
        json!(d)
    );
    let mut changed = p;
    changed.basis = "Different request".into();
    assert!(matches!(
        f.ledger
            .propose_model_scenario("proposal", id, changed)
            .await,
        Err(LedgerError::IdempotencyConflict)
    ));
    let applied = f
        .ledger
        .resolve_model_scenario("apply", d.request.id, decision())
        .await
        .unwrap();
    assert_eq!(
        json!(
            f.ledger
                .resolve_model_scenario("apply", d.request.id, decision())
                .await
                .unwrap()
        ),
        json!(applied)
    );
    let mut next = proposal(1);
    next.input.boundary = 3;
    let d2 = apply(&f, id, "next", next).await;
    let c2: ModelCheckpoint =
        serde_json::from_str(d2.candidate.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(c2.rolling.opening_stock, vec![96, 2, 0]);
    assert_eq!(
        c2.rolling
            .opening_cohorts
            .iter()
            .map(|v| v.expires)
            .collect::<Vec<_>>(),
        vec![3, 6]
    );
    assert_eq!(
        json!(f.ledger.model_scenario(id, Some(1)).await.unwrap()),
        json!(d.candidate)
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn replanning_rejects_rewritten_prefix_rejuvenation_false_mass_and_bad_dimensions() {
    let f = Fixture::new().await;
    let id = create(&f).await;
    apply(&f, id, "initial", proposal(0)).await;
    for mutation in 0..9 {
        let mut p = proposal(1);
        match mutation {
            0 => {
                p.input.plan[0].discarded[0] = 1;
                for row in &mut p.input.plan {
                    row.closing[0] -= 1;
                }
            }
            1 => p.input.model.lives[0].initial_remaining_periods = 4,
            2 => p.input.model.seasonal.physical.resources[0] = "renamed".into(),
            3 => p.input.boundary = 1,
            4 => p.input.boundary = 15,
            5 => p.input.material_bases[0].quantity_per_resource_unit[1] = 2,
            6 => p
                .input
                .model
                .seasonal
                .physical
                .periods
                .resize(121, p.input.model.seasonal.physical.periods[0].clone()),
            7 => p.input.plan[5].closing[0] += 1,
            _ => {
                p.input.boundary = 3;
                p.input.plan[2].discarded[0] = 1;
                for row in &mut p.input.plan[2..] {
                    row.closing[0] -= 1;
                }
            }
        }
        assert!(
            f.ledger
                .propose_model_scenario("retry", id, p)
                .await
                .is_err(),
            "mutation {mutation}"
        );
    }
    let mut p = proposal(1);
    p.input.model.seasonal.physical.periods[5].supplies[0] = 2;
    for row in &mut p.input.plan[5..] {
        row.closing[0] += 2;
    }
    let d = apply(&f, id, "retry", p).await;
    assert_eq!(d.candidate.generation, 2);
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_approvals_have_one_head_and_discard_does_not_advance_it() {
    let f = Fixture::new().await;
    let id = create(&f).await;
    let a = f
        .ledger
        .propose_model_scenario("a", id, proposal(0))
        .await
        .unwrap();
    let b = f
        .ledger
        .propose_model_scenario("b", id, proposal(0))
        .await
        .unwrap();
    let (x, y) = tokio::join!(
        f.ledger
            .resolve_model_scenario("x", a.request.id, decision()),
        f.ledger
            .resolve_model_scenario("y", b.request.id, decision())
    );
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    let loser = if x.is_ok() {
        b.request.id
    } else {
        a.request.id
    };
    let error = match x {
        Err(error) => error,
        Ok(_) => y.unwrap_err(),
    };
    assert!(matches!(error, LedgerError::ScenarioStale));
    let result = f
        .ledger
        .resolve_model_scenario(
            "discard",
            loser,
            ScenarioDecision {
                apply: false,
                basis: "Stale proposal discarded".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.state, "discarded");
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        1
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn dated_anchor_is_pinned_and_must_still_be_current_on_approval() {
    let f = Fixture::new().await;
    let dated_id = Uuid::new_v4();
    f.ledger
        .create_dated_scenario(
            "dated",
            ScenarioDefinition {
                id: dated_id,
                format_version: 1,
                unit: "test_unit".into(),
                resources: vec!["warehouse".into()],
                basis: "Synthetic clock aligned with model day zero".into(),
            },
        )
        .await
        .unwrap();
    let d = f
        .ledger
        .propose_dated_scenario(
            "opening",
            dated_id,
            ScenarioProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                basis: "Synthetic 97 units at day 1".into(),
                commands: vec![
                    DatedCommand::Advance { period: 0 },
                    DatedCommand::Advance { period: 1 },
                    DatedCommand::Introduce {
                        id: 1,
                        resource: 0,
                        quantity: 97,
                        available: 1,
                        expires: 20,
                        basis: "Synthetic opening".into(),
                    },
                ],
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_dated_scenario("approve-opening", d.request.id, decision())
        .await
        .unwrap();
    let mut def = definition();
    def.dated_bindings = vec![DatedBinding {
        scenario_id: dated_id,
        model_resources: vec![0],
        basis: "Same absolute day and physical unit".into(),
    }];
    let id = def.id;
    f.ledger
        .create_model_scenario("model", def.clone())
        .await
        .unwrap();
    let mut bad = def;
    bad.id = Uuid::new_v4();
    bad.resources[0].unit = "another unit".into();
    assert!(
        f.ledger
            .create_model_scenario("bad-unit", bad)
            .await
            .is_err()
    );
    let mut p = proposal(0);
    p.input.dated_generations = vec![1];
    let a = f
        .ledger
        .propose_model_scenario("a", id, p.clone())
        .await
        .unwrap();
    let c: ModelCheckpoint =
        serde_json::from_str(a.candidate.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(c.dated[0].checkpoint, d.candidate.checkpoint);
    let delta = f
        .ledger
        .propose_dated_scenario(
            "delta",
            dated_id,
            ScenarioProposal {
                id: Uuid::new_v4(),
                expected_generation: 1,
                basis: "Change future harvest only".into(),
                commands: vec![DatedCommand::Introduce {
                    id: 2,
                    resource: 0,
                    quantity: 1,
                    available: 10,
                    expires: 20,
                    basis: "Synthetic future lot".into(),
                }],
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_dated_scenario("delta-apply", delta.request.id, decision())
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .resolve_model_scenario("apply", a.request.id, decision())
            .await,
        Err(LedgerError::ScenarioStale)
    ));
    p.id = Uuid::new_v4();
    p.input.dated_generations = vec![2];
    apply(&f, id, "renewed", p).await;
    let mut next = proposal(1);
    next.input.dated_generations = vec![2];
    next.input.boundary = 3;
    assert!(
        f.ledger
            .propose_model_scenario("wrong-clock", id, next)
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        1
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn model_history_is_immutable_and_same_transaction_review_is_rejected() {
    let f = Fixture::new().await;
    let id = create(&f).await;
    let d = f
        .ledger
        .propose_model_scenario("proposal", id, proposal(0))
        .await
        .unwrap();
    for table in [
        "model_scenarios",
        "model_scenario_drafts",
        "model_scenario_outcomes",
    ] {
        for statement in [
            format!("DELETE FROM {table}"),
            format!("UPDATE {table} SET created_at=created_at"),
            format!("TRUNCATE {table} CASCADE"),
        ] {
            assert!(sqlx::query(&statement).execute(&f.pool).await.is_err());
        }
    }
    for savepoint in [false, true] {
        let mut tx = f.pool.begin().await.unwrap();
        let clone = Uuid::new_v4();
        if savepoint {
            sqlx::query("SAVEPOINT insert_draft")
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO model_scenario_drafts SELECT $1,scenario_id,base_generation,request,candidate,pg_current_xact_id(),clock_timestamp() FROM model_scenario_drafts WHERE id=$2").bind(clone).bind(d.request.id).execute(&mut *tx).await.unwrap();
        if savepoint {
            sqlx::query("RELEASE SAVEPOINT insert_draft")
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        assert!(sqlx::query("INSERT INTO model_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,1,'applied','same transaction')").bind(clone).bind(id).execute(&mut *tx).await.is_err());
        tx.rollback().await.unwrap();
    }
    assert!(sqlx::query("INSERT INTO model_scenario_drafts SELECT $1,scenario_id,base_generation,request,candidate,'1'::xid8,clock_timestamp() FROM model_scenario_drafts WHERE id=$2").bind(Uuid::new_v4()).bind(d.request.id).execute(&f.pool).await.is_err());
    f.ledger
        .resolve_model_scenario("apply", d.request.id, decision())
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn deferred_failure_rolls_back_outcome_and_keeps_retry_key_available() {
    let f = Fixture::new().await;
    let id = create(&f).await;
    let d = f
        .ledger
        .propose_model_scenario("proposal", id, proposal(0))
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_model_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred failure'; END $$; CREATE CONSTRAINT TRIGGER fail_model_commit AFTER INSERT ON model_scenario_outcomes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fail_model_commit()").execute(&f.pool).await.unwrap();
    assert!(
        f.ledger
            .resolve_model_scenario("apply", d.request.id, decision())
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        0
    );
    assert_eq!(
        f.ledger
            .model_scenario_draft(d.request.id)
            .await
            .unwrap()
            .state,
        "pending"
    );
    sqlx::query("DROP TRIGGER fail_model_commit ON model_scenario_outcomes")
        .execute(&f.pool)
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("apply", d.request.id, decision())
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn model_http_limits_are_scoped_and_observers_cannot_apply_or_import() {
    let f = Fixture::new().await;
    let id = create(&f).await;
    let operator = "o".repeat(32);
    let observer = "v".repeat(32);
    let reviewer = "r".repeat(32);
    let app = api::router_with_access(
        f.ledger.clone(),
        api::AccessTokens::new(
            operator.clone(),
            Some(observer.clone()),
            Some(reviewer.clone()),
        )
        .unwrap(),
    );
    let route = format!("/v1/scenarios/models/{id}/drafts");
    let encoded = serde_json::to_string(&proposal(0)).unwrap();
    // Legal whitespace exceeds the old general body limit, but not the model limit.
    let large = format!("{}{}", " ".repeat(20000), encoded);
    for (path, token, body, status) in [
        (route.clone(), operator.clone(), large, 200),
        (
            route.clone(),
            operator.clone(),
            " ".repeat(MAX_MODEL_REQUEST_BYTES + 1),
            413,
        ),
        (
            "/v1/scenarios/models".into(),
            operator.clone(),
            " ".repeat(20000),
            413,
        ),
        (route.clone(), observer.clone(), encoded.clone(), 403),
        (route.clone(), reviewer.clone(), encoded.clone(), 403),
        (
            route.clone(),
            operator.clone(),
            encoded.replace("\"input\":", "\"checkpoint\":\"forged\",\"input\":"),
            422,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(path)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
    let d = f
        .ledger
        .propose_model_scenario("pending", id, proposal(0))
        .await
        .unwrap();
    for token in [observer, reviewer] {
        let response = app
            .clone()
            .oneshot(
                Request::post(format!("/v1/model-drafts/{}/resolve", d.request.id))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .body(Body::from(serde_json::to_string(&decision()).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        0
    );
}
