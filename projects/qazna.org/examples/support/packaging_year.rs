//! Material balance for continued grain wrapping on the verified boundary.
use crate::{MONTHS, calendar};
use qazna::{model_scenarios::ModelInput, rolling::RollingSnapshot};
use serde::Serialize;
use serde_json::{Value, json};

const YEAR: usize = 12;
const GRAMS_PER_KG: i64 = 1000;
const SORT_LOT_KG: i64 = 100;

#[derive(Clone)]
struct Conditions {
    collection_cap_g: i64,
    ready: i64,
    reserve: i64,
    demand: [i64; YEAR],
    energy: i64,
    sort_cap_kg: [i64; YEAR],
    fab_cap_kg: [i64; YEAR],
    shared_slots: Vec<i64>,
}

#[derive(Default, Serialize)]
struct Month {
    month: String,
    use_g: i64,
    collected_g: i64,
    sorted_kg: i64,
    fabricated_kg: i64,
    arriving_wrappers_g: i64,
    extra_ready_g: i64,
    closing_ready_g: i64,
    waiting_to_sort_g: i64,
    sorted_material_kg: i64,
    sorting_due_kg: i64,
    fabrication_due_kg: i64,
    unrecoverable_to_date_g: i64,
    energy_used_ticks: i64,
    shared_work_and_factory_slots: i64,
}

struct MaterialRun {
    months: Vec<Month>,
    extra: i64,
}

// A used wrapper becomes available to collect next month. Sorting and fabrication
// each take a further month. Output due in January stays in the closing pipeline.
fn simulate(
    c: &Conditions,
    capture_percent: i64,
    yield_kg_per_lot: i64,
    first: usize,
) -> MaterialRun {
    let mut ready = c.ready;
    let mut used: i64 = 0;
    let mut waste = 0;
    let mut waiting = 0;
    let mut material = 0;
    let mut sorting_due = 0;
    let mut fabrication_due = 0;
    let mut energy = c.energy;
    let mut extra = 0;
    let mut months = Vec::new();
    for t in 0..c.shared_slots.len() {
        let season = t % YEAR;
        let collected = if t >= first && c.shared_slots[t] > 0 {
            (used * capture_percent / 100).min(c.collection_cap_g)
        } else {
            0
        };
        waste += used - collected;
        waiting += collected;
        material += sorting_due;
        let arrivals = fabrication_due * GRAMS_PER_KG;
        ready += arrivals;

        // Process available clean material first. One kg needs one fabrication slot.
        // A 100 kg sorting lot is charged as twenty five-kg recovery batches.
        let fab = material
            .min(c.fab_cap_kg[season])
            .min(c.shared_slots[t])
            .min(energy);
        material -= fab;
        let lots = (waiting / (SORT_LOT_KG * GRAMS_PER_KG))
            .min(c.sort_cap_kg[season] / SORT_LOT_KG)
            .min((c.shared_slots[t] - fab) / 20)
            .min((energy - fab) / 20);
        let sorted = lots * SORT_LOT_KG;
        waiting -= sorted * GRAMS_PER_KG;
        sorting_due = lots * yield_kg_per_lot;
        waste += (sorted - sorting_due) * GRAMS_PER_KG;
        fabrication_due = fab;
        let effort = fab + lots * 20;
        energy -= effort;

        used = c.demand[season];
        let gap = (c.reserve + used - ready).max(0);
        extra += gap;
        ready += gap - used;
        // The diagnostic extra supply is explicit so a missed order cannot generate
        // a fictitious stream of used packaging in subsequent months.
        assert_eq!(
            c.ready + extra,
            ready
                + used
                + waiting
                + waste
                + GRAMS_PER_KG * (material + sorting_due + fabrication_due)
        );
        months.push(Month {
            month: calendar(MONTHS + t),
            use_g: used,
            collected_g: collected,
            sorted_kg: sorted,
            fabricated_kg: fab,
            arriving_wrappers_g: arrivals,
            extra_ready_g: gap,
            closing_ready_g: ready,
            waiting_to_sort_g: waiting,
            sorted_material_kg: material,
            sorting_due_kg: sorting_due,
            fabrication_due_kg: fabrication_due,
            unrecoverable_to_date_g: waste,
            energy_used_ticks: effort,
            shared_work_and_factory_slots: effort,
        });
    }
    MaterialRun { months, extra }
}

