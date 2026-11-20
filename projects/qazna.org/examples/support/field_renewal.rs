//! Joint life-cycle, grain and treasury study for the expanded rotation calendar.
use super::{FIELD_PROCESSES, MONTHS, calendar, source_period};
use qazna::{
    coupled::{self, CoupledCheckpoint, CoupledDefinition},
    horizon::PeriodPlan,
    model_scenarios::{self, ModelDefinition, ModelInput},
    rolling::RollingCapital,
    seasonal::Seasonal,
};
use serde_json::{Value, json};
use uuid::Uuid;

#[path = "packaging_year.rs"]
mod packaging_year;

const CONSTRUCTION: std::ops::RangeInclusive<usize> = 37..=41;

fn assemble(
    original: &ModelInput,
    fields: &Seasonal,
    grain: &[PeriodPlan],
    starts: Vec<Vec<i64>>,
) -> ModelInput {
    let mut input = original.clone();
    input.boundary = 0;
    let s = &mut input.model.seasonal;
    s.land.clone_from(&fields.land);
    s.irrigation_m3.clone_from(&fields.irrigation_m3);
    s.crops.clone_from(&fields.crops);
    // Retain the 44 reference recipe and payment identifiers. Three spare
    // annual sowing lanes remain registered for dated-food audit, at zero capacity.
    for lane in FIELD_PROCESSES..15 {
        let mut dormant = s.crops[lane % 3].clone();
        dormant.process = lane;
        s.crops.push(dormant);
    }
    let reference = &original.model.seasonal;
    let h = &mut s.physical;
    for j in 0..FIELD_PROCESSES {
        h.processes[j].outputs[..9].copy_from_slice(&fields.physical.processes[j].outputs);
    }
    h.periods = (0..MONTHS)
        .map(|t| {
            let mut row = reference.physical.periods[source_period(t)].clone();
            row.capacities[..FIELD_PROCESSES]
                .copy_from_slice(&fields.physical.periods[t].capacities[..FIELD_PROCESSES]);
            row.capacities[FIELD_PROCESSES..15].fill(0);
            row.needs[..9].copy_from_slice(&grain[t].deliveries);
            row
        })
        .collect();
    s.closing_floors = (0..MONTHS)
        .map(|t| reference.closing_floors[source_period(t)].clone())
        .collect();
    input.plan = starts
        .into_iter()
        .enumerate()
        .map(|(t, mut activities)| {
            activities[..FIELD_PROCESSES].copy_from_slice(&grain[t].activities[..FIELD_PROCESSES]);
            for j in CONSTRUCTION {
                activities[j] = 0;
            }
            PeriodPlan {
                activities,
                deliveries: h.periods[t].needs.clone(),
                closing: vec![0; h.resources.len()],
                discarded: vec![0; h.resources.len()],
            }
        })
        .collect();
    input
}

