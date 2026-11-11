use super::*;
use qazna::scenarios::*;
use serde_json::json;

fn definition() -> ScenarioDefinition {
    ScenarioDefinition {
        id: Uuid::new_v4(),
        format_version: 1,
        unit: "grain_kg".into(),
        resources: vec!["store".into(), "kitchen".into()],
        basis: "Synthetic dated flow v1".into(),
    }
}
fn proposal(generation: i64, commands: Vec<DatedCommand>) -> ScenarioProposal {
    ScenarioProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        commands,
        basis: "Synthetic scenario observation".into(),
    }
}
fn opening() -> Vec<DatedCommand> {
    vec![
        DatedCommand::Advance { period: 0 },
        DatedCommand::Introduce {
            id: 1,
            resource: 0,
            quantity: 10,
            available: 0,
            expires: 4,
            basis: "Synthetic opening lot".into(),
        },
    ]
}
fn decision(apply: bool) -> ScenarioDecision {
    ScenarioDecision {
        apply,
        basis: "Operator reviewed synthetic changes".into(),
    }
}
async fn create(f: &Fixture) -> ScenarioSnapshot {
    f.ledger
        .create_dated_scenario("create", definition())
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn committed_generation_is_distinct_from_draft_and_restores_identity() {
    let f = Fixture::new().await;
    let initial = create(&f).await;
    let id = initial.definition.id;
    let mut commands = opening();
    commands.push(DatedCommand::Transfer {
        source: 0,
        target: 1,
        quantity: 4,
        due: 1,
    });
    commands.push(DatedCommand::Introduce {
        id: 2,
        resource: 0,
        quantity: 5,
        available: 2,
        expires: 6,
        basis: "Committed harvest".into(),
    });
    let draft = f
        .ledger
        .propose_dated_scenario("propose", id, proposal(0, commands))
        .await
        .unwrap();
    assert_eq!(f.ledger.dated_scenario(id, None).await.unwrap(), initial);
    assert!(matches!(
        f.ledger.dated_scenario(id, Some(1)).await,
        Err(LedgerError::ScenarioNotFound)
    ));
    let applied = f
        .ledger
        .resolve_dated_scenario("apply", draft.request.id, decision(true))
        .await
        .unwrap();
    assert_eq!(applied.state, "applied");
    assert_eq!(applied.candidate.generation, 1);
    assert_eq!(applied.candidate.flow_revision, 4);
    let reopened = Ledger::new(f.pool.clone());
    assert_eq!(
        reopened.dated_scenario(id, None).await.unwrap(),
        draft.candidate
    );
    assert_eq!(reopened.dated_scenario(id, Some(0)).await.unwrap(), initial);
    let after = reopened
        .propose_dated_scenario(
            "next",
            id,
            proposal(
                1,
                vec![
                    DatedCommand::Advance { period: 1 },
                    DatedCommand::Withdraw {
                        resource: 1,
                        quantity: 3,
                    },
                ],
            ),
        )
        .await
        .unwrap();
    reopened
        .resolve_dated_scenario("apply-next", after.request.id, decision(true))
        .await
        .unwrap();
    let state: serde_json::Value = serde_json::from_str(&after.candidate.checkpoint).unwrap();
    assert_eq!(state["origins"]["1"]["expires"], 4);
    assert_eq!(state["origins"]["1"]["withdrawn"], 3);
    assert_eq!(state["origins"]["2"]["released"], false);
    assert_eq!(state["stocks"]["1"]["1"], 1);
    assert_eq!(
        reopened.dated_scenario(id, Some(1)).await.unwrap(),
        draft.candidate
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    let inventory: i64 = sqlx::query_scalar("SELECT count(*) FROM resources")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(inventory, 0);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_reviewed_candidates_have_one_winner_and_no_silent_rebase() {
    let f = Fixture::new().await;
    let id = create(&f).await.definition.id;
    let a = f
        .ledger
        .propose_dated_scenario("a", id, proposal(0, opening()))
        .await
        .unwrap();
    let b = f
        .ledger
        .propose_dated_scenario("b", id, proposal(0, opening()))
        .await
        .unwrap();
    let (ra, rb) = tokio::join!(
        f.ledger
            .resolve_dated_scenario("aa", a.request.id, decision(true)),
        f.ledger
            .resolve_dated_scenario("bb", b.request.id, decision(true))
    );
    assert_eq!(usize::from(ra.is_ok()) + usize::from(rb.is_ok()), 1);
    assert!(matches!(
        if ra.is_err() { &ra } else { &rb },
        Err(LedgerError::ScenarioStale)
    ));
    let loser = if ra.is_err() {
        a.request.id
    } else {
        b.request.id
    };
    assert_eq!(
        f.ledger.dated_scenario_draft(loser).await.unwrap().state,
        "pending"
    );
    assert_eq!(
        f.ledger
            .resolve_dated_scenario("discard-stale", loser, decision(false))
            .await
            .unwrap()
            .state,
        "discarded"
    );
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap().generation,
        1
    );
    assert!(matches!(
        f.ledger
            .propose_dated_scenario("stale-create", id, proposal(0, opening()))
            .await,
        Err(LedgerError::ScenarioStale)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn scenario_idempotency_is_atomic_under_concurrent_retries() {
    let f = Fixture::new().await;
    let def = definition();
    let id = def.id;
    let (a, b) = tokio::join!(
        f.ledger.create_dated_scenario("same", def.clone()),
        f.ledger.create_dated_scenario("same", def.clone())
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let req = proposal(0, opening());
    let (a, b) = tokio::join!(
        f.ledger.propose_dated_scenario("draft", id, req.clone()),
        f.ledger.propose_dated_scenario("draft", id, req.clone())
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let (a, b) = tokio::join!(
        f.ledger
            .resolve_dated_scenario("resolve", req.id, decision(true)),
        f.ledger
            .resolve_dated_scenario("resolve", req.id, decision(true))
    );
    assert_eq!(a.unwrap(), b.unwrap());
    assert!(matches!(
        f.ledger
            .resolve_dated_scenario("resolve", req.id, decision(false))
            .await,
        Err(LedgerError::IdempotencyConflict)
    ));
    assert!(matches!(
        f.ledger
            .resolve_dated_scenario("new-resolve", req.id, decision(true))
            .await,
        Err(LedgerError::ScenarioClosed)
    ));
    assert_eq!(
        f.ledger
            .create_dated_scenario("same", def)
            .await
            .unwrap()
            .generation,
        0
    ); // original response, not current state
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap().generation,
        1
    );
    assert!(matches!(
        f.ledger.create_dated_scenario("draft", definition()).await,
        Err(LedgerError::IdempotencyConflict)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn failed_batch_and_failed_commit_preserve_current_and_retry_key() {
    let f = Fixture::new().await;
    let initial = create(&f).await;
    let id = initial.definition.id;
    let mut cmds = opening();
    cmds.push(DatedCommand::Withdraw {
        resource: 0,
        quantity: 11,
    });
    assert!(matches!(
        f.ledger
            .propose_dated_scenario("retry", id, proposal(0, cmds))
            .await,
        Err(LedgerError::Invalid(_))
    ));
    let draft = f
        .ledger
        .propose_dated_scenario("retry", id, proposal(0, opening()))
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_scenario_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred failure'; END $$; CREATE CONSTRAINT TRIGGER fail_scenario_commit AFTER INSERT ON dated_scenario_outcomes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fail_scenario_commit()").execute(&f.pool).await.unwrap();
    assert!(
        f.ledger
            .resolve_dated_scenario("apply-retry", draft.request.id, decision(true))
            .await
            .is_err()
    );
    assert_eq!(f.ledger.dated_scenario(id, None).await.unwrap(), initial);
    assert_eq!(
        f.ledger
            .dated_scenario_draft(draft.request.id)
            .await
            .unwrap()
            .state,
        "pending"
    );
    sqlx::query("DROP TRIGGER fail_scenario_commit ON dated_scenario_outcomes")
        .execute(&f.pool)
        .await
        .unwrap();
    f.ledger
        .resolve_dated_scenario("apply-retry", draft.request.id, decision(true))
        .await
        .unwrap();
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap(),
        draft.candidate
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn database_rejects_history_changes_forged_xids_and_stale_outcomes() {
    let f = Fixture::new().await;
    let id = create(&f).await.definition.id;
    let a = f
        .ledger
        .propose_dated_scenario("a", id, proposal(0, opening()))
        .await
        .unwrap();
    let b = f
        .ledger
        .propose_dated_scenario("b", id, proposal(0, opening()))
        .await
        .unwrap();
    for table in [
        "dated_scenarios",
        "dated_scenario_drafts",
        "dated_scenario_outcomes",
    ] {
        for sql in [
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table} CASCADE"),
            format!("UPDATE {table} SET created_at=created_at"),
        ] {
            assert!(sqlx::query(&sql).execute(&f.pool).await.is_err());
        }
    }
    let mut tx = f.pool.begin().await.unwrap();
    let d = Uuid::new_v4();
    sqlx::query("INSERT INTO dated_scenario_drafts SELECT $1,scenario_id,base_generation,request,candidate,flow_revision,pg_current_xact_id(),clock_timestamp() FROM dated_scenario_drafts WHERE id=$2").bind(d).bind(a.request.id).execute(&mut *tx).await.unwrap();
    assert!(sqlx::query("INSERT INTO dated_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,1,'applied','same transaction')").bind(d).bind(id).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    assert!(sqlx::query("INSERT INTO dated_scenario_drafts SELECT $1,scenario_id,base_generation,request,candidate,flow_revision,'1'::xid8,clock_timestamp() FROM dated_scenario_drafts WHERE id=$2").bind(d).bind(a.request.id).execute(&f.pool).await.is_err());
    f.ledger
        .resolve_dated_scenario("apply", a.request.id, decision(true))
        .await
        .unwrap();
    assert!(sqlx::query("INSERT INTO dated_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,2,'applied','stale')").bind(b.request.id).bind(id).execute(&f.pool).await.is_err());
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap().generation,
        1
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn cross_scope_identity_invalid_definitions_and_oversized_batches_are_rejected() {
    let f = Fixture::new().await;
    let id = create(&f).await.definition.id;
    for mutation in 0..4 {
        let mut def = definition();
        match mutation {
            0 => def.resources.push("store".into()),
            1 => def.format_version = 2,
            2 => def.unit.clear(),
            _ => def.resources = vec![],
        };
        assert!(
            f.ledger
                .create_dated_scenario("invalid", def)
                .await
                .is_err()
        );
    }
    assert!(
        f.ledger
            .propose_dated_scenario("empty", id, proposal(0, vec![]))
            .await
            .is_err()
    );
    assert!(
        f.ledger
            .propose_dated_scenario(
                "huge",
                id,
                proposal(
                    0,
                    (0..65)
                        .map(|period| DatedCommand::Advance { period })
                        .collect()
                )
            )
            .await
            .is_err()
    );
    let a = f
        .ledger
        .propose_dated_scenario("a", id, proposal(0, opening()))
        .await
        .unwrap();
    let other = f
        .ledger
        .create_dated_scenario("other", definition())
        .await
        .unwrap();
    assert!(sqlx::query("INSERT INTO dated_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,1,'applied','wrong scope')").bind(a.request.id).bind(other.definition.id).execute(&f.pool).await.is_err());
    assert_eq!(
        f.ledger
            .dated_scenario(other.definition.id, None)
            .await
            .unwrap(),
        other
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn checkpoint_capacity_failure_does_not_consume_identity_or_generation() {
    let f = Fixture::new().await;
    let id = create(&f).await.definition.id;
    let mut gen_no = 0;
    for block in 0..4u64 {
        let mut commands = if block == 0 {
            vec![DatedCommand::Advance { period: 0 }]
        } else {
            vec![]
        };
        for i in 0..60 {
            commands.push(DatedCommand::Introduce {
                id: block * 60 + i,
                resource: 0,
                quantity: 1,
                available: 0,
                expires: 10,
                basis: "x".repeat(2048),
            });
        }
        let result = f
            .ledger
            .propose_dated_scenario(&format!("batch-{block}"), id, proposal(gen_no, commands))
            .await;
        match result {
            Ok(d) => {
                f.ledger
                    .resolve_dated_scenario(&format!("apply-{block}"), d.request.id, decision(true))
                    .await
                    .unwrap();
                gen_no += 1;
            }
            Err(LedgerError::Invalid(_)) => {
                assert!(block > 0);
                let head = f.ledger.dated_scenario(id, None).await.unwrap();
                assert_eq!(head.generation, gen_no);
                let d = f
                    .ledger
                    .propose_dated_scenario(
                        &format!("batch-{block}"),
                        id,
                        proposal(
                            gen_no,
                            vec![DatedCommand::Introduce {
                                id: block * 60,
                                resource: 0,
                                quantity: 1,
                                available: 0,
                                expires: 10,
                                basis: "retry after rejected large batch".into(),
                            }],
                        ),
                    )
                    .await
                    .unwrap();
                assert_eq!(d.candidate.generation, gen_no + 1);
                return;
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
    panic!("checkpoint limit was not enforced");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn scenario_http_denies_ai_and_raw_snapshot_and_enforces_body_limit() {
    let f = Fixture::new().await;
    let def = definition();
    let id = def.id;
    let operator = "scenario-operator-token-32-characters";
    let reviewer = "scenario-reviewer-token-32-characters";
    let observer = "scenario-observer-token-32-characters";
    let app = api::router_with_access(
        f.ledger.clone(),
        api::AccessTokens::new(
            operator.into(),
            Some(observer.into()),
            Some(reviewer.into()),
        )
        .unwrap(),
    );
    let body = json!(def).to_string();
    for (token, payload, expected) in [
        (reviewer, body.clone(), StatusCode::FORBIDDEN),
        (observer, body.clone(), StatusCode::FORBIDDEN),
        ("invalid", body.clone(), StatusCode::UNAUTHORIZED),
        (
            operator,
            format!("{body}{}", " ".repeat(16385 - body.len())),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
        (
            operator,
            format!("{body}{}", " ".repeat(16384 - body.len())),
            StatusCode::OK,
        ),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/scenarios/dated")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", "http-create")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
    }
    let req = proposal(0, opening());
    let d = f
        .ledger
        .propose_dated_scenario("draft", id, req.clone())
        .await
        .unwrap();
    for (path, payload, token, status) in [
        (
            format!("/v1/scenario-drafts/{}/resolve", req.id),
            json!(decision(true)),
            reviewer,
            StatusCode::FORBIDDEN,
        ),
        (
            format!("/v1/scenarios/dated/{id}/drafts"),
            json!({"id":Uuid::new_v4(),"expected_generation":0,"commands":opening(),"basis":"raw injection","checkpoint":d.candidate.checkpoint}),
            operator,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", "http-deny")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), status);
    }
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap().generation,
        0
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn structurally_conserved_but_forged_candidate_cannot_replace_reviewed_commands() {
    let f = Fixture::new().await;
    let initial = create(&f).await;
    let id = initial.definition.id;
    let request = proposal(0, opening());
    // Simulate an inconsistent stored proposal: its mass balance is valid, but
    // it introduces 11 instead of the 10 units in the saved command history.
    let mut forged = qazna::dated::DatedFlow::new(&id.to_string(), "grain_kg", 2).unwrap();
    forged.advance(0).unwrap();
    forged
        .introduce(1, 0, 11, 0, 4, "Synthetic opening lot")
        .unwrap();
    sqlx::query("INSERT INTO dated_scenario_drafts(id,scenario_id,base_generation,request,candidate,flow_revision) VALUES ($1,$2,0,$3,$4,2)")
        .bind(request.id).bind(id).bind(json!(request)).bind(forged.checkpoint().unwrap()).execute(&f.pool).await.unwrap();
    assert!(matches!(
        f.ledger
            .resolve_dated_scenario("forged", request.id, decision(true))
            .await,
        Err(LedgerError::InvalidStoredResponse)
    ));
    assert_eq!(f.ledger.dated_scenario(id, None).await.unwrap(), initial);
    let mut foreign = definition();
    foreign.unit = "energy_kwh".into();
    sqlx::query("INSERT INTO dated_scenarios(id,definition,initial) VALUES ($1,$2,$3)")
        .bind(foreign.id)
        .bind(json!(foreign))
        .bind(initial.checkpoint.as_bytes())
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(matches!(
        f.ledger.dated_scenario(foreign.id, None).await,
        Err(LedgerError::InvalidStoredResponse)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn restored_historical_xid_collision_does_not_block_committed_draft() {
    let f = Fixture::new().await;
    let id = create(&f).await.definition.id;
    let original = f
        .ledger
        .propose_dated_scenario("original", id, proposal(0, opening()))
        .await
        .unwrap();
    // Reserve a decision transaction ID, then emulate restoring an older draft
    // bearing that same historical stamp in a separately committed transaction.
    let mut decision_tx = f.pool.begin().await.unwrap();
    let stamp: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut *decision_tx)
        .await
        .unwrap();
    let mut restore_tx = f.pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE dated_scenario_drafts DISABLE TRIGGER current_transaction_xid")
        .execute(&mut *restore_tx)
        .await
        .unwrap();
    let restored_id = Uuid::new_v4();
    let mut request = original.request;
    request.id = restored_id;
    sqlx::query("INSERT INTO dated_scenario_drafts(id,scenario_id,base_generation,request,candidate,flow_revision,created_xid) VALUES ($1,$2,0,$3,$4,2,$5::text::xid8)")
        .bind(restored_id).bind(id).bind(json!(request)).bind(original.candidate.checkpoint.as_bytes()).bind(&stamp)
        .execute(&mut *restore_tx).await.unwrap();
    sqlx::query("ALTER TABLE dated_scenario_drafts ENABLE TRIGGER current_transaction_xid")
        .execute(&mut *restore_tx)
        .await
        .unwrap();
    restore_tx.commit().await.unwrap();
    sqlx::query("INSERT INTO dated_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,1,'applied','Review after restored XID collision')")
        .bind(restored_id).bind(id).execute(&mut *decision_tx).await.unwrap();
    decision_tx.commit().await.unwrap();
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap(),
        original.candidate
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn savepoint_creation_cannot_bypass_separate_committed_review() {
    let f = Fixture::new().await;
    let id = create(&f).await.definition.id;
    let original = f
        .ledger
        .propose_dated_scenario("template", id, proposal(0, opening()))
        .await
        .unwrap();
    for released in [false, true] {
        let mut tx = f.pool.begin().await.unwrap();
        sqlx::query("SAVEPOINT draft_creation")
            .execute(&mut *tx)
            .await
            .unwrap();
        let id2 = Uuid::new_v4();
        sqlx::query("INSERT INTO dated_scenario_drafts SELECT $1,scenario_id,base_generation,request,candidate,flow_revision,pg_current_xact_id(),clock_timestamp() FROM dated_scenario_drafts WHERE id=$2")
            .bind(id2).bind(original.request.id).execute(&mut *tx).await.unwrap();
        if released {
            sqlx::query("RELEASE SAVEPOINT draft_creation")
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        assert!(sqlx::query("INSERT INTO dated_scenario_outcomes(draft_id,scenario_id,generation,state,basis) VALUES ($1,$2,1,'applied','savepoint bypass')")
            .bind(id2).bind(id).execute(&mut *tx).await.is_err());
        tx.rollback().await.unwrap();
    }
    assert_eq!(
        f.ledger.dated_scenario(id, None).await.unwrap().generation,
        0
    );
}
