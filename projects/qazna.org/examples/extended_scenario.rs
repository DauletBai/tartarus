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
fn balance_candidate(m: &CapitalCycle, p: &mut [PeriodPlan]) {
    let retire = m.retirements(p).unwrap();
    let h = &m.seasonal.physical;
    let mut stock = h.initial.clone();
    let mut arrivals = vec![vec![0i64; R]; p.len()];
    for (t, row) in p.iter_mut().enumerate() {
        for r in 0..R {
            stock[r] += arrivals[t][r] + h.periods[t].supplies[r];
        }
        for (j, recipe) in h.processes.iter().enumerate() {
            let x = row.activities[j];
            if x == 0 {
                continue;
            }
            assert!(t + recipe.lead_periods < arrivals.len());
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
        for r in [10, 11, 12, 13].into_iter().filter(|_| t < 60) {
            let target = if t == 59 { h.initial[r] } else { 0 };
            row.discarded[r] = (stock[r] - target).max(0);
            stock[r] -= row.discarded[r];
        }
        row.closing = stock.clone();
    }
}

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
    balance_candidate(&m, &mut p);
    let h = &m.seasonal.physical;
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
        services: None,
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
    let bridge = boundary_bridge(&input, &check, &c);
    let recurrence = json!({
        "format_version":1,
        "three_month_bridge":bridge,
        "physical":([1,2,3].into_iter().map(|n| qazna::recurrence::repeat_physical(&input,n).unwrap()).collect::<Vec<_>>()),
        "money_without_renewal":qazna::recurrence::repeat_cash(&check,3,false).unwrap(),
        "money_with_explicit_treasury_renewal":qazna::recurrence::repeat_cash(&check,3,true).unwrap(),
        "limits":"Repeated money is a conditional pattern test, not evidence of repeated physical provision. Renewal uses existing treasury balances and fixed next-cycle prefix floors; no live authority. Repeated external flows remain declared boundaries."
    });
    let result = json!({"definition":definition,"input":input,"policy":policy,"reference":{"recurrence":recurrence,"source":"research/coupled-results.json#/upstream_capital/cases/1","mass":mass,"food":dated,"money":money,"limitations":["synthetic aggregate quantities and fixed food routes","common resource/capacity constraints, no measured national calibration","no physical receipt attestation or live monetary operation","money failure does not automatically rewrite production","18-month storage life is an assumption"]}});
    let output = env::args().nth(1).expect("output path");
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!(
        "Verified extended fixture: 60-month baseline and 63-month continuation with dated food and funded payments"
    );
}

