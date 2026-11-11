use super::*;
use qazna::{coupled::*, model_scenarios::*, scenarios::ScenarioDecision};
use serde_json::json;
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../fixtures/extended-scenario.json")).unwrap()
}
fn decision() -> ScenarioDecision {
    ScenarioDecision {
        apply: true,
        basis: "Reviewed integrated synthetic evidence".into(),
    }
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn extended_model_and_circulation_survive_review_with_exact_food_and_mass() {
    let f = Fixture::new().await;
    let v = fixture();
    let mut def: ModelDefinition = serde_json::from_value(v["definition"].clone()).unwrap();
    def.id = Uuid::new_v4();
    let id = def.id;
    f.ledger
        .create_model_scenario("definition", def)
        .await
        .unwrap();
    let input: ModelInput = serde_json::from_value(v["input"].clone()).unwrap();
    let draft = f
        .ledger
        .propose_model_scenario(
            "plan",
            id,
            ModelProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                input: input.clone(),
                basis: "Five years".into(),
            },
        )
        .await
        .unwrap();
    let c: ModelCheckpoint =
        serde_json::from_str(draft.candidate.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(json!(c.food_routes), v["reference"]["food"]);
    assert_eq!(json!(c.materials[0]), v["reference"]["mass"]);
    f.ledger
        .resolve_model_scenario("approve-model", draft.request.id, decision())
        .await
        .unwrap();
    let cd = CoupledDefinition {
        id: Uuid::new_v4(),
        format_version: 1,
        model_id: id,
        policy: serde_json::from_value(v["policy"].clone()).unwrap(),
        basis: "Fixed schedule certified opening".into(),
    };
    let cid = cd.id;
    f.ledger
        .create_coupled_scenario("circulation", cd)
        .await
        .unwrap();
    let d = f
        .ledger
        .propose_coupled_scenario(
            "payments",
            cid,
            CoupledProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                model_generation: 1,
                unavailable_events: vec![],
                basis: "Exact derived events".into(),
            },
        )
        .await
        .unwrap();
    let e: CoupledCheckpoint =
        serde_json::from_str(d.candidate.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(e.evidence.links.len(), 2795);
    assert_eq!(e.evidence.events.len(), 5740);
    assert!(e.evidence.all_scheduled_payments_settled && e.evidence.resident_zero_each_period);
    assert_eq!(
        e.evidence.money.opening_total,
        v["reference"]["money"]["opening"]
    );
    f.ledger
        .resolve_coupled_scenario("approve-money", d.request.id, decision())
        .await
        .unwrap();
    assert_eq!(f.balance(f.treasury).await, "1000");
    assert_eq!(f.balance(f.resident).await, "0");
    let original = f.ledger.model_scenario(id, None).await.unwrap();
    for mode in 0..5 {
        let mut wrong = input.clone();
        match mode {
            0 => wrong.food_routes = None,
            1 => wrong.food_routes.as_mut().unwrap().harvests[0].shelf_periods += 1,
            2 => wrong.model.seasonal.physical.periods[20].capacities[43] = 0,
            3 => wrong.model.seasonal.physical.periods[20].capacities[31] = 0,
            _ => wrong.model.seasonal.physical.periods[20].capacities[17] = 0,
        }
        assert!(
            f.ledger
                .propose_model_scenario(
                    "retry",
                    id,
                    ModelProposal {
                        id: Uuid::new_v4(),
                        expected_generation: 1,
                        input: wrong,
                        basis: "Invalid resource/history claim".into()
                    }
                )
                .await
                .is_err()
        );
    }
    let mut next = input;
    next.boundary = 30;
    let d = f
        .ledger
        .propose_model_scenario(
            "retry",
            id,
            ModelProposal {
                id: Uuid::new_v4(),
                expected_generation: 1,
                input: next,
                basis: "Valid continuation after failed requests".into(),
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_model_scenario("next", d.request.id, decision())
        .await
        .unwrap();
    assert_eq!(
        json!(f.ledger.model_scenario(id, Some(1)).await.unwrap()),
        json!(original)
    );
    let d = f
        .ledger
        .propose_coupled_scenario(
            "next-payments",
            cid,
            CoupledProposal {
                id: Uuid::new_v4(),
                expected_generation: 1,
                model_generation: 2,
                unavailable_events: vec![],
                basis: "Preserve first 30 months".into(),
            },
        )
        .await
        .unwrap();
    f.ledger
        .resolve_coupled_scenario("next-money", d.request.id, decision())
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn food_route_units_and_initial_short_life_are_rejected_before_persistence() {
    let f = Fixture::new().await;
    let v = fixture();
    for mode in 0..2 {
        let mut def: ModelDefinition = serde_json::from_value(v["definition"].clone()).unwrap();
        def.id = Uuid::new_v4();
        let id = def.id;
        let mut input: ModelInput = serde_json::from_value(v["input"].clone()).unwrap();
        if mode == 0 {
            def.resources[0].unit = "different unit".into();
        } else {
            input
                .food_routes
                .as_mut()
                .unwrap()
                .initial_remaining_periods = vec![1; 9];
        }
        f.ledger
            .create_model_scenario(&format!("definition-{mode}"), def)
            .await
            .unwrap();
        assert!(
            f.ledger
                .propose_model_scenario(
                    &format!("plan-{mode}"),
                    id,
                    ModelProposal {
                        id: Uuid::new_v4(),
                        expected_generation: 0,
                        input,
                        basis: "Invalid interpretation".into()
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
