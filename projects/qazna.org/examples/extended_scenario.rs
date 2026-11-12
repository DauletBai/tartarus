//! Reproducible runtime fixture: five-year upstream graph plus packaging recovery.
//! Reads the maintained synthetic report, never empirical or normative authority.
use qazna::{
    capital::CapitalCycle,
    coupled::{CirculationPolicy, CoupledCheckpoint, CoupledDefinition},
    food_routes::{FoodRoutes, HarvestRoute},
    horizon::{PeriodPlan, Process},
    materials::MaterialBasis,
    mathematics::verify_linear,
    model_scenarios::{ModelDefinition, ModelInput, ModelResource},
};
use serde_json::{Value, json};
use std::{env, fs};
use uuid::Uuid;
const R: usize = 31;
const J: usize = 44;
fn main() {
    let source: Value =
        serde_json::from_slice(&fs::read("research/coupled-results.json").unwrap()).unwrap();
    let c = &source["upstream_capital"]["cases"][1];
    assert_eq!(c["name"], "demand_matched_power_with_smaller_fuel_deposit");
    let mut m: CapitalCycle = serde_json::from_value(c["model"].clone()).unwrap();
    let mut p: Vec<PeriodPlan> = serde_json::from_value(c["candidate"].clone()).unwrap();
    let h = &mut m.seasonal.physical;
    h.resources.extend(
        [
            "finite_packaging_ore_kg",
            "recovered_packaging_material_kg",
            "recoverable_residue_kg",
            "packaging_waste_g",
        ]
        .map(str::to_string),
    );
    h.initial.extend([600_000_000, 24_000_000, 0, 0]);
    h.storage_caps.resize(R, 10_000_000_000_000);
    for recipe in &mut h.processes {
        recipe.inputs.resize(R, 0);
        recipe.outputs.resize(R, 0);
        // Explicit packaging residue at the consuming process; no disappearance.
        recipe.outputs[30] = recipe.inputs[10];
    }
    h.processes[33].inputs[21] = 0;
    h.processes[33].inputs[28] = 1;
    for (input, output, lag) in [
        (vec![(27, 10), (9, 2)], vec![(28, 7), (29, 3)], 2),
        (vec![(29, 5), (9, 1)], vec![(28, 2), (30, 3000)], 1),
    ] {
        let mut recipe = Process {
            inputs: vec![0; R],
            outputs: vec![0; R],
            lead_periods: lag,
        };
        for (r, q) in input {
            recipe.inputs[r] = q;
        }
        for (r, q) in output {
            recipe.outputs[r] = q;
        }
        h.processes.push(recipe);
    }
    for row in &mut h.periods {
        row.supplies.resize(R, 0);
        row.needs.resize(R, 0);
        row.capacities.resize(J, 2_000_000);
    }
    for floor in &mut m.seasonal.closing_floors {
        floor.resize(R, 0);
    }
    for limit in &mut m.limits {
        limit.activity_use.resize(J, 0);
        if ["shared_factory", "shared_human_work_ticks"].contains(&limit.name.as_str()) {
            limit.activity_use[42] = 1;
            limit.activity_use[43] = 1;
        }
    }
    for (t, row) in p.iter_mut().enumerate() {
        row.activities.resize(J, 0);
        row.deliveries.resize(R, 0);
        row.closing.resize(R, 0);
        row.discarded.resize(R, 0);
        if t < 58 {
            row.activities[42] = 1_000_000;
        }
        if (2..59).contains(&t) {
            row.activities[43] = 600_000;
        }
    }
    // Recompute the existing declared demand-following fuel schedule for new energy use.
    for iteration in 0..64 {
        let old: Vec<_> = p
            .iter()
            .map(|r| (r.activities[30], r.activities[31]))
            .collect();
        let first = p[0].activities[31];
        for t in (0..59).rev() {
            p[t].activities[30] = p[t + 1].activities[31] + if t == 58 { first } else { 0 };
            let need: i64 = p[t + 1]
                .activities
                .iter()
                .zip(&h.processes)
                .map(|(&x, r)| x * r.inputs[9])
                .sum();
            p[t].activities[31] = need / 1000 + i64::from(need % 1000 != 0);
        }
        if p.iter()
            .map(|r| (r.activities[30], r.activities[31]))
            .eq(old)
        {
            break;
        }
        assert!(iteration < 63, "generation did not converge");
    }
    let retire = m.retirements(&p).unwrap();
    let h = &m.seasonal.physical;
    let mut stock = h.initial.clone();
    let mut arrivals = vec![vec![0i64; R]; 60];
    for (t, row) in p.iter_mut().enumerate() {
        for r in 0..R {
            stock[r] += arrivals[t][r] + h.periods[t].supplies[r];
        }
        for (j, recipe) in h.processes.iter().enumerate() {
            let x = row.activities[j];
            if x == 0 {
                continue;
            }
            assert!(t + recipe.lead_periods < 60);
            for (r, s) in stock.iter_mut().enumerate() {
                *s -= x * recipe.inputs[r];
                arrivals[t + recipe.lead_periods][r] += x * recipe.outputs[r];
            }
        }
        row.discarded = retire[t].clone();
        for (r, s) in stock.iter_mut().enumerate() {
            *s -= row.deliveries[r] + row.discarded[r];
        }
        row.discarded[9] = stock[9].max(0);
        stock[9] -= row.discarded[9];
        for r in [10, 11, 12, 13] {
            let target = if t == 59 { h.initial[r] } else { 0 };
            row.discarded[r] = (stock[r] - target).max(0);
            stock[r] -= row.discarded[r];
        }
        row.closing = stock.clone();
    }
    let mut weights = vec![0; R];
    for r in [27, 28, 29] {
        weights[r] = 1000;
    }
    weights[10] = 1;
    weights[30] = 1;
    let basis=MaterialBasis{name:"packaging dry constituent".into(),unit:"g".into(),source:"Synthetic v1: ore 70% primary yield, residue 40% recovery, packaging 1 g/tick; waste is explicit; not calibrated".into(),quantity_per_resource_unit:weights};
    let routes=FoodRoutes {resources:(0..9).collect(),initial_remaining_periods:vec![18;9],harvests:(0..15).map(|process| HarvestRoute{process,shelf_periods:18}).collect(),transfers:(15..27).collect(),unit:"gross-equivalent kg".into(),basis:"Inherited fixed-yield one-to-one preserved-food routes; synthetic 18-month shelf assumption, not food-safety certification".into()};
    verify_linear(&m.compile().unwrap(), &h.vector(&p).unwrap(), None).unwrap();
    m.replay(&p).unwrap();
    let mass = basis.audit(h, &p).unwrap();
    let dated = routes.audit(&m, &p).unwrap();
    let resources = h
        .resources
        .iter()
        .enumerate()
        .map(|(r, name)| ModelResource {
            name: name.clone(),
            unit: if r < 9 {
                "gross-equivalent kg"
            } else if [27, 28, 29].contains(&r) {
                "kg"
            } else if r == 30 {
                "g"
            } else {
                "declared synthetic unit"
            }
            .into(),
        })
        .collect();
    let definition=ModelDefinition{id:Uuid::nil(),format_version:1,resources,period_unit:"month".into(),time_origin:"Synthetic month zero".into(),dated_bindings:vec![],basis:"Selected 60-month 100-million aggregate demand candidate; not calibrated national coefficients".into()};
    let mut norm = vec!["1".to_string(); R];
    for r in [13, 15, 16, 17, 18, 27, 29, 30] {
        norm[r] = "0".into();
    }
    let mut policy=CirculationPolicy{accounts:[vec!["common treasury".into(),"aggregate provision".into()],h.resources.iter().map(|r|format!("shared unit: {r}")).collect()].concat(),resource_suppliers:(2..R+2).collect(),process_operators:h.processes.iter().map(|p|2+p.outputs.iter().position(|&q|q>0).unwrap()).collect(),process_order:(0..J).collect(),capital_resources:m.lives.iter().map(|l|l.resource).collect(),common_measure:norm.clone(),unit_measure:norm,opening:(0..R+2).map(|i|if i==1 {"0"}else{"1000000000000000"}.into()).collect(),measure_unit:"synthetic fixed accounting tick".into(),basis:"Illustrative common-production shares; exact minimum opening for this fixed schedule only, not a universal funding or issuance rule; people and in-situ deposits unpriced".into()};
    let input = ModelInput {
        model: m,
        plan: p,
        boundary: 18,
        material_bases: vec![basis],
        dated_generations: vec![],
        food_routes: Some(routes),
        population: None,
    };
    // Derive an explicit offline opening certificate; the live service never
    // silently increases a submitted opening policy or issues the difference.
    let physical = qazna::model_scenarios::preview(definition.clone(), input.clone()).unwrap();
    let def = CoupledDefinition {
        id: Uuid::nil(),
        format_version: 1,
        model_id: definition.id,
        policy: policy.clone(),
        basis: "Synthetic fixed-schedule certificate".into(),
    };
    let trial = qazna::coupled::preview(def.clone(), physical.clone(), vec![]).unwrap();
    let c: CoupledCheckpoint = serde_json::from_str(trial.checkpoint.as_ref().unwrap()).unwrap();
    policy.opening = c.evidence.bound.minimum_opening_by_account.clone();
    let check = qazna::coupled::preview(
        CoupledDefinition {
            policy: policy.clone(),
            ..def
        },
        physical,
        vec![],
    )
    .unwrap();
    let c: CoupledCheckpoint = serde_json::from_str(check.checkpoint.as_ref().unwrap()).unwrap();
    assert!(c.evidence.all_scheduled_payments_settled && c.evidence.resident_zero_each_period);
    let money = json!({"links":c.evidence.links.len(),"events":c.evidence.events.len(),"opening":c.evidence.money.opening_total,
        "closing":c.evidence.money.closing,"settled_volume":c.evidence.money.settled_volume,"bound":c.evidence.bound});
    let result = json!({"definition":definition,"input":input,"policy":policy,"reference":{"source":"research/coupled-results.json#/upstream_capital/cases/1","mass":mass,"food":dated,"money":money,"limitations":["synthetic aggregate quantities and fixed food routes","common resource/capacity constraints, no measured national calibration","no physical receipt attestation or live monetary operation","money failure does not automatically rewrite production","18-month storage life is an assumption"]}});
    let output = env::args().nth(1).expect("output path");
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!(
        "Verified extended fixture: 60 months, 31 resources, 44 processes, food identity and packaging recovery"
    );
}