// Schedule a whole module at the last possible start. Its completion can
// serve opening capacity only in the month after its three construction periods.
fn plan_modules(input: &mut ModelInput, grain: &[PeriodPlan]) {
    let m = &input.model;
    let s = &m.seasonal;
    let h = &s.physical;
    // Plan factories last: projects for the other four asset types consume
    // factory slots. Their construction never consumes those four capabilities.
    for j in [37, 39, 40, 41, 38] {
        let recipe = &h.processes[j];
        let r = recipe.outputs.iter().position(|&v| v > 0).unwrap();
        assert_eq!(recipe.outputs[r], 1);
        let life = m.lives.iter().find(|l| l.resource == r).unwrap();
        let delay = recipe.lead_periods + 1;
        let mut active = [0; MONTHS];
        active[..life.initial_remaining_periods].fill(h.initial[r]);
        // Constructing factories can itself require factory capacity. Revisit
        // earlier months until every scheduled start has an available builder.
        loop {
            let mut changed = false;
            for t in 0..MONTHS {
                let mut required = s.closing_floors[t][r]; // protect the final operating capacity
                for l in m.limits.iter().filter(|l| l.resource == r) {
                    let used: i64 = input.plan[t]
                        .activities
                        .iter()
                        .zip(&l.activity_use)
                        .map(|(a, k)| a * k)
                        .sum();
                    required = required.max(ceil_units(used, l.units_per_stock));
                }
                for store in m.stores.iter().filter(|l| l.asset_resource == r) {
                    let kg: i64 = store
                        .stored_resources
                        .iter()
                        .map(|&k| grain[t].closing[k])
                        .sum();
                    required = required.max(ceil_units(kg, store.units_per_asset));
                }
                let additional = required - active[t];
                if additional > 0 {
                    let start = t
                        .checked_sub(delay)
                        .expect("insufficient opening capacity before earliest commissioning");
                    input.plan[start].activities[j] += additional;
                    let end = MONTHS.min(t + life.service_periods);
                    for q in &mut active[t..end] {
                        *q += additional;
                    }
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
}

fn ceil_units(need: i64, per_unit: i64) -> i64 {
    need / per_unit + i64::from(need % per_unit != 0)
}

fn balance(input: &mut ModelInput) {
    let h = &input.model.seasonal.physical;
    let retire = input.model.retirements(&input.plan).unwrap();
    let mut inventory = h.initial.clone();
    let mut arrivals = vec![vec![0; inventory.len()]; MONTHS];
    for (t, row) in input.plan.iter_mut().enumerate() {
        row.discarded.clone_from(&retire[t]);
        for (r, q) in inventory.iter_mut().enumerate() {
            *q += arrivals[t][r] + h.periods[t].supplies[r] - row.deliveries[r] - row.discarded[r];
        }
        for (j, recipe) in h.processes.iter().enumerate() {
            let q = row.activities[j];
            if q == 0 {
                continue;
            }
            let arriving = &mut arrivals[t + recipe.lead_periods];
            for (r, stock) in inventory.iter_mut().enumerate() {
                *stock -= q * recipe.inputs[r];
                arriving[r] += q * recipe.outputs[r];
            }
        }
        row.closing.clone_from(&inventory);
    }
}

fn total(values: &[String]) -> u128 {
    values.iter().map(|v| v.parse::<u128>().unwrap()).sum()
}

pub(super) fn study(
    original: &ModelInput,
    fields: &Seasonal,
    grain: &[PeriodPlan],
    starts: Vec<Vec<i64>>,
    fixture: &Value,
) -> Value {
    let mut input = assemble(original, fields, grain, starts);
    plan_modules(&mut input, grain);
    balance(&mut input);
    let cap = input.model.replay(&input.plan).unwrap();
    for (t, row) in input.plan.iter().enumerate() {
        assert_eq!(row.closing[..9], grain[t].closing);
    }
    let dates = input
        .food_routes
        .as_ref()
        .unwrap()
        .audit(&input.model, &input.plan)
        .unwrap();
    let mut definition: ModelDefinition =
        serde_json::from_value(fixture["definition"].clone()).unwrap();
    definition.time_origin = "2026-01".into();
    definition.basis="72-month rotation and upstream renewal, including one predecessor year and 60 population demand months.".into();
    let money_def = CoupledDefinition {
        id: Uuid::nil(),
        format_version: 1,
        model_id: definition.id,
        policy: serde_json::from_value(fixture["policy"].clone()).unwrap(),
        basis: "Unchanged 60-month reference account balances and measures applied once at the start of the expanded calendar.".into(),
    };
    let physical = model_scenarios::preview(definition, input.clone()).unwrap();
    let trial = coupled::preview(money_def, physical.clone(), vec![]).unwrap();
    let checkpoint: CoupledCheckpoint =
        serde_json::from_str(trial.checkpoint.as_deref().unwrap()).unwrap();
    let e = checkpoint.evidence;
    assert_eq!(total(&e.money.closing).to_string(), e.money.opening_total);
    let built: Vec<i64> = CONSTRUCTION
        .map(|j| input.plan.iter().map(|p| p.activities[j]).sum())
        .collect();
    let construction: Vec<_> = input
        .plan
        .iter()
        .enumerate()
        .filter_map(|(t, row)| {
            let counts = &row.activities[37..42];
            counts
                .iter()
                .any(|&q| q > 0)
                .then(|| json!({"start":calendar(t),"modules":counts}))
        })
        .collect();
    let terminal = RollingCapital::checkpoint(&input.model, &input.plan, MONTHS).unwrap();
    let packaging_2032 = packaging_year::study(&input, terminal.snapshot());
    let cohorts: Vec<_> = terminal
        .snapshot()
        .opening_cohorts
        .iter()
        .filter(|c| (22..27).contains(&c.resource))
        .map(|c| {
            json!({
                "asset": input.model.seasonal.physical.resources[c.resource],
                "quantity": c.quantity,
                "retires": calendar(c.expires)
            })
        })
        .collect();
    let cut = e
        .money
        .first_blocked_period
        .expect("baseline liquidity failure") as usize;
    let treasury_before = &e.closing_by_period[cut - 1][0];
    input.boundary = cut;
    let revision = model_scenarios::preview_continuation(physical, input).unwrap();
    let funded = coupled::preview_funded_continuation(trial, revision).unwrap();
    let funded_checkpoint: CoupledCheckpoint =
        serde_json::from_str(funded.checkpoint.as_deref().unwrap()).unwrap();
    let after = funded_checkpoint.evidence;
    assert!(after.all_scheduled_payments_settled && after.resident_zero_each_period);
    assert_eq!(e.closing_by_period[..cut], after.closing_by_period[..cut]);
    assert_eq!(
        total(&after.money.closing).to_string(),
        e.money.opening_total
    );
    let allocation: u128 = after
        .treasury_allocations
        .iter()
        .map(|a| a.amount.parse::<u128>().unwrap())
        .sum();
    json!({
        "physical_months": MONTHS,
        "packaging_2032": packaging_2032,
        "food_expired_kg": dates.expired,
        "closing_food_in_transit_kg": dates.final_in_transit,
        "total_modules_built": built,
        "module_order": ["power", "factory", "warehouse", "training", "extraction"],
        "construction": construction,
        "post_horizon_retirements": cohorts,
        "pending_starts_at_end": terminal.snapshot().pending.len(),
        "terminal_operating_modules": cap.available_opening_stock_by_period.last().unwrap()[22..27],
        "money": {
            "opening_total": e.money.opening_total,
            "closing_total": total(&e.money.closing).to_string(),
            "first_blocked_month": calendar(cut),
            "events": e.money.attempted_events,
            "settled": e.money.settled_events,
            "opening_deficits": e.opening_deficits,
            "funded_continuation": {
                "month": calendar(cut),
                "treasury_before": treasury_before,
                "allocation_total": allocation.to_string(),
                "allocations": after.treasury_allocations,
                "events": after.money.attempted_events,
                "settled": after.money.settled_events,
                "all_settled": after.all_scheduled_payments_settled,
                "closing_total": total(&after.money.closing).to_string()
            },
            "minimum_opening_total_for_schedule": e.bound.total_minimum_opening,
            "all_settled": e.all_scheduled_payments_settled
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build, candidate, capped_packaging, targets};
    use qazna::population::PopulationStudy;

    #[test]
    fn one_month_late_is_rejected_even_with_the_same_number_of_modules() {
        let fixture: Value =
            serde_json::from_slice(include_bytes!("../../fixtures/extended-scenario.json"))
                .unwrap();
        let original: ModelInput = serde_json::from_value(fixture["input"].clone()).unwrap();
        let population: PopulationStudy =
            serde_json::from_slice(include_bytes!("../../fixtures/population-study.json")).unwrap();
        let needs = targets(&original, &population);
        let mut fields = build(&original, &needs, 100);
        let (grain, _, _) = candidate(&mut fields);
        // The production case requests enough water on its explicitly expanded land.
        let water = crate::stage_water(&fields, &grain);
        for (cap, need) in fields.irrigation_m3.iter_mut().zip(water) {
            *cap = (*cap).max(need);
        }
        let (_, starts) = capped_packaging(&original, &grain);
        let mut input = assemble(&original, &fields, &grain, starts);
        plan_modules(&mut input, &grain);
        balance(&mut input);
        input.model.replay(&input.plan).unwrap();
        for (lane, expected) in [
            (37, "active_capacity:fuel_generation:18"),
            (39, "active_storage:shared_grain_and_preserved_storage:30"),
        ] {
            let mut late = input.clone();
            let t = late
                .plan
                .iter()
                .position(|p| p.activities[lane] > 0)
                .unwrap();
            late.plan[t].activities[lane] -= 1;
            late.plan[t + 1].activities[lane] += 1;
            balance(&mut late);
            let error = late.model.replay(&late.plan).unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
}