pub(super) fn study(input: &ModelInput, boundary: &RollingSnapshot) -> Value {
    let s = &input.model.seasonal;
    let h = &s.physical;
    assert_eq!(boundary.boundary, MONTHS);
    assert!(boundary.pending.is_empty());
    let opening = &boundary.opening_stock;
    assert_eq!(opening[27..30], [0, 0, 0]);
    for j in [15, 19, 23] {
        assert_eq!(
            (h.processes[j].inputs[10], h.processes[j].outputs[30]),
            (1, 1)
        );
    }
    let fab = &h.processes[33];
    let recovery = &h.processes[43];
    assert_eq!(
        (fab.inputs[28], fab.outputs[10], fab.lead_periods),
        (1, 1000, 1)
    );
    assert_eq!(
        (
            recovery.inputs[29],
            recovery.outputs[28],
            recovery.lead_periods
        ),
        (5, 2, 1)
    );
    let factory = input
        .model
        .limits
        .iter()
        .find(|l| l.name == "shared_factory")
        .unwrap();
    let work = input
        .model
        .limits
        .iter()
        .find(|l| l.name == "shared_human_work_ticks")
        .unwrap();
    for j in [33, 43] {
        assert_eq!(
            (
                work.activity_use[j],
                factory.activity_use[j],
                h.processes[j].inputs[9]
            ),
            (1, 1, 1)
        );
    }

    let last_year = &h.periods[MONTHS - YEAR..];
    let horizon = 6 * YEAR;
    let c = Conditions {
        collection_cap_g: i64::MAX,
        ready: opening[10],
        reserve: s.closing_floors.last().unwrap()[10],
        energy: opening[9],
        demand: std::array::from_fn(|t| last_year[t].needs[6..9].iter().sum()),
        sort_cap_kg: std::array::from_fn(|t| last_year[t].capacities[43] * recovery.inputs[29]),
        fab_cap_kg: std::array::from_fn(|t| last_year[t].capacities[33]),
        shared_slots: (0..horizon)
            .map(|t| {
                let active: i64 = boundary
                    .opening_cohorts
                    .iter()
                    .filter(|v| v.resource == factory.resource && v.expires > MONTHS + t)
                    .map(|v| v.quantity)
                    .sum();
                (active * factory.units_per_stock)
                    .min(opening[work.resource] * work.units_per_stock)
            })
            .collect(),
    };
    let trials = [(0, 0), (100, 40), (95, 80), (98, 95)];
    let mut cases = Vec::new();
    for (capture, yield_kg) in trials {
        let latest = (1..horizon)
            .rev()
            .find(|&t| simulate(&c, capture, yield_kg, t).extra == 0);
        let later_gap = latest
            .filter(|&t| t + 1 < horizon)
            .map(|t| simulate(&c, capture, yield_kg, t + 1).extra);
        let scheduled = simulate(&c, capture, yield_kg, latest.unwrap_or(1));
        let peak = scheduled
            .months
            .iter()
            .map(|r| r.collected_g)
            .max()
            .unwrap();
        let freight = Conditions {
            collection_cap_g: peak * 9 / 10,
            ..c.clone()
        };
        let capped = simulate(&freight, capture, yield_kg, latest.unwrap_or(1));
        cases.push(json!({
            "capture_percent": capture, "recovered_kg_per_100_kg": yield_kg,
            "extra_ready_g": scheduled.extra, "first_extra_month": scheduled.months.iter().find(|r| r.extra_ready_g > 0).map(|r| &r.month),
            "latest_collection_start_without_extra": latest.map(|t| calendar(MONTHS + t)),
            "extra_g_if_latest_start_delayed_one_month": later_gap,
            "peak_collection_g_per_month": peak,
            "collection_capacity_stress": {"cap_g_per_month": freight.collection_cap_g, "extra_ready_g": capped.extra},
            "months": scheduled.months
        }));
    }
    let mut remaining = c.ready;
    let first_exhaustion = c.demand.iter().cycle().take(horizon).position(|&use_g| {
        remaining -= use_g;
        remaining < 0
    });
    json!({
        "basis": "2032–2037 wrapping workload repeats the twelve 2031 monthly grain quantities ordered. Grain production, shared capacity reservations for other sectors, collection transport and subsequent payments are outside this balance.",
        "opening_ready_g": c.ready, "operating_reserve_g": c.reserve, "opening_energy_ticks": c.energy,
        "historical_waste_excluded_g": opening[30], "opening_packaging_ore_kg": opening[27],
        "opening_recovered_kg": opening[28], "opening_residue_kg": opening[29],
        "annual_use_g": c.demand.iter().sum::<i64>(),
        "first_unfilled_month_without_reserve_or_recovery": first_exhaustion.map(|t| calendar(MONTHS + t)),
        "first_month_without_active_factory": c.shared_slots.iter().position(|&q| q == 0).map(|t| calendar(MONTHS+t)),
        "end_month": calendar(MONTHS+horizon-1), "cases": cases
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn losses_round_down_and_late_output_cannot_cover_earlier_use() {
        let mut c = Conditions {
            collection_cap_g: i64::MAX,
            ready: 300_003,
            reserve: 0,
            demand: [100_001; YEAR],
            energy: 10_000,
            sort_cap_kg: [100; YEAR],
            fab_cap_kg: [100; YEAR],
            shared_slots: vec![200; YEAR],
        };
        let r = simulate(&c, 100, 100, 1);
        assert_eq!(r.months[2].arriving_wrappers_g, 0);
        assert_eq!(r.months[3].arriving_wrappers_g, 100_000);
        assert_eq!(r.months[3].extra_ready_g, 1);
        let late = simulate(&c, 100, 100, 2);
        assert_eq!(late.months[3].extra_ready_g, 100_001);
        assert_eq!(late.months[1].unrecoverable_to_date_g, 100_001);
        c.shared_slots[1] = 19;
        assert_eq!(simulate(&c, 100, 95, 1).months[1].sorted_kg, 0);
        c.energy = 0;
        assert_eq!(simulate(&c, 100, 100, 1).months[3].arriving_wrappers_g, 0);
    }
}
