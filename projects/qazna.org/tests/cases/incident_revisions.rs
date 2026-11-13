use super::*;
use qazna::incident_revisions::*;

fn split(mut d: IncidentDeclaration) -> (IncidentDeclaration, IncidentDeclaration) {
    let mut b = d.clone();
    b.id = Uuid::new_v4();
    d.revision.changes.truncate(2);
    b.revision.changes.drain(..2);
    (d, b)
}
fn combination(ids: Vec<Uuid>) -> IncidentCombination {
    IncidentCombination {
        id: Uuid::new_v4(),
        expected_generation: 1,
        incidents: ids,
        basis: "Combined exact observations".into(),
    }
}
fn correction(target: Option<Uuid>, generation: i64) -> IncidentCorrection {
    IncidentCorrection {
        expected_generation: generation,
        replacement_id: target,
        basis: "Operator corrected source evidence".into(),
    }
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn combined_incidents_keep_lineage_and_use_existing_reviewed_plan_path() {
    let f = Fixture::new().await;
    let (m, d, mut input) = baseline(&f).await;
    let (a, b) = split(d);
    let old_a = f
        .ledger
        .declare_service_incident("a", m, a.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("b", m, b.clone())
        .await
        .unwrap();
    let request = combination(vec![b.id, a.id]);
    let combined = f
        .ledger
        .combine_service_incidents("merge", m, request.clone())
        .await
        .unwrap();
    let mut expected = vec![a.id, b.id];
    expected.sort();
    assert_eq!(combined.supersedes, expected);
    assert_eq!(combined.state, "open");
    for id in [a.id, b.id] {
        let record = f.ledger.service_incident(id).await.unwrap();
        assert_eq!(record.state, "superseded");
        assert_eq!(
            record.disposition.unwrap().replacement_id,
            Some(combined.id)
        );
    }
    assert_eq!(
        json!(f.ledger.declare_service_incident("a", m, a).await.unwrap()),
        json!(old_a)
    );
    assert_eq!(
        json!(
            f.ledger
                .combine_service_incidents("merge", m, request)
                .await
                .unwrap()
        ),
        json!(combined)
    );
    input.services.as_mut().unwrap().revisions[0] = combined.revision.clone();
    repair(&f, m, input).await;
    let cash = coupled(&f, m).await;
    money(&f, cash, 0, 2, vec![]).await;
    let link = IncidentPlan {
        model_generation: 2,
        coupled_id: cash,
        coupled_generation: 1,
        basis: "Joint physical and monetary review".into(),
    };
    assert!(matches!(
        f.ledger
            .link_service_incident_plan("old-link", old_a.id, link.clone())
            .await,
        Err(LedgerError::ScenarioClosed)
    ));
    let linked = f
        .ledger
        .link_service_incident_plan("new-link", combined.id, link)
        .await
        .unwrap();
    assert_eq!(linked.state, "plan_linked");
    assert_eq!(linked.supersedes, expected);
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn conflicting_observation_requires_explicit_correction_before_union() {
    let f = Fixture::new().await;
    let (m, d, _) = baseline(&f).await;
    let (a, mut b) = split(d);
    b.revision = a.revision.clone();
    b.revision.changes[0].available_bps -= 1;
    f.ledger
        .declare_service_incident("a", m, a.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("b", m, b.clone())
        .await
        .unwrap();
    let request = combination(vec![a.id, b.id]);
    assert!(
        f.ledger
            .combine_service_incidents("retry", m, request.clone())
            .await
            .is_err()
    );
    assert!(f.ledger.service_incident(request.id).await.is_err());
    assert_eq!(f.ledger.service_incident(a.id).await.unwrap().state, "open");
    assert_eq!(f.ledger.service_incident(b.id).await.unwrap().state, "open");
    let mut good = a.clone();
    good.id = Uuid::new_v4();
    f.ledger
        .declare_service_incident("good", m, good.clone())
        .await
        .unwrap();
    let fixed = f
        .ledger
        .correct_service_incident("correct", b.id, correction(Some(good.id), 1))
        .await
        .unwrap();
    assert_eq!(fixed.state, "superseded");
    assert_eq!(fixed.revision, b.revision);
    let mut next = request;
    next.incidents = vec![a.id, good.id];
    let combined = f
        .ledger
        .combine_service_incidents("retry", m, next)
        .await
        .unwrap();
    assert_eq!(combined.revision.changes, a.revision.changes);
    assert!(
        f.ledger
            .correct_service_incident("again", b.id, correction(None, 1))
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger
            .service_incident(good.id)
            .await
            .unwrap()
            .disposition
            .unwrap()
            .replacement_id,
        Some(combined.id)
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn competing_combinations_commit_one_complete_lineage_and_allow_replay() {
    let f = Fixture::new().await;
    let (m, d, _) = baseline(&f).await;
    let (a, b) = split(d);
    f.ledger
        .declare_service_incident("a", m, a.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("b", m, b.clone())
        .await
        .unwrap();
    for ids in [
        vec![],
        vec![a.id],
        vec![a.id, a.id],
        vec![a.id; 17],
        vec![a.id, Uuid::new_v4()],
    ] {
        assert!(
            f.ledger
                .combine_service_incidents("invalid", m, combination(ids))
                .await
                .is_err()
        );
    }
    let x = combination(vec![a.id, b.id]);
    let y = combination(vec![a.id, b.id]);
    let (one, two) = tokio::join!(
        f.ledger.combine_service_incidents("x", m, x.clone()),
        f.ledger.combine_service_incidents("y", m, y.clone())
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let (winner, loser, key, request) = match (one, two) {
        (Ok(w), Err(_)) => (w, y.id, "x", x),
        (Err(_), Ok(w)) => (w, x.id, "y", y),
        _ => unreachable!(),
    };
    assert!(f.ledger.service_incident(loser).await.is_err());
    assert_eq!(winner.supersedes.len(), 2);
    assert_eq!(
        json!(
            f.ledger
                .combine_service_incidents(key, m, request)
                .await
                .unwrap()
        ),
        json!(winner)
    );
    assert_eq!(
        f.ledger
            .service_incidents(m, IncidentQuery::default())
            .await
            .unwrap()
            .len(),
        3
    );
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn correcting_linked_incident_keeps_plan_and_never_rewinds_physical_history() {
    let f = Fixture::new().await;
    let (m, d, input) = baseline(&f).await;
    f.ledger
        .declare_service_incident("old", m, d.clone())
        .await
        .unwrap();
    repair(&f, m, input.clone()).await;
    let cash = coupled(&f, m).await;
    money(&f, cash, 0, 2, vec![]).await;
    let plan = IncidentPlan {
        model_generation: 2,
        coupled_id: cash,
        coupled_generation: 1,
        basis: "Original reviewed repair".into(),
    };
    let linked = f
        .ledger
        .link_service_incident_plan("linked", d.id, plan)
        .await
        .unwrap();
    let saved = f.ledger.model_scenario(m, None).await.unwrap();
    let mut recovery = d.clone();
    recovery.id = Uuid::new_v4();
    recovery.expected_generation = 2;
    for c in &mut recovery.revision.changes {
        std::mem::swap(&mut c.available_bps, &mut c.previous_bps);
    }
    recovery.revision.source_reference = "test:correction".into();
    f.ledger
        .declare_service_incident("recovery", m, recovery.clone())
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .correct_service_incident("fix", d.id, correction(Some(recovery.id), 1))
            .await,
        Err(LedgerError::ScenarioStale)
    ));
    let fixed = f
        .ledger
        .correct_service_incident("fix", d.id, correction(Some(recovery.id), 2))
        .await
        .unwrap();
    assert_eq!(fixed.state, "superseded");
    assert_eq!(json!(fixed.plan), json!(linked.plan));
    assert_eq!(
        json!(f.ledger.model_scenario(m, None).await.unwrap()),
        json!(saved)
    );
    assert!(
        f.ledger
            .correct_service_incident("cycle", recovery.id, correction(Some(d.id), 2))
            .await
            .is_err()
    );
    let withdrawn = f
        .ledger
        .correct_service_incident("withdraw", recovery.id, correction(None, 2))
        .await
        .unwrap();
    assert_eq!(withdrawn.state, "withdrawn");
    assert_eq!(
        json!(
            f.ledger
                .correct_service_incident("withdraw", recovery.id, correction(None, 2))
                .await
                .unwrap()
        ),
        json!(withdrawn)
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn sql_rejects_partial_union_cycles_and_history_mutation() {
    let f = Fixture::new().await;
    let (m, d, _) = baseline(&f).await;
    let (a, b) = split(d);
    f.ledger
        .declare_service_incident("a", m, a.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("b", m, b.clone())
        .await
        .unwrap();
    let id = Uuid::new_v4();
    let mut revision = a.revision.clone();
    revision.changes.extend(b.revision.changes.clone());
    revision.source_reference = format!("qazna:incident-combination:{id}");
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO service_incidents(id,model_id,base_generation,revision) VALUES($1,$2,1,$3)",
    )
    .bind(id)
    .bind(m)
    .bind(json!(revision))
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO service_incident_supersessions(incident_id,model_id,model_generation,replacement_id,kind,basis) VALUES($1,$2,1,$3,'merged','partial')").bind(a.id).bind(m).bind(id).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    assert!(f.ledger.service_incident(id).await.is_err());
    assert_eq!(f.ledger.service_incident(a.id).await.unwrap().state, "open");
    assert!(sqlx::query("INSERT INTO service_incident_supersessions(incident_id,model_id,model_generation,replacement_id,kind,basis) VALUES($1,$2,1,$3,'replaced','reverse edge')").bind(b.id).bind(m).bind(a.id).execute(&f.pool).await.is_err());
    let mut extra = a.clone();
    extra.id = Uuid::new_v4();
    f.ledger
        .declare_service_incident("extra", m, extra.clone())
        .await
        .unwrap();
    let combined = f
        .ledger
        .combine_service_incidents("valid", m, combination(vec![a.id, b.id]))
        .await
        .unwrap();
    // An older unused duplicate cannot be appended to an already committed union.
    assert!(sqlx::query("INSERT INTO service_incident_supersessions(incident_id,model_id,model_generation,replacement_id,kind,basis) VALUES($1,$2,1,$3,'merged','late ancestry')").bind(extra.id).bind(m).bind(combined.id).execute(&f.pool).await.is_err());
    for statement in [
        "UPDATE service_incident_supersessions SET basis=basis",
        "DELETE FROM service_incident_supersessions",
        "TRUNCATE service_incident_supersessions",
    ] {
        assert!(sqlx::query(statement).execute(&f.pool).await.is_err());
    }
    assert_eq!(combined.supersedes.len(), 2);
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn correction_and_combination_http_are_operator_only_and_reject_unknown_actions() {
    let f = Fixture::new().await;
    let (m, d, _) = baseline(&f).await;
    f.ledger
        .declare_service_incident("old", m, d.clone())
        .await
        .unwrap();
    let tokens = api::AccessTokens::new("o".repeat(32), Some("v".repeat(32)), Some("r".repeat(32)))
        .unwrap()
        .with_attestors(vec![api::AttestorCredential {
            principal: Uuid::new_v4(),
            token: "a".repeat(32),
        }])
        .unwrap();
    let app = api::router_with_access(f.ledger.clone(), tokens);
    for token in ["v", "r", "a"] {
        for path in [
            format!("/v1/service-incidents/{}/correct", d.id),
            format!("/v1/scenarios/models/{m}/incidents/combine"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::post(path)
                        .header("authorization", format!("Bearer {}", token.repeat(32)))
                        .header("content-type", "application/json")
                        .header("idempotency-key", uid())
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
    let mut bad = json!(correction(None, 1));
    bad["apply_plan"] = json!(true);
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/v1/service-incidents/{}/correct", d.id))
                .header("authorization", format!("Bearer {}", "o".repeat(32)))
                .header("content-type", "application/json")
                .header("idempotency-key", "http")
                .body(Body::from(bad.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .oneshot(
            Request::post(format!("/v1/service-incidents/{}/correct", d.id))
                .header("authorization", format!("Bearer {}", "o".repeat(32)))
                .header("content-type", "application/json")
                .header("idempotency-key", "http")
                .body(Body::from(json!(correction(None, 1)).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn combination_rejects_foreign_stale_and_linked_sources_without_partial_rows() {
    let f = Fixture::new().await;
    let (m, a, input) = baseline(&f).await;
    let (other, b, _) = baseline(&f).await;
    f.ledger
        .declare_service_incident("a", m, a.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("b", other, b.clone())
        .await
        .unwrap();
    assert!(
        f.ledger
            .correct_service_incident("cross-correction", a.id, correction(Some(b.id), 1))
            .await
            .is_err()
    );
    let foreign = combination(vec![a.id, b.id]);
    assert!(
        f.ledger
            .combine_service_incidents("cross", m, foreign.clone())
            .await
            .is_err()
    );
    assert!(f.ledger.service_incident(foreign.id).await.is_err());
    let mut c = a.clone();
    c.id = Uuid::new_v4();
    f.ledger
        .declare_service_incident("c", m, c.clone())
        .await
        .unwrap();
    let request = combination(vec![a.id, c.id]);
    repair(&f, m, input).await;
    assert!(matches!(
        f.ledger
            .combine_service_incidents("stale", m, request.clone())
            .await,
        Err(LedgerError::ScenarioStale)
    ));
    let mut current = request.clone();
    current.expected_generation = 2;
    assert!(
        f.ledger
            .combine_service_incidents("old-base", m, current.clone())
            .await
            .is_err()
    );
    let cash = coupled(&f, m).await;
    money(&f, cash, 0, 2, vec![]).await;
    f.ledger
        .link_service_incident_plan(
            "plan",
            a.id,
            IncidentPlan {
                model_generation: 2,
                coupled_id: cash,
                coupled_generation: 1,
                basis: "Reviewed".into(),
            },
        )
        .await
        .unwrap();
    assert!(
        f.ledger
            .combine_service_incidents("linked", m, current)
            .await
            .is_err()
    );
    assert!(f.ledger.service_incident(request.id).await.is_err());
    assert_eq!(f.ledger.service_incident(c.id).await.unwrap().state, "open");
    assert_eq!(
        f.ledger.service_incident(a.id).await.unwrap().state,
        "plan_linked"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn withdrawal_racing_combination_has_one_atomic_outcome() {
    let f = Fixture::new().await;
    let (m, d, _) = baseline(&f).await;
    let (a, b) = split(d);
    f.ledger
        .declare_service_incident("a", m, a.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("b", m, b.clone())
        .await
        .unwrap();
    let request = combination(vec![a.id, b.id]);
    let (withdraw, merge) = tokio::join!(
        f.ledger
            .correct_service_incident("withdraw", a.id, correction(None, 1)),
        f.ledger
            .combine_service_incidents("merge", m, request.clone())
    );
    assert_eq!(
        usize::from(withdraw.is_ok()) + usize::from(merge.is_ok()),
        1
    );
    let first = f.ledger.service_incident(a.id).await.unwrap();
    let second = f.ledger.service_incident(b.id).await.unwrap();
    if merge.is_ok() {
        assert_eq!(first.state, "superseded");
        assert_eq!(second.state, "superseded");
        assert_eq!(
            f.ledger
                .service_incident(request.id)
                .await
                .unwrap()
                .supersedes
                .len(),
            2
        );
    } else {
        assert_eq!(first.state, "withdrawn");
        assert_eq!(second.state, "open");
        assert!(f.ledger.service_incident(request.id).await.is_err());
    }
    assert_eq!(
        f.ledger.model_scenario(m, None).await.unwrap().generation,
        1
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
}
