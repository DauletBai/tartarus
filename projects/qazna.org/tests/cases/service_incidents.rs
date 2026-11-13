use super::*;
use qazna::{coupled::*, model_scenarios::*, scenarios::ScenarioDecision, service_incidents::*};
use serde_json::{Value, json};
fn report() -> Value {
    serde_json::from_str(include_str!("../../research/service-results.json")).unwrap()
}
fn decision() -> ScenarioDecision {
    ScenarioDecision {
        apply: true,
        basis: "Reviewed service plan".into(),
    }
}
fn proposal(input: ModelInput, generation: i64) -> ModelProposal {
    ModelProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        input,
        basis: "Service continuation".into(),
    }
}
async fn baseline(f: &Fixture) -> (Uuid, IncidentDeclaration, ModelInput) {
    let v = report();
    let mut d: ModelDefinition =
        serde_json::from_value(v["runtime_fixture"]["definition"].clone()).unwrap();
    d.id = Uuid::new_v4();
    let id = d.id;
    f.ledger.create_model_scenario(&uid(), d).await.unwrap();
    let input: ModelInput =
        serde_json::from_value(v["feedback"]["baseline_input"].clone()).unwrap();
    let draft = f
        .ledger
        .propose_model_scenario(&uid(), id, proposal(input, 0))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario(&uid(), draft.request.id, decision())
        .await
        .unwrap();
    let repaired: ModelInput =
        serde_json::from_value(v["feedback"]["repair"]["input"].clone()).unwrap();
    let revision = repaired.services.as_ref().unwrap().revisions[0].clone();
    (
        id,
        IncidentDeclaration {
            id: Uuid::new_v4(),
            expected_generation: 1,
            revision,
        },
        repaired,
    )
}
fn uid() -> String {
    Uuid::new_v4().to_string()
}
async fn coupled(f: &Fixture, model: Uuid) -> Uuid {
    let d = CoupledDefinition {
        id: Uuid::new_v4(),
        format_version: 1,
        model_id: model,
        policy: serde_json::from_value(report()["feedback"]["circulation"]["policy"].clone())
            .unwrap(),
        basis: "Reviewed common opening vector".into(),
    };
    let id = d.id;
    f.ledger.create_coupled_scenario(&uid(), d).await.unwrap();
    id
}
async fn money(f: &Fixture, id: Uuid, generation: i64, physical: i64, unavailable: Vec<usize>) {
    let d = f
        .ledger
        .propose_coupled_scenario(
            &uid(),
            id,
            CoupledProposal {
                id: Uuid::new_v4(),
                expected_generation: generation,
                model_generation: physical,
                unavailable_events: unavailable,
                basis: "Recalculate from approved physical plan".into(),
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_coupled_scenario(&uid(), d.request.id, decision())
        .await
        .unwrap();
}
async fn repair(f: &Fixture, id: Uuid, input: ModelInput) {
    let d = f
        .ledger
        .propose_model_scenario(&uid(), id, proposal(input, 1))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario(&uid(), d.request.id, decision())
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn unrepairable_incident_is_durable_idempotent_and_paginated() {
    let f = Fixture::new().await;
    let (m, mut d, mut input) = baseline(&f).await;
    d.revision.changes[0].available_bps = 0;
    input.services.as_mut().unwrap().pools[0].availability_bps[4] = 0;
    input.services.as_mut().unwrap().revisions[0] = d.revision.clone();
    let (a, b) = tokio::join!(
        f.ledger.declare_service_incident("outage", m, d.clone()),
        f.ledger.declare_service_incident("outage", m, d.clone())
    );
    let a = a.unwrap();
    assert_eq!(json!(a), json!(b.unwrap()));
    assert_eq!(a.state, "open");
    assert!(
        f.ledger
            .propose_model_scenario("impossible", m, proposal(input, 1))
            .await
            .is_err()
    );
    assert_eq!(
        json!(f.ledger.service_incident(d.id).await.unwrap()),
        json!(a)
    );
    let mut other = d.clone();
    other.id = Uuid::new_v4();
    other.revision.source_reference = "second declaration".into();
    assert!(matches!(
        f.ledger
            .declare_service_incident("outage", m, other.clone())
            .await,
        Err(LedgerError::IdempotencyConflict)
    ));
    let b = f
        .ledger
        .declare_service_incident("other", m, other)
        .await
        .unwrap();
    let page = f
        .ledger
        .service_incidents(
            m,
            IncidentQuery {
                after: None,
                limit: Some(1),
            },
        )
        .await
        .unwrap();
    assert_eq!(page[0].id, a.id);
    let page = f
        .ledger
        .service_incidents(
            m,
            IncidentQuery {
                after: Some(a.sequence),
                limit: Some(1),
            },
        )
        .await
        .unwrap();
    assert_eq!(page[0].id, b.id);
    assert!(
        f.ledger
            .service_incidents(
                m,
                IncidentQuery {
                    after: Some(b.sequence),
                    limit: None
                }
            )
            .await
            .unwrap()
            .is_empty()
    );
    for q in [
        IncidentQuery {
            after: Some(-1),
            limit: None,
        },
        IncidentQuery {
            after: None,
            limit: Some(101),
        },
        IncidentQuery {
            after: None,
            limit: Some(0),
        },
    ] {
        assert!(f.ledger.service_incidents(m, q).await.is_err());
    }
    assert!(
        f.ledger
            .service_incidents(Uuid::new_v4(), IncidentQuery::default())
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger.model_scenario(m, None).await.unwrap().generation,
        1
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn declaration_rejects_forged_changes_and_stale_base_without_consuming_key() {
    let f = Fixture::new().await;
    let (m, d, input) = baseline(&f).await;
    let mut cases = vec![];
    let mut x = d.clone();
    x.expected_generation = 0;
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes[0].previous_bps = 1;
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes.swap(0, 1);
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes.push(x.revision.changes[0].clone());
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes[0].pool = "missing".into();
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes[0].period = usize::MAX;
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes[0].available_bps = 10001;
    cases.push(x);
    let mut x = d.clone();
    x.revision.changes.clear();
    cases.push(x);
    let mut x = d.clone();
    x.revision.boundary = 5;
    cases.push(x);
    let mut x = d.clone();
    x.revision.source_reference.clear();
    cases.push(x);
    for bad in cases {
        assert!(
            f.ledger
                .declare_service_incident("retry", m, bad)
                .await
                .is_err()
        );
    }
    assert!(
        f.ledger
            .service_incidents(m, IncidentQuery::default())
            .await
            .unwrap()
            .is_empty()
    );
    f.ledger
        .declare_service_incident("retry", m, d.clone())
        .await
        .unwrap();
    repair(&f, m, input).await;
    let mut stale = d;
    stale.id = Uuid::new_v4();
    assert!(matches!(
        f.ledger.declare_service_incident("stale", m, stale).await,
        Err(LedgerError::ScenarioStale)
    ));
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn incident_link_requires_matching_reviewed_physical_and_settled_money() {
    let f = Fixture::new().await;
    let (m, d, input) = baseline(&f).await;
    let record = f
        .ledger
        .declare_service_incident("incident", m, d.clone())
        .await
        .unwrap();
    let mut unrelated = d.clone();
    unrelated.id = Uuid::new_v4();
    unrelated.revision.changes[0].available_bps = 0;
    f.ledger
        .declare_service_incident("unrelated", m, unrelated.clone())
        .await
        .unwrap();
    let c = coupled(&f, m).await;
    money(&f, c, 0, 1, vec![]).await;
    let request = IncidentPlan {
        model_generation: 2,
        coupled_id: c,
        coupled_generation: 2,
        basis: "Match reviewed repair and circulation".into(),
    };
    let pending = f
        .ledger
        .propose_model_scenario("pending", m, proposal(input.clone(), 1))
        .await
        .unwrap();
    assert!(
        f.ledger
            .link_service_incident_plan("link", d.id, request.clone())
            .await
            .is_err()
    );
    f.ledger
        .resolve_model_scenario("repair", pending.request.id, decision())
        .await
        .unwrap();
    assert!(
        f.ledger
            .link_service_incident_plan("link", d.id, request.clone())
            .await
            .is_err()
    );
    let blocked = report()["feedback"]["circulation"]["blocked_event"]
        .as_u64()
        .unwrap() as usize;
    money(&f, c, 1, 2, vec![blocked]).await;
    assert!(
        f.ledger
            .link_service_incident_plan("link", d.id, request.clone())
            .await
            .is_err()
    );
    assert_eq!(
        json!(f.ledger.service_incident(d.id).await.unwrap()),
        json!(record)
    );
    money(&f, c, 2, 2, vec![]).await;
    assert!(matches!(
        f.ledger
            .link_service_incident_plan("link", d.id, request.clone())
            .await,
        Err(LedgerError::ScenarioStale)
    ));
    let mut request = request;
    request.coupled_generation = 3;
    assert!(
        f.ledger
            .link_service_incident_plan("wrong-incident", unrelated.id, request.clone())
            .await
            .is_err()
    );
    // Direct SQL also rejects linking a different declaration to these approvals.
    assert!(sqlx::query("INSERT INTO service_incident_plans(incident_id,model_id,model_generation,coupled_id,coupled_generation,basis) VALUES($1,$2,2,$3,3,'wrong declaration')").bind(unrelated.id).bind(m).bind(c).execute(&f.pool).await.is_err());
    let (a, b) = tokio::join!(
        f.ledger
            .link_service_incident_plan("link", d.id, request.clone()),
        f.ledger
            .link_service_incident_plan("link", d.id, request.clone())
    );
    let a = a.unwrap();
    assert_eq!(json!(a), json!(b.unwrap()));
    assert_eq!(a.state, "plan_linked");
    assert!(matches!(
        f.ledger
            .link_service_incident_plan("another-key", d.id, request)
            .await,
        Err(LedgerError::ScenarioClosed)
    ));
    for table in ["service_incidents", "service_incident_plans"] {
        for statement in [
            format!("UPDATE {table} SET model_id=model_id"),
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table} CASCADE"),
        ] {
            assert!(sqlx::query(&statement).execute(&f.pool).await.is_err());
        }
    }
    let mut next = input;
    next.boundary = 6;
    let p = f
        .ledger
        .propose_model_scenario("next", m, proposal(next, 2))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("next-apply", p.request.id, decision())
        .await
        .unwrap();
    assert_eq!(
        json!(f.ledger.service_incident(d.id).await.unwrap()),
        json!(a)
    );
    assert_eq!(
        f.ledger.service_incident(unrelated.id).await.unwrap().state,
        "open"
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn incident_http_limits_and_role_separation() {
    let f = Fixture::new().await;
    let (m, d, _) = baseline(&f).await;
    let tokens = api::AccessTokens::new("o".repeat(32), Some("v".repeat(32)), Some("r".repeat(32)))
        .unwrap()
        .with_attestors(vec![api::AttestorCredential {
            principal: Uuid::new_v4(),
            token: "a".repeat(32),
        }])
        .unwrap();
    let app = api::router_with_access(f.ledger.clone(), tokens);
    let path = format!("/v1/scenarios/models/{m}/incidents");
    for token in ["v", "r", "a"] {
        for (method, path, body) in [
            ("GET", path.clone(), String::new()),
            ("POST", path.clone(), json!(d).to_string()),
            (
                "GET",
                format!("/v1/service-incidents/{}", d.id),
                String::new(),
            ),
            (
                "POST",
                format!("/v1/service-incidents/{}/plan", d.id),
                "{}".into(),
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header("authorization", format!("Bearer {}", token.repeat(32)))
                        .header("content-type", "application/json")
                        .header("idempotency-key", uid())
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
    for (body, status) in [
        (
            format!("{}{}", json!(d), " ".repeat(16384)),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
        (json!(d).to_string(), StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(&path)
                    .header("authorization", format!("Bearer {}", "o".repeat(32)))
                    .header("content-type", "application/json")
                    .header("idempotency-key", "http-declaration")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    for suffix in ["?limit=101", "?after=-1", "?extra=1"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("{path}{suffix}"))
                    .header("authorization", format!("Bearer {}", "o".repeat(32)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn competing_links_and_cross_scope_references_cannot_replace_history() {
    let f = Fixture::new().await;
    let (model, declaration, input) = baseline(&f).await;
    let (other_model, other_declaration, other_input) = baseline(&f).await;
    f.ledger
        .declare_service_incident("one", model, declaration.clone())
        .await
        .unwrap();
    f.ledger
        .declare_service_incident("two", other_model, other_declaration.clone())
        .await
        .unwrap();
    let first_money = coupled(&f, model).await;
    let other_money = coupled(&f, other_model).await;
    repair(&f, model, input).await;
    repair(&f, other_model, other_input).await;
    money(&f, first_money, 0, 2, vec![]).await;
    money(&f, other_money, 0, 2, vec![]).await;
    let mut request = IncidentPlan {
        model_generation: 2,
        coupled_id: other_money,
        coupled_generation: 1,
        basis: "Reviewed match".into(),
    };
    assert!(
        f.ledger
            .link_service_incident_plan("scope", declaration.id, request.clone())
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO service_incident_plans(incident_id,model_id,model_generation,coupled_id,coupled_generation,basis) VALUES($1,$2,2,$3,1,'wrong scope')").bind(declaration.id).bind(model).bind(other_money).execute(&f.pool).await.is_err());
    request.coupled_id = first_money;
    let (a, b) = tokio::join!(
        f.ledger
            .link_service_incident_plan("winner-a", declaration.id, request.clone()),
        f.ledger
            .link_service_incident_plan("winner-b", declaration.id, request)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let (winner, error) = match (a, b) {
        (Ok(w), Err(e)) | (Err(e), Ok(w)) => (w, e),
        _ => panic!("exactly one plan link must commit"),
    };
    assert!(matches!(error, LedgerError::ScenarioClosed));
    let list = f
        .ledger
        .service_incidents(model, IncidentQuery::default())
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(json!(list[0]), json!(winner));
    assert_eq!(
        f.ledger
            .service_incident(other_declaration.id)
            .await
            .unwrap()
            .state,
        "open"
    );
    // The same UUID cannot be reused for a different model, including raw SQL.
    assert!(
        sqlx::query(
            "INSERT INTO service_incidents(id,model_id,base_generation,revision) VALUES($1,$2,1,$3)"
        )
        .bind(declaration.id)
        .bind(other_model)
        .bind(json!(declaration.revision))
        .execute(&f.pool)
        .await
        .is_err()
    );
}

#[path = "incident_revisions.rs"]
mod revisions;