// A prospective revision of the selected fixture. The first 57 periods are frozen.
// Replay food, capital and the constituent ledger from their original origins.
fn boundary_bridge(
    original: &ModelInput,
    money: &qazna::coupled::CoupledVersion,
    prior: &CoupledCheckpoint,
) -> Value {
    let mut next = original.clone();
    let s = &mut next.model.seasonal;
    for month in 0..3 {
        let period = s.physical.periods[month].clone();
        let mut row = original.plan[month].clone();
        row.activities.fill(0);
        row.deliveries.clone_from(&period.needs);
        next.plan.push(row);
        s.physical.periods.push(period);
        s.irrigation_m3.push(s.irrigation_m3[month]);
        s.closing_floors.push(s.closing_floors[month].clone());
        for land in &mut s.land {
            land.available_hectares.push(land.available_hectares[month]);
        }
    }
    // Manual food routes available in the extended months: preserve at t=60
    // for dispatch at t=61, and dispatch existing lots at t=60 for t=61.
    let h = &s.physical;
    for (food, preserve, dispatch) in [(6, 15, 17), (7, 19, 21), (8, 23, 25)] {
        next.plan[60].activities[preserve] = h.periods[62].needs[food];
        next.plan[60].activities[dispatch] = h.periods[61].needs[food];
        next.plan[61].activities[dispatch] = h.periods[62].needs[food];
    }
    let yield_per_fuel = h.processes[31].outputs[9];
    let units = |n: i64| n / yield_per_fuel + i64::from(n % yield_per_fuel != 0);
    for t in [60, 59] {
        let demand = h
            .processes
            .iter()
            .zip(&next.plan[t + 1].activities)
            .map(|(p, x)| p.inputs[9] * x)
            .sum();
        next.plan[t].activities[31] = units(demand);
    }
    // Each extra fuel unit costs one energy tick to extract. A generating unit
    // must also replace its own fuel, leaving yield_per_fuel - 1 net ticks.
    assert!(yield_per_fuel > 1);
    assert_eq!(h.processes[30].inputs[9], 1);
    assert_eq!(h.processes[30].outputs[20], 1);
    assert_eq!(h.processes[31].inputs[20], 1);
    let boundary_fuel = next.plan[59].activities[31];
    let net = yield_per_fuel - 1;
    let extra_generation = boundary_fuel / net + i64::from(boundary_fuel % net != 0);
    let extraction = boundary_fuel + extra_generation;
    next.plan[57].activities[31] += extra_generation;
    next.plan[58].activities[30] += extraction;
    balance_candidate(&next.model, &mut next.plan);
    next.boundary = 57;
    let physical =
        qazna::model_scenarios::preview_continuation(prior.evidence.physical.clone(), next.clone())
            .unwrap();
    let run = qazna::coupled::preview_continuation(money.clone(), physical, vec![]).unwrap();
    let e = serde_json::from_str::<CoupledCheckpoint>(run.checkpoint.as_ref().unwrap())
        .unwrap()
        .evidence;
    let failed_period = e.failed.first().map(|f| e.events[f.event].period);
    // All original prefix events settled. Fund the revised suffix before its
    // first event, using the treasury carried from the frozen prefix.
    use qazna::mathematics::{Movement, ScheduledMovement, replay_schedule, schedule_opening};
    let carried: Vec<u64> = e.closing_by_period[56]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    assert_eq!(
        e.closing_by_period[..57],
        prior.evidence.closing_by_period[..57]
    );
    let suffix: Vec<_> = e
        .events
        .iter()
        .filter(|v| v.period >= 57)
        .map(|v| ScheduledMovement {
            period: v.period as u32,
            movement: Movement {
                from: v.from,
                to: v.to,
                amount: v.amount.parse().unwrap(),
            },
            basis: format!("{:?} physical link {}", v.kind, v.link),
        })
        .collect();
    let required = schedule_opening(carried.len(), &suffix).unwrap();
    let floors: Vec<u64> = required
        .minimum_opening_by_account
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let mut funded_schedule: Vec<_> = (1..carried.len())
        .filter_map(|to| {
            let amount = floors[to].saturating_sub(carried[to]);
            (amount > 0).then(|| ScheduledMovement {
                period: 57,
                movement: Movement {
                    from: 0,
                    to,
                    amount,
                },
                basis: "Provision for the revised production schedule".into(),
            })
        })
        .collect();
    let draws: u64 = funded_schedule.iter().map(|e| e.movement.amount).sum();
    assert!(carried[0] >= draws + floors[0]);
    let allocations = json!(&funded_schedule);
    funded_schedule.extend(suffix);
    let funded = replay_schedule(&carried, &funded_schedule, &[]).unwrap();
    assert_eq!(funded.attempted_events, funded.settled_events);
    assert_eq!(funded.opening_total, e.money.opening_total);
    let extension_delivered: i64 = next.plan[60..]
        .iter()
        .flat_map(|p| &p.deliveries[6..9])
        .sum();
    json!({"periods":next.plan.len(),"first_replanned_period":57,"extra_extracted_fuel":extraction,
        "fuel_deposit_at_end":next.plan.last().unwrap().closing[18],"extension_food_delivered_kg":extension_delivered,
        "generation_at_59":boundary_fuel,"original_money_opening":e.money.opening_total,"events":e.events.len(),
        "settled_events":e.money.settled_events,"first_unfunded_period":failed_period,
        "opening_deficits_by_account":e.opening_deficits,"treasury_draw":draws,"allocations":allocations,
        "funded_suffix":funded,
        "physically_verified":true,"all_payments_settled":e.all_scheduled_payments_settled})
}
