use super::*;
use qazna::{model_scenarios::*, scenarios::ScenarioDecision};
use serde_json::json;
fn input() -> (ModelDefinition, ModelInput) {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../research/population-results.json")).unwrap();
    let mut d: ModelDefinition =
        serde_json::from_value(v["runtime_fixture"]["definition"].clone()).unwrap();
    d.id = Uuid::new_v4();
    (
        d,
        serde_json::from_value(v["runtime_fixture"]["input"].clone()).unwrap(),
    )
}
fn decision() -> ScenarioDecision {
    ScenarioDecision {
        apply: true,
        basis: "Reviewed synthetic population calculation".into(),
    }
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn population_assumptions_are_frozen_and_need_understatement_is_rejected() {
    let f = Fixture::new().await;
    let (def, data) = input();
    let id = def.id;
    f.ledger
        .create_model_scenario("population", def)
        .await
        .unwrap();
    let proposal = |input: ModelInput, generation: i64| ModelProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        input,
        basis: "Explicit population scenario".into(),
    };
    let d = f
        .ledger
        .propose_model_scenario("first", id, proposal(data.clone(), 0))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("apply", d.request.id, decision())
        .await
        .unwrap();
    let saved = f.ledger.model_scenario(id, None).await.unwrap();
    let evidence: ModelCheckpoint =
        serde_json::from_str(saved.checkpoint.as_ref().unwrap()).unwrap();
    let pop = evidence.population.unwrap();
    assert_eq!(pop.projection.initial, "100000000");
    assert!(!pop.projection.all_selected_needs_known);
    assert!(pop.coverage.iter().all(|r| r.unmet == "0"));
    for mode in 0..4 {
        let mut wrong = data.clone();
        wrong.boundary = 20;
        match mode {
            0 => wrong.population = None,
            1 => {
                wrong.population.as_mut().unwrap().study.registry.version =
                    "silently replaced source".into()
            }
            2 => {
                wrong.model.seasonal.physical.periods[20].needs[0] -= 1;
                wrong.plan[20].deliveries[0] -= 1;
                for row in &mut wrong.plan[20..] {
                    row.closing[0] += 1;
                }
            }
            _ => wrong.population.as_mut().unwrap().study.initial[0].age_months = 1,
        }
        assert!(
            f.ledger
                .propose_model_scenario("retry", id, proposal(wrong, 1))
                .await
                .is_err()
        );
        assert_eq!(
            json!(f.ledger.model_scenario(id, None).await.unwrap()),
            json!(saved)
        );
    }
    let mut next = data;
    next.boundary = 30;
    let d = f
        .ledger
        .propose_model_scenario("retry", id, proposal(next, 1))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("apply-next", d.request.id, decision())
        .await
        .unwrap();
    let current = f.ledger.model_scenario(id, None).await.unwrap();
    assert_eq!(current.generation, 2);
    let now: ModelCheckpoint = serde_json::from_str(current.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(json!(now.population), json!(pop));
    assert_eq!(
        json!(f.ledger.model_scenario(id, Some(1)).await.unwrap()),
        json!(saved)
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn population_units_and_unknown_demographic_sources_cannot_be_approved() {
    let f = Fixture::new().await;
    for mode in 0..2 {
        let (mut d, mut data) = input();
        if mode == 0 {
            d.resources[0].unit = "g".into();
        } else {
            let p = data
                .population
                .as_mut()
                .unwrap()
                .study
                .registry
                .parameters
                .iter_mut()
                .find(|p| p.id == "births")
                .unwrap();
            p.estimate = None;
            p.kind = qazna::population::EvidenceKind::Unknown;
        }
        let id = d.id;
        f.ledger
            .create_model_scenario(&format!("create-{mode}"), d)
            .await
            .unwrap();
        assert!(
            f.ledger
                .propose_model_scenario(
                    &format!("invalid-{mode}"),
                    id,
                    ModelProposal {
                        id: Uuid::new_v4(),
                        expected_generation: 0,
                        input: data,
                        basis: "Invalid imported evidence".into()
                    }
                )
                .await
                .is_err()
        );
        assert_eq!(
            f.ledger.model_scenario(id, None).await.unwrap().generation,
            0
        );
    }
}

fn profile_input() -> (ModelDefinition, ModelInput) {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../research/need-profile-results.json")).unwrap();
    let mut d: ModelDefinition =
        serde_json::from_value(v["runtime_fixture"]["definition"].clone()).unwrap();
    d.id = Uuid::new_v4();
    (
        d,
        serde_json::from_value(v["runtime_fixture"]["input"].clone()).unwrap(),
    )
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn regional_shortage_rejection_preserves_retry_and_approved_profile_history() {
    let f = Fixture::new().await;
    let (d, data) = profile_input();
    let id = d.id;
    f.ledger.create_model_scenario("profiles", d).await.unwrap();
    let proposal = |input, generation| ModelProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        input,
        basis: "Reviewed local needs".into(),
    };
    let mut wrong = data.clone();
    let local = &mut wrong
        .population
        .as_mut()
        .unwrap()
        .profiles
        .as_mut()
        .unwrap()
        .local_delivery;
    local[0].quantities[0] += 1;
    local[1].quantities[0] -= 1;
    assert!(
        f.ledger
            .propose_model_scenario("same-key", id, proposal(wrong, 0))
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        0
    );
    let draft = f
        .ledger
        .propose_model_scenario("same-key", id, proposal(data.clone(), 0))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("approve", draft.request.id, decision())
        .await
        .unwrap();
    let saved = f.ledger.model_scenario(id, None).await.unwrap();
    for mode in 0..3 {
        let mut next = data.clone();
        next.boundary = 30;
        let profiles = &mut next.population.as_mut().unwrap().profiles;
        match mode {
            0 => *profiles = None,
            1 => profiles.as_mut().unwrap().exposure_source.source_version = "2".into(),
            _ => profiles.as_mut().unwrap().cells[0].person_days[40] -= 1,
        }
        assert!(
            f.ledger
                .propose_model_scenario("next", id, proposal(next, 1))
                .await
                .is_err()
        );
        assert_eq!(
            json!(f.ledger.model_scenario(id, None).await.unwrap()),
            json!(saved)
        );
    }
    let mut next = data;
    next.boundary = 30;
    let draft = f
        .ledger
        .propose_model_scenario("next", id, proposal(next, 1))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("approve-next", draft.request.id, decision())
        .await
        .unwrap();
    assert_eq!(
        json!(f.ledger.model_scenario(id, Some(1)).await.unwrap()),
        json!(saved)
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn service_shortage_and_rewritten_capacity_preserve_reviewed_history() {
    let f = Fixture::new().await;
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../research/service-results.json")).unwrap();
    let mut d: ModelDefinition =
        serde_json::from_value(v["runtime_fixture"]["definition"].clone()).unwrap();
    d.id = Uuid::new_v4();
    let id = d.id;
    let data: ModelInput = serde_json::from_value(v["runtime_fixture"]["input"].clone()).unwrap();
    f.ledger.create_model_scenario("service", d).await.unwrap();
    let proposal = |input, generation| ModelProposal {
        id: Uuid::new_v4(),
        expected_generation: generation,
        input,
        basis: "Reviewed care and education capacity".into(),
    };
    let mut wrong = data.clone();
    wrong.services.as_mut().unwrap().pools[0].availability_bps[20] = 0;
    assert!(
        f.ledger
            .propose_model_scenario("retry", id, proposal(wrong, 0))
            .await
            .is_err()
    );
    assert_eq!(
        f.ledger.model_scenario(id, None).await.unwrap().generation,
        0
    );
    let draft = f
        .ledger
        .propose_model_scenario("retry", id, proposal(data.clone(), 0))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("apply", draft.request.id, decision())
        .await
        .unwrap();
    let saved = f.ledger.model_scenario(id, None).await.unwrap();
    let evidence: ModelCheckpoint =
        serde_json::from_str(saved.checkpoint.as_ref().unwrap()).unwrap();
    assert!(evidence.services.unwrap().all_selected_services_feasible);
    for mode in 0..3 {
        let mut next = data.clone();
        next.boundary = 30;
        match mode {
            0 => next.services = None,
            1 => next.services.as_mut().unwrap().registry.version = "unreviewed norms".into(),
            _ => {
                next.services.as_mut().unwrap().pools[0].process_load_parameters[1] =
                    "no_work".into()
            }
        }
        assert!(
            f.ledger
                .propose_model_scenario("next", id, proposal(next, 1))
                .await
                .is_err()
        );
        assert_eq!(
            json!(f.ledger.model_scenario(id, None).await.unwrap()),
            json!(saved)
        );
    }
    let mut next = data;
    next.boundary = 30;
    let draft = f
        .ledger
        .propose_model_scenario("next", id, proposal(next, 1))
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("apply-next", draft.request.id, decision())
        .await
        .unwrap();
    assert_eq!(
        json!(f.ledger.model_scenario(id, Some(1)).await.unwrap()),
        json!(saved)
    );
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
}
