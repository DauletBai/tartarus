//! Material balance for continued grain wrapping on the verified boundary.
use crate::{MONTHS, calendar};
use qazna::{model_scenarios::ModelInput, rolling::RollingSnapshot};
use serde::Serialize;
use serde_json::{Value, json};

#[path = "freight_energy.rs"]
mod freight_energy;

const YEAR: usize = 12;
const GRAMS_PER_KG: i64 = 1000;
const SORT_LOT_KG: i64 = 100;

#[derive(Clone)]
struct Conditions {
    freight: Option<Freight>,
    collection_cap_g: i64,
    ready: i64,
    reserve: i64,
    demand: [i64; YEAR],
    energy: i64,
    sort_cap_kg: [i64; YEAR],
    fab_cap_kg: [i64; YEAR],
    shared_slots: Vec<i64>,
}

// Compatible vehicles go from the receiving point back to a sorting site.
#[derive(Clone, Copy)]
struct Freight {
    compatible_per_10k: i64,
    blocked_month: Option<usize>,
    work: i64,
    work_per_kg: i64,
    energy_per_kg: i64,
}

const OUTBOUND_PAYLOAD_KG: i64 = 20_000;
const RETURN_PAYLOAD_KG: i64 = 10_000;

fn loads(quantity: i64, payload: i64) -> i64 {
    quantity / payload + i64::from(quantity % payload != 0)
}

#[derive(Default, Serialize)]
struct FreightMonth {
    dispatch_kg: i64,
    outbound_trips: i64,
    arriving_grain_trips: i64,
    eligible_return_trips: i64,
    loaded_return_trips: i64,
    arriving_at_sorter_g: i64,
    dispatch_energy_ticks: i64,
    return_energy_ticks: i64,
    extra_dispatch_energy_ticks: i64,
    closing_energy_ticks: i64,
    transport_work_ticks: i64,
}

