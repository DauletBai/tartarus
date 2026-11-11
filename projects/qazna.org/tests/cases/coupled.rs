use super::*;
use qazna::{coupled::*, model_scenarios::*, scenarios::*};
use serde_json::json;
fn decision() -> ScenarioDecision {
    ScenarioDecision {
        apply: true,
        basis: "Operator reviewed synthetic case".into(),
    }
}
fn physical_proposal(generation: i64) -> ModelProposal {
    ModelProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        input: serde_json::from_str(include_str!("../../fixtures/model-scenario.json")).unwrap(),
        basis: "Synthetic physical input".into(),
    }
}
fn proposal(generation: i64, model_generation: i64) -> CoupledProposal {
    CoupledProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        model_generation,
        unavailable_events: vec![],
        basis: "Synthetic linked circulation".into(),
    }
}
fn definition(model_id: Uuid) -> CoupledDefinition {
    CoupledDefinition {
        id: Uuid::new_v4(),
        format_version: 1,
        model_id,
        policy: serde_json::from_str(include_str!("../../fixtures/circulation-policy.json"))
            .unwrap(),
        basis: "Synthetic shared-production rule".into(),
    }
}
async fn physical(f: &Fixture, dated: Option<Uuid>) -> Uuid {
    let def = ModelDefinition {
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
        dated_bindings: dated
            .into_iter()
            .map(|scenario_id| DatedBinding {
                scenario_id,
                model_resources: vec![0],
                basis: "Same synthetic clock and unit".into(),
            })
            .collect(),
        basis: "fixture".into(),
    };
    let id = def.id;
    f.ledger
        .create_model_scenario("physical", def)
        .await
        .unwrap();
    let mut p = physical_proposal(0);
    if dated.is_some() {
        p.input.dated_generations = vec![1];
    }
    let d = f
        .ledger
        .propose_model_scenario("physical-plan", id, p)
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("physical-apply", d.request.id, decision())
        .await
        .unwrap();
    id
}
async fn create(f: &Fixture, model: Uuid) -> Uuid {
    let def = definition(model);
    let id = def.id;
    f.ledger
        .create_coupled_scenario("coupled", def)
        .await
        .unwrap();
    id
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn coupled_persistence_is_idempotent_and_never_posts_to_live_ledger() {
    let f = Fixture::new().await;
    let m = physical(&f, None).await;
    let id = create(&f, m).await;
    let p = proposal(0, 1);
    let d = f
        .ledger
        .propose_coupled_scenario("plan", id, p.clone())
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .coupled_scenario(id, None)
            .await
            .unwrap()
            .generation,
        0
    );
    assert_eq!(
        json!(d),
        json!(
            f.ledger
                .propose_coupled_scenario("plan", id, p.clone())
                .await
                .unwrap()
        )
    );
    let mut altered = p;
    altered.basis = "Different".into();
    assert!(matches!(
        f.ledger.propose_coupled_scenario("plan", id, altered).await,
        Err(LedgerError::IdempotencyConflict)
    ));
    let applied = f
        .ledger
        .resolve_coupled_scenario("apply", d.request.id, decision())
        .await
        .unwrap();
    assert_eq!(
        json!(applied),
        json!(
            f.ledger
                .resolve_coupled_scenario("apply", d.request.id, decision())
                .await
                .unwrap()
        )
    );
    let c: CoupledCheckpoint =
        serde_json::from_str(d.candidate.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(c.evidence.physical.generation, 1);
    assert_eq!(c.evidence.money.settled_volume, "198");
    assert_eq!(
        json!(f.ledger.coupled_scenario(id, Some(1)).await.unwrap()),
        json!(d.candidate)
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn changed_physical_head_blocks_approval_and_new_draft_retains_prefix() {
    let f = Fixture::new().await;
    let m = physical(&f, None).await;
    let id = create(&f, m).await;
    let old = f
        .ledger
        .propose_coupled_scenario("old", id, proposal(0, 1))
        .await
        .unwrap();
    let mut p = physical_proposal(1);
    p.input.boundary = 3;
    let physical = f
        .ledger
        .propose_model_scenario("physical-next", m, p)
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("physical-next-apply", physical.request.id, decision())
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .resolve_coupled_scenario("stale", old.request.id, decision())
            .await,
        Err(LedgerError::ScenarioStale)
    ));
    let d = f
        .ledger
        .propose_coupled_scenario("new", id, proposal(0, 2))
        .await
        .unwrap();
    f.ledger
        .resolve_coupled_scenario("apply-new", d.request.id, decision())
        .await
        .unwrap();
    let mut rewrite = proposal(1, 2);
    rewrite.unavailable_events = vec![1];
    assert!(
        f.ledger
            .propose_coupled_scenario("rewrite", id, rewrite)
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger
            .coupled_scenario(id, None)
            .await
            .unwrap()
            .generation,
        1
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_approvals_choose_one_generation_and_stale_draft_can_be_discarded() {
    let f = Fixture::new().await;
    let m = physical(&f, None).await;
    let id = create(&f, m).await;
    let a = f
        .ledger
        .propose_coupled_scenario("a", id, proposal(0, 1))
        .await
        .unwrap();
    let b = f
        .ledger
        .propose_coupled_scenario("b", id, proposal(0, 1))
        .await
        .unwrap();
    let (x, y) = tokio::join!(
        f.ledger
            .resolve_coupled_scenario("x", a.request.id, decision()),
        f.ledger
            .resolve_coupled_scenario("y", b.request.id, decision())
    );
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    let loser = if x.is_ok() {
        b.request.id
    } else {
        a.request.id
    };
    let error = match x {
        Err(e) => e,
        Ok(_) => y.unwrap_err(),
    };
    assert!(matches!(error, LedgerError::ScenarioStale));
    f.ledger
        .resolve_coupled_scenario(
            "discard",
            loser,
            ScenarioDecision {
                apply: false,
                basis: "Discard stale calculation".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .coupled_scenario(id, None)
            .await
            .unwrap()
            .generation,
        1
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn dated_change_invalidates_coupling_even_when_physical_head_has_not_changed() {
    let f = Fixture::new().await;
    let dated = Uuid::new_v4();
    f.ledger
        .create_dated_scenario(
            "dated",
            ScenarioDefinition {
                id: dated,
                format_version: 1,
                unit: "test_unit".into(),
                resources: vec!["warehouse".into()],
                basis: "Same synthetic timeline".into(),
            },
        )
        .await
        .unwrap();
    let d = f
        .ledger
        .propose_dated_scenario(
            "dated-plan",
            dated,
            ScenarioProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                basis: "Synthetic stock".into(),
                commands: vec![
                    DatedCommand::Advance { period: 0 },
                    DatedCommand::Advance { period: 1 },
                    DatedCommand::Introduce {
                        id: 1,
                        resource: 0,
                        quantity: 97,
                        available: 1,
                        expires: 20,
                        basis: "Synthetic stock".into(),
                    },
                ],
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_dated_scenario("dated-apply", d.request.id, decision())
        .await
        .unwrap();
    let m = physical(&f, Some(dated)).await;
    let id = create(&f, m).await;
    let coupled = f
        .ledger
        .propose_coupled_scenario("plan", id, proposal(0, 1))
        .await
        .unwrap();
    let d = f
        .ledger
        .propose_dated_scenario(
            "dated-changed",
            dated,
            ScenarioProposal {
                id: Uuid::new_v4(),
                expected_generation: 1,
                basis: "Updated future supply".into(),
                commands: vec![DatedCommand::Introduce {
                    id: 2,
                    resource: 0,
                    quantity: 1,
                    available: 10,
                    expires: 20,
                    basis: "Future supply".into(),
                }],
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_dated_scenario("dated-changed-apply", d.request.id, decision())
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .resolve_coupled_scenario("apply", coupled.request.id, decision())
            .await,
        Err(LedgerError::ScenarioStale)
    ));
    assert!(matches!(
        f.ledger
            .propose_coupled_scenario("new", id, proposal(0, 1))
            .await,
        Err(LedgerError::ScenarioStale)
    ));
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn coupled_history_fks_and_separate_transaction_review_are_enforced() {
    let f = Fixture::new().await;
    let m = physical(&f, None).await;
    let id = create(&f, m).await;
    let d = f
        .ledger
        .propose_coupled_scenario("plan", id, proposal(0, 1))
        .await
        .unwrap();
    for table in [
        "coupled_scenarios",
        "coupled_scenario_drafts",
        "coupled_scenario_outcomes",
    ] {
        for sql in [
            format!("DELETE FROM {table}"),
            format!("UPDATE {table} SET created_at=created_at"),
            format!("TRUNCATE {table} CASCADE"),
        ] {
            assert!(sqlx::query(&sql).execute(&f.pool).await.is_err());
        }
    }
    for savepoint in [false, true] {
        let mut tx = f.pool.begin().await.unwrap();
        let cloned = Uuid::new_v4();
        if savepoint {
            sqlx::query("SAVEPOINT nested")
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO coupled_scenario_drafts(id,scenario_id,base_generation,model_id,model_generation,request,candidate) SELECT $1,scenario_id,base_generation,model_id,model_generation,request,candidate FROM coupled_scenario_drafts WHERE id=$2").bind(cloned).bind(d.request.id).execute(&mut *tx).await.unwrap();
        if savepoint {
            sqlx::query("RELEASE SAVEPOINT nested")
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        assert!(sqlx::query("INSERT INTO coupled_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,1,'applied','same transaction')").bind(cloned).bind(id).execute(&mut *tx).await.is_err());
        tx.rollback().await.unwrap();
    }
    // A scenario cannot point to an unapproved physical generation.
    assert!(sqlx::query("INSERT INTO coupled_scenario_drafts(id,scenario_id,base_generation,model_id,model_generation,request,candidate) SELECT $1,scenario_id,base_generation,model_id,999,request,candidate FROM coupled_scenario_drafts WHERE id=$2").bind(Uuid::new_v4()).bind(d.request.id).execute(&f.pool).await.is_err());
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn coupled_commit_failure_is_atomic_and_same_key_can_be_retried() {
    let f = Fixture::new().await;
    let m = physical(&f, None).await;
    let id = create(&f, m).await;
    let d = f
        .ledger
        .propose_coupled_scenario("plan", id, proposal(0, 1))
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_coupled_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected'; END $$; CREATE CONSTRAINT TRIGGER fail_coupled_commit AFTER INSERT ON coupled_scenario_outcomes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fail_coupled_commit()").execute(&f.pool).await.unwrap();
    assert!(
        f.ledger
            .resolve_coupled_scenario("apply", d.request.id, decision())
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger
            .coupled_scenario(id, None)
            .await
            .unwrap()
            .generation,
        0
    );
    assert_eq!(
        f.ledger
            .coupled_scenario_draft(d.request.id)
            .await
            .unwrap()
            .state,
        "pending"
    );
    sqlx::query("DROP TRIGGER fail_coupled_commit ON coupled_scenario_outcomes")
        .execute(&f.pool)
        .await
        .unwrap();
    f.ledger
        .resolve_coupled_scenario("apply", d.request.id, decision())
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn coupled_api_rejects_model_credentials_and_injected_payments() {
    let f = Fixture::new().await;
    let m = physical(&f, None).await;
    let id = create(&f, m).await;
    let tokens =
        api::AccessTokens::new("o".repeat(32), Some("v".repeat(32)), Some("r".repeat(32))).unwrap();
    let app = api::router_with_access(f.ledger.clone(), tokens);
    let mut value = json!(proposal(0, 1));
    value["events"] = json!([]);
    for (token, body, expected) in [
        ("o", value.to_string(), 422),
        ("v", json!(proposal(0, 1)).to_string(), 403),
        ("r", json!(proposal(0, 1)).to_string(), 403),
        ("o", " ".repeat(16385), 413),
    ] {
        let result = app
            .clone()
            .oneshot(
                Request::post(format!("/v1/scenarios/coupled/{id}/drafts"))
                    .header("authorization", format!("Bearer {}", token.repeat(32)))
                    .header("content-type", "application/json")
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status().as_u16(), expected);
    }
    let d = f
        .ledger
        .propose_coupled_scenario("plan", id, proposal(0, 1))
        .await
        .unwrap();
    for token in ["v", "r"] {
        let result = app
            .clone()
            .oneshot(
                Request::post(format!("/v1/coupled-drafts/{}/resolve", d.request.id))
                    .header("authorization", format!("Bearer {}", token.repeat(32)))
                    .header("content-type", "application/json")
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .body(Body::from(json!(decision()).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::FORBIDDEN);
    }
}
