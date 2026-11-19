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

fn schedule_power(h: &qazna::horizon::Horizon, p: &mut [PeriodPlan]) {
    // Recompute the existing declared demand-following fuel schedule for new energy use.
    for iteration in 0..64 {
        let old: Vec<_> = p
            .iter()
            .map(|r| (r.activities[30], r.activities[31]))
            .collect();
        let first = p[0].activities[31];
        for t in (0..p.len() - 1).rev() {
            p[t].activities[30] =
                p[t + 1].activities[31] + if t + 2 == p.len() { first } else { 0 };
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
    schedule_power(h, &mut p);
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
    let january = january_opening(&definition, &input, &check.definition);
    let bridge = boundary_bridge(&input, &check, &c);
    let harvest = harvest_renewal(&input, &check, &c);
    let recurrence = json!({
        "format_version":1,
        "three_month_bridge":bridge,
        "sixth_year_harvest":harvest,
        "physical":([1,2,3].into_iter().map(|n| qazna::recurrence::repeat_physical(&input,n).unwrap()).collect::<Vec<_>>()),
        "money_without_renewal":qazna::recurrence::repeat_cash(&check,3,false).unwrap(),
        "money_with_explicit_treasury_renewal":qazna::recurrence::repeat_cash(&check,3,true).unwrap(),
        "limits":"Repeated money is a conditional pattern test, not evidence of repeated physical provision. Renewal uses existing treasury balances and fixed next-cycle prefix floors; no live authority. Repeated external flows remain declared boundaries."
    });
    let result = json!({"definition":definition,"input":input,"policy":policy,"reference":{"january_opening":january,"recurrence":recurrence,"source":"research/coupled-results.json#/upstream_capital/cases/1","mass":mass,"food":dated,"money":money,"limitations":["synthetic aggregate quantities and fixed food routes","common resource/capacity constraints, no measured national calibration","no physical receipt attestation or live monetary operation","money failure does not automatically rewrite production","18-month storage life is an assumption"]}});
    let output = env::args().nth(1).expect("output path");
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!(
        "Verified extended fixture: 60-month baseline, 63-month bridge and 72-month harvest scenarios with dated food and funded payments"
    );
}

// A prospective revision of the selected fixture. The first 57 periods are frozen.
// Replay food, capital and the constituent ledger from their original origins.
fn boundary_bridge(
    original: &ModelInput,
    money: &qazna::coupled::CoupledVersion,
    prior: &CoupledCheckpoint,
) -> Value {
    let mut next = extend_calendar(original, 3);
    // Manual food routes available in the extended months: preserve at t=60
    // for dispatch at t=61, and dispatch existing lots at t=60 for t=61.
    let h = &next.model.seasonal.physical;
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
    let extension_delivered: i64 = next.plan[60..]
        .iter()
        .flat_map(|p| &p.deliveries[6..9])
        .sum();
    let periods = next.plan.len();
    let deposit = next.plan.last().unwrap().closing[18];
    let (e, allocations, draws, funded) = verify_funding(next, money, prior);
    let failed_period = e.failed.first().map(|f| e.events[f.event].period);
    json!({"periods":periods,"first_replanned_period":57,"extra_extracted_fuel":extraction,
        "fuel_deposit_at_end":deposit,"extension_food_delivered_kg":extension_delivered,
        "generation_at_59":boundary_fuel,"original_money_opening":e.money.opening_total,"events":e.events.len(),
        "settled_events":e.money.settled_events,"first_unfunded_period":failed_period,
        "opening_deficits_by_account":e.opening_deficits,"treasury_draw":draws,"allocations":allocations,
        "funded_suffix":funded,
        "physically_verified":true,"all_payments_settled":e.all_scheduled_payments_settled})
}

fn verify_funding(
    next: ModelInput,
    money: &qazna::coupled::CoupledVersion,
    prior: &CoupledCheckpoint,
) -> (
    qazna::coupled::CoupledEvidence,
    Value,
    u64,
    qazna::mathematics::ScheduleRun,
) {
    let physical =
        qazna::model_scenarios::preview_continuation(prior.evidence.physical.clone(), next)
            .unwrap();
    let reviewed =
        qazna::coupled::preview_funded_continuation(money.clone(), physical.clone()).unwrap();
    let reviewed: CoupledCheckpoint =
        serde_json::from_str(reviewed.checkpoint.as_ref().unwrap()).unwrap();
    let run = qazna::coupled::preview_continuation(money.clone(), physical, vec![]).unwrap();
    let e = serde_json::from_str::<CoupledCheckpoint>(run.checkpoint.as_ref().unwrap())
        .unwrap()
        .evidence;

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
    let receipts = &reviewed.evidence.treasury_allocations;
    assert_eq!(receipts.len(), funded_schedule.len());
    for (entry, independent) in receipts.iter().zip(&funded_schedule) {
        assert_eq!(entry.period, independent.period as usize);
        assert_eq!(entry.to, independent.movement.to);
        assert_eq!(entry.amount, independent.movement.amount.to_string());
    }
    let allocations = json!(&funded_schedule);
    funded_schedule.extend(suffix);
    let funded = replay_schedule(&carried, &funded_schedule, &[]).unwrap();
    assert_eq!(funded.attempted_events, funded.settled_events);
    assert_eq!(funded.opening_total, e.money.opening_total);
    assert!(reviewed.evidence.all_scheduled_payments_settled);
    assert_eq!(reviewed.evidence.money.closing, funded.closing);
    assert_eq!(
        reviewed.evidence.closing_by_period[..57],
        prior.evidence.closing_by_period[..57]
    );
    (e, allocations, draws, funded)
}

fn extend_calendar(original: &ModelInput, months: usize) -> ModelInput {
    let mut next = original.clone();
    let s = &mut next.model.seasonal;
    for month in 0..months {
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
    next.boundary = 57;
    next
}

fn batches(quantity: i64, yield_per_batch: i64) -> i64 {
    assert!(quantity >= 0 && yield_per_batch > 0);
    quantity / yield_per_batch + i64::from(quantity % yield_per_batch != 0)
}

// Replan prospectively from the same five-year parent as the bridge.
// All dates below are absolute. Grow enough for the declared closing stocks.
fn harvest_renewal(
    original: &ModelInput,
    money: &qazna::coupled::CoupledVersion,
    prior: &CoupledCheckpoint,
) -> Value {
    let mut next = extend_calendar(original, 12);
    let seasons = &mut next.model.seasonal;
    seasons.closing_floors[71] = seasons.closing_floors[59].clone();
    for life in &next.model.lives {
        seasons.closing_floors[71][life.resource] = original.plan[59].closing[life.resource];
    }
    let mut harvested = Vec::new();
    for (crop, preserve, dispatch) in [(0, 15, 17), (1, 19, 21), (2, 23, 25)] {
        let delivered = crop + 6;
        for t in 60..71 {
            next.plan[t].activities[dispatch] = next.plan[t + 1].deliveries[delivered];
        }
        next.plan[70].activities[dispatch] += seasons.closing_floors[71][delivered];
        for t in 60..70 {
            next.plan[t].activities[preserve] = next.plan[t + 1].activities[dispatch];
        }
        next.plan[70].activities[preserve] = seasons.closing_floors[71][crop + 3];
        let processed: i64 = next.plan[60..]
            .iter()
            .map(|row| row.activities[preserve])
            .sum();
        let recipe = &seasons.physical.processes[crop];
        let net = recipe.outputs[crop] - recipe.inputs[crop];
        let area = batches(
            processed + seasons.closing_floors[71][crop] - original.plan[59].closing[crop],
            net,
        );
        seasons
            .crops
            .iter_mut()
            .find(|c| c.process == crop)
            .unwrap()
            .sowing_periods
            .push(63);
        next.plan[63].activities[crop] = area;
        harvested.push(json!({"resource": seasons.physical.resources[crop],"hectares":area,
            "seed_kg":area*recipe.inputs[crop],"harvest_period":63+recipe.lead_periods,"harvest_kg":area*recipe.outputs[crop]}));
    }

    let h = &next.model.seasonal.physical;
    let retire = next.model.retirements(&next.plan).unwrap();
    let mut replacement = Vec::new();
    for (t, quantities) in retire.iter().enumerate().skip(60) {
        for (asset, build) in [(14, 27), (22, 37), (23, 38), (24, 39), (25, 40), (26, 41)] {
            if quantities[asset] > 0 {
                // The factory already has a second installed cohort at the boundary.
                // Stage that replacement after period 59; other modules must be
                // commissioned the month before their predecessors retire.
                let start = if asset == 23 && t == 60 {
                    60
                } else {
                    t - 1 - h.processes[build].lead_periods
                };
                next.plan[start].activities[build] += quantities[asset];
                if build == 27 {
                    next.plan[start - h.processes[35].lead_periods].activities[35] +=
                        quantities[asset];
                }
                replacement.push(json!({"asset":h.resources[asset],"quantity":quantities[asset],"start":start,"arrival":start+h.processes[build].lead_periods,"retirement":t}));
            }
        }
    }
    // Replace annual packaging use from month 60, allowing one month of fabrication lead.
    let mut packaging = original.plan[59].closing[10];
    let mut fabricated = 0;
    let target = next.model.seasonal.closing_floors[71][10];
    for t in 60..=71 {
        packaging += fabricated * h.processes[33].outputs[10];
        packaging -= next.plan[t]
            .activities
            .iter()
            .zip(&h.processes)
            .map(|(n, p)| n * p.inputs[10])
            .sum::<i64>();
        assert!(packaging >= 0);
        if t < 71 {
            let future_use: i64 = next.plan[t + 1]
                .activities
                .iter()
                .zip(&h.processes)
                .map(|(n, p)| n * p.inputs[10])
                .sum();
            fabricated = batches(
                (target + future_use - packaging).max(0),
                h.processes[33].outputs[10],
            );
            next.plan[t].activities[33] = fabricated;
        }
    }
    let fabrication: i64 = next.plan[60..].iter().map(|r| r.activities[33]).sum();
    let base = &original.plan[59].closing;
    let primary = &h.processes[42];
    let secondary = &h.processes[43];
    let reclaimed_opening = base[29] / secondary.inputs[29];
    let available = base[28] + reclaimed_opening * secondary.outputs[28];
    let deficit = (fabrication - available).max(0);
    // Find the smallest integer primary run that enables fabrication, including residue recovery.
    let supply = |runs: i64| {
        runs * primary.outputs[28]
            + (runs * primary.outputs[29] + base[29] % secondary.inputs[29]) / secondary.inputs[29]
                * secondary.outputs[28]
    };
    let max_primary = base[27] / primary.inputs[27];
    assert!(supply(max_primary) >= deficit);
    let (mut lo, mut hi) = (0, max_primary);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if supply(mid) >= deficit {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    assert!(lo > 0 && supply(lo - 1) < deficit);
    let mut variants = Vec::new();
    let mut controls = Value::Null;
    for (name, runs) in [
        ("use_remaining_packaging_ore", max_primary),
        ("minimize_packaging_ore", lo),
    ] {
        let mut candidate = next.clone();
        let p = &mut candidate.plan;
        p[60].activities[42] = runs;
        p[60].activities[43] = reclaimed_opening;
        p[62].activities[43] = batches(
            (deficit - runs * primary.outputs[28]).max(0),
            secondary.outputs[28],
        );
        // Fuel extracted today powers tomorrow's generation. Compute the upstream
        // energy costs before locking month 59's carried fuel floor.
        for t in (59..71).rev() {
            p[t].activities[30] = p[t + 1].activities[31];
            p[t].activities[31] = batches(
                energy_use(&p[t + 1], &h.processes),
                h.processes[31].outputs[9],
            );
        }
        p[58].activities[31] =
            batches(energy_use(&p[59], &h.processes), h.processes[31].outputs[9]);
        let boundary = p[58].activities[31] + p[59].activities[31];
        let extra_use =
            energy_use(&p[58], &h.processes) - energy_use(&original.plan[58], &h.processes);
        let extra = batches(boundary + extra_use, h.processes[31].outputs[9] - 1);
        p[57].activities[31] += extra;
        p[58].activities[30] += boundary + extra;
        balance_candidate(&candidate.model, p);
        for (t, row) in p.iter().enumerate() {
            for (r, q) in row.closing.iter().enumerate() {
                assert!(*q >= 0, "negative stock {t}/{r}: {q}");
            }
        }
        candidate.model.replay(p).unwrap();
        if runs == lo {
            // Rebalance every row after the perturbation so the verifier tests
            // the missing physical supply, not stale cached closing balances.
            let mut missed_sowing = p.clone();
            missed_sowing[63].activities[..3].fill(0);
            balance_candidate(&candidate.model, &mut missed_sowing);
            assert!(candidate.model.replay(&missed_sowing).is_err());
            let mut missed_renewal = p.clone();
            missed_renewal[62].activities[37] = 0;
            balance_candidate(&candidate.model, &mut missed_renewal);
            assert!(candidate.model.replay(&missed_renewal).is_err());
            controls = json!({"missed_sowing_rejected":true,"missed_power_replacement_rejected":true,
                "one_fewer_primary_batch_material_deficit_kg":deficit-supply(lo-1)});
        }
        let closing = &p[71].closing;
        let delivered: i64 = p[60..].iter().flat_map(|r| &r.deliveries[6..9]).sum();
        // Optimistic physical upper bound for another identical packaging-use
        // schedule: convert every remaining unit, ignoring all capacity and time limits.
        let remaining_primary = closing[27] / primary.inputs[27];
        let remaining_secondary =
            (closing[29] + remaining_primary * primary.outputs[29]) / secondary.inputs[29];
        let remaining_material = closing[28]
            + remaining_primary * primary.outputs[28]
            + remaining_secondary * secondary.outputs[28];
        let available_ticks =
            closing[10] - target + remaining_material * h.processes[33].outputs[10];
        let annual_use: i64 = p[60..]
            .iter()
            .map(|row| {
                row.activities
                    .iter()
                    .zip(&h.processes)
                    .map(|(n, proc)| n * proc.inputs[10])
                    .sum::<i64>()
            })
            .sum();
        assert!(annual_use > available_ticks);
        let mut result = json!({"policy":name,
            "annual_packaging_use_ticks":annual_use,"next_year_packaging_upper_bound_ticks":available_ticks,
            "next_year_packaging_shortfall_lower_bound_ticks":annual_use-available_ticks,"packaging_ore_extracted_kg":runs*primary.inputs[27],
            "remaining_packaging_ore_kg":closing[27],"packaging_waste_generated_g":closing[30]-base[30],
            "remaining_residue_kg":closing[29],"remaining_recovered_material_kg":closing[28],"remaining_fuel_deposit":closing[18],"closing_food_kg":closing[..9],"year_six_delivered_kg":delivered});
        let (e, allocations, draw, funded) = verify_funding(candidate, money, prior);
        result["money"] = json!({"opening":funded.opening_total,"suffix_events_settled":funded.settled_events,
            "unfunded_events_before_transfers":e.failed.len(),"treasury_draw":draw,"allocations":allocations});
        variants.push(result);
    }
    json!({"periods":72,"first_replanned_period":57,"terminal_food_floors_kg":next.model.seasonal.closing_floors[71][..9],
        "controls":controls,"sowing_period":63,"harvests":harvested,"replacement":replacement,"variants":variants})
}

fn energy_use(row: &PeriodPlan, recipes: &[Process]) -> i64 {
    row.activities
        .iter()
        .zip(recipes)
        .map(|(x, r)| x * r.inputs[9])
        .sum()
}

// Use the first model year as production history for a January delivery.
// Only the two January needs change. All other baseline deliveries stay due.
fn january_opening(
    definition: &ModelDefinition,
    original: &ModelInput,
    policy: &CoupledDefinition,
) -> Value {
    use qazna::{population::PopulationStudy, rolling::RollingCapital};
    const JAN: usize = 12;
    let study: PopulationStudy =
        serde_json::from_slice(&fs::read("fixtures/population-study.json").unwrap()).unwrap();
    let january = &study.project().unwrap().months[0];
    assert_eq!(january.calendar, "2027-01");
    let mut next = original.clone();
    next.boundary = JAN;
    let mut rows = Vec::new();
    let baseline = RollingCapital::checkpoint(&original.model, &original.plan, JAN).unwrap();
    for (item, crop, pack, ship, destination) in [
        ("rice_component", 0, 15, 17, 6),
        ("oats_component", 1, 19, 21, 7),
    ] {
        let demand = january.demands.iter().find(|d| d.item == item).unwrap();
        assert_eq!(demand.unit, definition.resources[destination].unit);
        let target: i64 = demand.central.as_ref().unwrap().parse().unwrap();
        let baseline_delivery = original.plan[JAN].deliveries[destination];
        let extra = target - baseline_delivery;
        assert!(extra > 0);
        let h = &mut next.model.seasonal.physical;
        let net_yield = h.processes[crop].outputs[crop] - h.processes[crop].inputs[crop];
        let starts = extra / net_yield + i64::from(extra % net_yield != 0);
        let rule = next
            .model
            .seasonal
            .crops
            .iter()
            .find(|rule| rule.process == crop)
            .unwrap();
        let sow = rule.sowing_periods[0];
        let dispatch = JAN - h.processes[ship].lead_periods;
        let preservation = dispatch - h.processes[pack].lead_periods;
        assert!(sow + h.processes[crop].lead_periods <= preservation);
        next.plan[sow].activities[crop] += starts;
        next.plan[preservation].activities[pack] += extra;
        next.plan[dispatch].activities[ship] += extra;
        next.plan[JAN].deliveries[destination] = target;
        h.periods[JAN].needs[destination] = target;
        let before = baseline.snapshot();
        let arrivals: i64 = before
            .pending
            .iter()
            .filter(|c| c.completes == JAN)
            .map(|c| c.activities * h.processes[c.process].outputs[destination])
            .sum();
        assert_eq!(
            before.opening_stock[destination] + arrivals,
            baseline_delivery
        );
        rows.push(json!({
            "item": item,
            "baseline_january_delivery_kg": baseline_delivery,
            "baseline_opening_shelf_kg": before.opening_stock[destination],
            "baseline_due_kg": arrivals,
            "january_target_kg": target,
            "extra_dispatch_kg": extra,
            "extra_hectares": starts * rule.hectares_per_activity,
            "extra_seed_kg": starts * h.processes[crop].inputs[crop],
            "extra_harvest_kg": starts * h.processes[crop].outputs[crop]
        }));
    }
    schedule_power(&next.model.seasonal.physical, &mut next.plan);
    balance_candidate(&next.model, &mut next.plan);
    let h = &next.model.seasonal.physical;
    verify_linear(
        &next.model.compile().unwrap(),
        &h.vector(&next.plan).unwrap(),
        None,
    )
    .unwrap();
    let verified = RollingCapital::checkpoint(&next.model, &next.plan, JAN).unwrap();
    for (row, r) in rows.iter_mut().zip([6, 7]) {
        let shelf = verified.snapshot().opening_stock[r];
        let pending: Vec<_> = verified
            .snapshot()
            .pending
            .iter()
            .filter(|c| c.completes == JAN && h.processes[c.process].outputs[r] > 0)
            .collect();
        let due: i64 = pending
            .iter()
            .map(|c| c.activities * h.processes[c.process].outputs[r])
            .sum();
        assert_eq!(shelf + due, next.plan[JAN].deliveries[r]);
        row["opening_shelf_kg"] = json!(shelf);
        row["shipments_due_kg"] = json!(due);
        row["commitments"] = json!(pending);
    }
    // A missing December shipment or counting a receipt as December stock must fail.
    let mut late = next.plan.clone();
    late[JAN - 1].activities[17] -= 1;
    balance_candidate(&next.model, &mut late);
    assert!(next.model.replay(&late).is_err());
    let mut twice = next.plan.clone();
    twice[JAN - 1].closing[6] += 1;
    assert!(next.model.replay(&twice).is_err());
    // The ledger replay starts in the predecessor year. The physical January
    // checkpoint above and the first unsettled cash deadline can differ.
    next.boundary = 0;
    let physical = qazna::model_scenarios::preview(definition.clone(), next.clone()).unwrap();
    let run = qazna::coupled::preview(policy.clone(), physical.clone(), vec![]).unwrap();
    let ledger: CoupledCheckpoint = serde_json::from_str(run.checkpoint.as_ref().unwrap()).unwrap();
    let e = ledger.evidence;
    let total = |values: &[String]| {
        values
            .iter()
            .map(|q| q.parse::<u128>().unwrap())
            .sum::<u128>()
    };
    assert_eq!(total(&e.money.closing).to_string(), e.money.opening_total);
    let cut = e.money.first_blocked_period.unwrap() as usize;
    let extra_fuel: i64 = next
        .plan
        .iter()
        .zip(&original.plan)
        .map(|(n, b)| n.activities[30] - b.activities[30])
        .sum();
    next.boundary = cut;
    let revision = qazna::model_scenarios::preview_continuation(physical, next).unwrap();
    let funded = qazna::coupled::preview_funded_continuation(run, revision).unwrap();
    let settled: CoupledCheckpoint =
        serde_json::from_str(funded.checkpoint.as_ref().unwrap()).unwrap();
    let revised = settled.evidence;
    assert!(revised.all_scheduled_payments_settled && revised.resident_zero_each_period);
    assert_eq!(revised.closing_by_period[..cut], e.closing_by_period[..cut]);
    assert_eq!(
        total(&revised.money.closing).to_string(),
        e.money.opening_total
    );
    let allocated: u128 = revised
        .treasury_allocations
        .iter()
        .map(|a| a.amount.parse::<u128>().unwrap())
        .sum();
    json!({
        "calendar_at_period_zero": "2026-01",
        "delivery_month": january.calendar,
        "january_index": JAN,
        "verified_periods": original.plan.len(),
        "items": rows,
        "extra_fuel_extracted": extra_fuel,
        "same_opening_cash": {
            "opening_total": e.money.opening_total,
            "closing_total": total(&e.money.closing).to_string(),
            "attempted_events": e.money.attempted_events,
            "settled_events": e.money.settled_events,
            "first_blocked_period": cut,
            "opening_deficits_by_account": e.opening_deficits,
            "all_settled": e.all_scheduled_payments_settled
        },
        "treasury_funded_variant": {
            "allocated": allocated.to_string(),
            "allocations": revised.treasury_allocations,
            "settled_events_including_allocations": revised.money.settled_events,
            "opening_total": revised.money.opening_total,
            "closing_total": total(&revised.money.closing).to_string()
        },
        "scope": "The first year is a modeled predecessor. The population target replaces two needs for one January only. Other baseline deliveries, initial stocks, capacities and reserve floors are retained."
    })
}