#[derive(Default, Serialize)]
struct Month {
    month: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    freight: Option<FreightMonth>,
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
    let mut return_in_transit = 0;
    let mut outbound_in_transit = 0;
    for t in 0..c.shared_slots.len() {
        let season = t % YEAR;
        let mut collected = if t >= first && c.shared_slots[t] > 0 {
            (used * capture_percent / 100).min(c.collection_cap_g)
        } else {
            0
        };
        let mut slots = c.shared_slots[t];
        let mut freight = None;
        if let Some(route) = c.freight {
            let dispatch = c.demand[season];
            let trips = loads(dispatch, OUTBOUND_PAYLOAD_KG);
            let eligible = if route.blocked_month == Some(t) {
                0
            } else {
                outbound_in_transit * route.compatible_per_10k / 10_000
            };
            let outbound_work = dispatch * route.work_per_kg;
            assert!(outbound_work <= route.work);
            let outbound_energy = dispatch * route.energy_per_kg;
            let extra_energy = (outbound_energy - energy).max(0);
            energy += extra_energy - outbound_energy;
            let remaining_work = route.work - outbound_work;
            let capacity_kg = (eligible * RETURN_PAYLOAD_KG)
                .min(energy / route.energy_per_kg)
                .min(remaining_work / route.work_per_kg);
            collected = collected.min(capacity_kg * GRAMS_PER_KG);
            let billed_kg = loads(collected, GRAMS_PER_KG);
            let return_energy = billed_kg * route.energy_per_kg;
            energy -= return_energy;
            let transport_work = outbound_work + billed_kg * route.work_per_kg;
            slots = slots.min(route.work - transport_work);
            waiting += return_in_transit;
            freight = Some(FreightMonth {
                dispatch_kg: dispatch,
                outbound_trips: trips,
                arriving_grain_trips: outbound_in_transit,
                eligible_return_trips: eligible,
                loaded_return_trips: loads(billed_kg, RETURN_PAYLOAD_KG),
                arriving_at_sorter_g: return_in_transit,
                dispatch_energy_ticks: outbound_energy,
                return_energy_ticks: return_energy,
                extra_dispatch_energy_ticks: extra_energy,
                transport_work_ticks: transport_work,
                ..Default::default()
            });
            outbound_in_transit = trips;
            return_in_transit = collected;
        } else {
            waiting += collected;
        }
        waste += used - collected;
        material += sorting_due;
        let arrivals = fabrication_due * GRAMS_PER_KG;
        ready += arrivals;

        // Process available clean material first. One kg needs one fabrication slot.
        // A 100 kg sorting lot is charged as twenty five-kg recovery batches.
        let fab = material.min(c.fab_cap_kg[season]).min(slots).min(energy);
        material -= fab;
        let lots = (waiting / (SORT_LOT_KG * GRAMS_PER_KG))
            .min(c.sort_cap_kg[season] / SORT_LOT_KG)
            .min((slots - fab) / 20)
            .min((energy - fab) / 20);
        let sorted = lots * SORT_LOT_KG;
        waiting -= sorted * GRAMS_PER_KG;
        sorting_due = lots * yield_kg_per_lot;
        waste += (sorted - sorting_due) * GRAMS_PER_KG;
        fabrication_due = fab;
        let effort = fab + lots * 20;
        energy -= effort;
        if let Some(row) = &mut freight {
            row.closing_energy_ticks = energy;
        }

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
                + return_in_transit
                + waiting
                + waste
                + GRAMS_PER_KG * (material + sorting_due + fabrication_due)
        );
        months.push(Month {
            month: calendar(MONTHS + t),
            freight,
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
        freight: None,
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
    for j in [17, 21, 25] {
        assert_eq!(h.processes[j].inputs[9], 1);
        assert_eq!(work.activity_use[j], 4);
        assert!(
            last_year
                .iter()
                .all(|row| row.needs[6 + (j - 17) / 4] <= row.capacities[j])
        );
    }
    let mut freight_cases = Vec::new();
    // Keep the same 95% collection, 80% yield and December 2032 launch across comparisons.
    let launch = 11;
    for (share, blocked) in [
        (0, None),
        (10, None),
        (20, None),
        (25, None),
        (25, Some(launch)),
    ] {
        let route = Conditions {
            freight: Some(Freight {
                compatible_per_10k: share,
                blocked_month: blocked,
                work: opening[work.resource] * work.units_per_stock,
                work_per_kg: work.activity_use[17],
                energy_per_kg: h.processes[17].inputs[9],
            }),
            ..c.clone()
        };
        let run = simulate(&route, 95, 80, launch);
        let latest = (1..horizon)
            .rev()
            .find(|&start| simulate(&route, 95, 80, start).extra == 0);
        let trips = run.months.iter().map(|m| m.freight.as_ref().unwrap());
        let extra_energy: i64 = trips.clone().map(|m| m.extra_dispatch_energy_ticks).sum();
        let energy_used: i64 = run
            .months
            .iter()
            .map(|m| {
                let f = m.freight.as_ref().unwrap();
                f.dispatch_energy_ticks + f.return_energy_ticks + m.energy_used_ticks
            })
            .sum();
        assert_eq!(
            c.energy + extra_energy,
            energy_used + trips.clone().next_back().unwrap().closing_energy_ticks
        );
        let reference = share == 25 && blocked.is_none();
        let revised = if reference {
            latest.map(|t| {
                let revised = simulate(&route, 95, 80, t);
                assert_eq!(revised.extra, 0);
                json!({"start":calendar(MONTHS+t), "extra_ready_g":revised.extra, "energy_supply":freight_energy::study(input,boundary,&revised.months), "months":revised.months})
            })
        } else {
            None
        };
        freight_cases.push(json!({
            "compatible_per_10000":share, "blocked_return_month":blocked.map(|t|calendar(MONTHS+t)),
            "extra_ready_g":run.extra,"latest_start_without_extra_wrapping":latest.map(|t|calendar(MONTHS+t)),
            "first_extra_dispatch_energy":run.months.iter().find(|m|m.freight.as_ref().unwrap().extra_dispatch_energy_ticks>0).map(|m|&m.month),
            "extra_dispatch_energy_ticks":extra_energy,
            "peak_loaded_return_trips":trips.map(|m|m.loaded_return_trips).max(),
            "revised_collection":revised,
            "months":if reference {Some(run.months)} else {None}
        }));
    }
    let mut remaining = c.ready;
    let first_exhaustion = c.demand.iter().cycle().take(horizon).position(|&use_g| {
        remaining -= use_g;
        remaining < 0
    });
    json!({
        "basis": "2032–2037 wrapping workload repeats the twelve 2031 monthly grain quantities ordered. Grain production, shared capacity reservations for other sectors and subsequent payments are outside this balance. The separate grain_return_freight comparison charges outbound and return transport.",
        "opening_ready_g": c.ready, "operating_reserve_g": c.reserve, "opening_energy_ticks": c.energy,
        "historical_waste_excluded_g": opening[30], "opening_packaging_ore_kg": opening[27],
        "opening_recovered_kg": opening[28], "opening_residue_kg": opening[29],
        "annual_use_g": c.demand.iter().sum::<i64>(),
        "first_unfilled_month_without_reserve_or_recovery": first_exhaustion.map(|t| calendar(MONTHS + t)),
        "first_month_without_active_factory": c.shared_slots.iter().position(|&q| q == 0).map(|t| calendar(MONTHS+t)),
        "grain_return_freight": {
            "basis":"Monthly grain dispatch repeats 2031 orders. Each grain trip arrives next month and provides at most one return from that destination. A compatible return carries at most 10 tonnes; it arrives for sorting next month. Loaded return kg use the manual dispatch energy and labour coefficients as a sensitivity. Extra outbound energy is a diagnostic requirement.",
            "outbound_payload_kg":OUTBOUND_PAYLOAD_KG, "return_payload_kg":RETURN_PAYLOAD_KG,
            "collection_start":calendar(MONTHS+launch), "capture_percent":95, "clean_yield_percent":80,
            "cases":freight_cases
        },
        "end_month": calendar(MONTHS+horizon-1), "cases": cases
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vehicles_and_wrappers_arrive_before_they_can_be_used_again() {
        let route = Freight {
            compatible_per_10k: 10_000,
            blocked_month: None,
            work: 200_000_000,
            work_per_kg: 4,
            energy_per_kg: 1,
        };
        let mut c = Conditions {
            freight: Some(route),
            collection_cap_g: i64::MAX,
            ready: 20_000_000,
            reserve: 0,
            demand: [0; YEAR],
            energy: 100_000_000,
            sort_cap_kg: [100_000; YEAR],
            fab_cap_kg: [100_000; YEAR],
            shared_slots: vec![100_000; 6],
        };
        c.demand[0] = 10_000_001;
        let run = simulate(&c, 100, 100, 1);
        let trips = |t: usize| run.months[t].freight.as_ref().unwrap();
        assert_eq!(trips(0).eligible_return_trips, 0);
        assert_eq!(
            (trips(1).outbound_trips, trips(1).arriving_grain_trips),
            (0, 501)
        );
        assert_eq!(trips(1).loaded_return_trips, 2);
        assert_eq!(trips(1).return_energy_ticks, 10_001);
        assert_eq!(
            (run.months[1].sorted_kg, run.months[2].sorted_kg),
            (0, 10_000)
        );
        assert_eq!(run.months[4].arriving_wrappers_g, 10_000_000);
        c.freight = Some(Freight {
            blocked_month: Some(1),
            ..route
        });
        assert_eq!(simulate(&c, 100, 100, 1).months[2].sorted_kg, 0);
        c.energy = 0;
        let limited = simulate(&c, 100, 100, 1);
        assert_eq!(
            limited.months[0]
                .freight
                .as_ref()
                .unwrap()
                .extra_dispatch_energy_ticks,
            10_000_001
        );
    }

    #[test]
    fn losses_round_down_and_late_output_cannot_cover_earlier_use() {
        let mut c = Conditions {
            freight: None,
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
