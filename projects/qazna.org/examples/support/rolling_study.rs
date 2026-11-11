//! Continuation of the upstream graph plus a separately typed material subchain.
use qazna::{
    capital::CapitalCycle,
    horizon::{Horizon, Period, PeriodPlan, Process},
    materials::MaterialBasis,
    mathematics::verify_linear,
    rolling::RollingCapital,
};
use serde_json::{Value, json};
fn material_case(
    name: &str,
    recycle: bool,
    energy: i64,
    recovery_capacity: i64,
    invent: bool,
) -> Value {
    // Mass tracks one dry constituent; energy is a different dimension.
    let mut h = Horizon {
        resources: vec![
            "ore_kg".into(),
            "material_kg".into(),
            "recoverable_residue_kg".into(),
            "unrecovered_waste_kg".into(),
            "energy_ticks".into(),
        ],
        initial: vec![150, 0, 0, 0, energy],
        storage_caps: vec![1000; 5],
        processes: vec![
            Process {
                inputs: vec![10, 0, 0, 0, 2],
                outputs: vec![0, 7, 3, 0, 0],
                lead_periods: 2,
            },
            Process {
                inputs: vec![0, 0, 5, 0, 1],
                outputs: vec![0, if invent { 3 } else { 2 }, 0, 3, 0],
                lead_periods: 1,
            },
        ],
        periods: vec![
            Period {
                supplies: vec![0; 5],
                needs: vec![0; 5],
                capacities: vec![15, recovery_capacity]
            };
            6
        ],
        objective_resource: 1,
    };
    h.periods[5].needs[1] = 100;
    let mut plans: Vec<PeriodPlan> = (0..6)
        .map(|_| PeriodPlan {
            activities: vec![0; 2],
            deliveries: vec![0; 5],
            closing: vec![0; 5],
            discarded: vec![0; 5],
        })
        .collect();
    plans[0].activities[0] = if recycle { 13 } else { 15 };
    plans[2].activities[1] = if recycle { 5 } else { 0 };
    plans[5].deliveries[1] = 100;
    let mut stock = h.initial.clone();
    for t in 0..6 {
        for (j, p) in h.processes.iter().enumerate() {
            for (k, s) in stock.iter_mut().enumerate() {
                if t >= p.lead_periods {
                    *s += plans[t - p.lead_periods].activities[j] * p.outputs[k];
                }
                *s -= plans[t].activities[j] * p.inputs[k];
            }
        }
        for (s, &d) in stock.iter_mut().zip(&plans[t].deliveries) {
            *s -= d;
        }
        plans[t].closing = stock.clone();
    }
    let basis=MaterialBasis{name:"tracked dry material".into(),unit:"kg".into(),source:"Synthetic recipe v1: 70% primary yield, 40% residue recovery; not measured mineral engineering".into(),quantity_per_resource_unit:vec![1,1,1,1,0]};
    let matrix = verify_linear(&h.compile().unwrap(), &h.vector(&plans).unwrap(), None);
    let audit = basis.audit(&h, &plans);
    let feasible = matrix.is_ok() && audit.is_ok();
    json!({"name":name,"feasible":feasible,"exact_matrix":format!("{matrix:?}"),"material_audit":match audit{Ok(e)=>json!(e),Err(e)=>json!({"error":format!("{e:?}")})},"ore_consumed_kg":plans[0].activities[0]*10,"energy_consumed_ticks":plans[0].activities[0]*2+plans[2].activities[1],"basis":basis,"model":h,"candidate":plans})
}
pub fn study(upstream: &Value) -> Value {
    let selected = &upstream["cases"][1];
    let model: CapitalCycle = serde_json::from_value(selected["model"].clone()).unwrap();
    let plan: Vec<PeriodPlan> = serde_json::from_value(selected["candidate"].clone()).unwrap();
    let mut boundaries = vec![];
    for cut in [1, 12, 15, 18, 24, 30, 42, 59] {
        let checkpoint = RollingCapital::checkpoint(&model, &plan, cut).unwrap();
        let matrix = checkpoint.compile(&model).unwrap();
        let exact = verify_linear(
            &matrix,
            &model.seasonal.physical.vector(&plan).unwrap(),
            None,
        )
        .is_ok();
        let replay = checkpoint.replay(&model, &plan).is_ok();
        assert!(exact && replay);
        boundaries.push(json!({"snapshot":checkpoint.snapshot(),"exact_matrix":exact,"independent_replay":replay,"columns":matrix.objective.len(),"rows":matrix.rows.len()}));
    }
    let checkpoint = RollingCapital::checkpoint(&model, &plan, 15).unwrap();
    let mut revised = model.clone();
    revised.seasonal.physical.periods[15].supplies[9] += 1;
    let mut revised_plan = plan.clone();
    revised_plan[15].discarded[9] += 1;
    let revised_ok = verify_linear(
        &checkpoint.compile(&revised).unwrap(),
        &revised.seasonal.physical.vector(&revised_plan).unwrap(),
        None,
    )
    .is_ok()
        && checkpoint.replay(&revised, &revised_plan).is_ok();
    let mut reset = model.clone();
    reset
        .lives
        .iter_mut()
        .find(|l| l.resource == 22)
        .unwrap()
        .initial_remaining_periods = 24;
    let reset_rejected =
        checkpoint.compile(&reset).is_err() && checkpoint.replay(&reset, &plan).is_err();
    let mut outage = model.clone();
    outage.seasonal.physical.periods[18].capacities[31] = 0;
    let outage_rejected = verify_linear(
        &checkpoint.compile(&outage).unwrap(),
        &outage.seasonal.physical.vector(&plan).unwrap(),
        None,
    )
    .is_err()
        && checkpoint.replay(&outage, &plan).is_err();
    assert!(revised_ok && reset_rejected && outage_rejected);
    json!({"kind":"absolute_timeline_continuation_and_constituent_accounting","source_case":selected["name"],"months":60,"model_calls":0,"checkpoints":boundaries,"future_supply_revision_accepted":revised_ok,"asset_age_reset_rejected":reset_rejected,"unchanged_plan_after_future_outage_rejected":outage_rejected,
        "material_cases":[material_case("primary_only",false,100,5,false),material_case("residue_recovery",true,100,5,false),material_case("insufficient_recovery_energy",true,30,5,false),material_case("recovery_capacity_shortage",true,100,4,false),material_case("invented_recovery_mass",true,100,5,true)],
        "scope":["Checkpoints freeze mathematically verified candidate prefixes, not externally authenticated execution records",
        "Absolute time and original inputs preserve finite deposits, open construction/harvest, seasonal water/land commitments and asset cohort expiry; past starts cannot be charged twice or erased",
        "The full prefix stays in the matrix. This bounded implementation does not compact history, import unsigned snapshots, change recipe versions mid-flight or persist runtime plans",
        "The typed kg material experiment is separate from synthetic upstream ticks: 70% primary yield and 40% residue recovery are assumptions, not measured ore grades",
        "All tracked mass remains in stock, provision, explicit disposal or committed work. Recovery consumes energy and capacity and leaves unrecovered waste",
        "Asset dismantling/recycling, material chemistry, hazards, crop nutrient retention and original dated-lot checkpoints remain separate gates; aggregate checkpoint stock does not certify freshness",
        "No autonomous postings, issuance, AI calls, national-scale cost estimate or optimality claim"]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_preserves_mass_with_explicit_energy_tradeoff() {
        let a = material_case("primary", false, 100, 5, false);
        let b = material_case("recovery", true, 100, 5, false);
        assert_eq!(a["feasible"], true);
        assert_eq!(b["feasible"], true);
        assert_eq!(a["ore_consumed_kg"], 150);
        assert_eq!(b["ore_consumed_kg"], 130);
        assert_eq!(a["energy_consumed_ticks"], 30);
        assert_eq!(b["energy_consumed_ticks"], 31);
        assert_eq!(b["candidate"][5]["closing"], json!([20, 1, 14, 15, 69]));
        assert_eq!(
            b["material_audit"]["work_in_progress"],
            json!([130, 130, 25, 0, 0, 0])
        );
    }
    #[test]
    fn recovery_rejects_unbalanced_mass_and_missing_capabilities() {
        assert_eq!(
            material_case("energy", true, 30, 5, false)["feasible"],
            false
        );
        assert_eq!(
            material_case("capacity", true, 100, 4, false)["feasible"],
            false
        );
        let invented = material_case("mass", true, 100, 5, true);
        assert_eq!(invented["feasible"], false);
        assert!(invented["exact_matrix"].as_str().unwrap().starts_with("Ok"));
    }
    #[test]
    fn large_graph_continues_with_construction_and_crop_commitments() {
        let source = super::super::upstream_study::study(&super::super::automation_study::study(
            &super::super::seasonal_study::study(&super::super::nutrition_study::study()),
        ));
        let out = study(&source);
        let c = out["checkpoints"].as_array().unwrap();
        assert_eq!(c.len(), 8);
        assert!(
            c.iter()
                .all(|c| c["independent_replay"] == true && c["exact_matrix"] == true)
        );
        let p = c[2]["snapshot"]["pending"].as_array().unwrap();
        assert!(
            p.iter()
                .any(|x| x["process"] == 37 && x["started"] == 14 && x["completes"] == 17)
        );
        assert_eq!(out["asset_age_reset_rejected"], true);
    }
}
