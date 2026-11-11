//! A bounded upstream/finite-capital extension of the existing automation graph.
use qazna::{
    capital::*,
    horizon::{PeriodPlan, Process, StockCapacity},
    mathematics::verify_linear,
    seasonal::Seasonal,
};
use serde_json::{Value, json};
const R: usize = 27;
const J: usize = 42;
const ENERGY: usize = 9;
const PACK: usize = 10;
const SPARE: usize = 11;
const KIT: usize = 12;
const PLACE: usize = 13;
const ROBOT: usize = 14;
const ORE_DEPOSIT: usize = 17;
const FUEL_DEPOSIT: usize = 18;
const ORE: usize = 19;
const FUEL: usize = 20;
const MATERIAL: usize = 21;
const POWER: usize = 22;
const FACTORY: usize = 23;
const STORE: usize = 24;
const SCHOOL: usize = 25;
const EXTRACTOR: usize = 26;
fn recipe(inputs: &[(usize, i64)], resource: usize, output: i64, lag: usize) -> Process {
    let mut p = Process {
        inputs: vec![0; R],
        outputs: vec![0; R],
        lead_periods: lag,
    };
    for &(k, n) in inputs {
        p.inputs[k] = n;
    }
    p.outputs[resource] = output;
    p
}
fn build(base: &Value, replacement: bool) -> (CapitalCycle, Vec<PeriodPlan>) {
    let mut s: Seasonal = serde_json::from_value(base["model"].clone()).unwrap();
    let mut plans: Vec<PeriodPlan> = serde_json::from_value(base["candidate"].clone()).unwrap();
    let mut limits: Vec<StockCapacity> =
        serde_json::from_value(base["shared_capacities"].clone()).unwrap();
    let h = &mut s.physical;
    h.resources.extend(
        [
            "finite_ore_deposit_ticks",
            "finite_fuel_deposit_ticks",
            "extracted_ore_ticks",
            "extracted_fuel_ticks",
            "processed_material_ticks",
            "power_modules",
            "factory_modules",
            "warehouse_modules",
            "training_centres",
            "extraction_modules",
        ]
        .map(str::to_string),
    );
    h.initial.resize(R, 0);
    h.storage_caps.resize(R, 10_000_000_000_000);
    for (k, n) in [
        (ENERGY, 30_000_000_000),
        (PACK, 8_000_000_000),
        (SPARE, 12_000_000_000),
        (KIT, 600),
        (PLACE, 600),
        (ORE_DEPOSIT, 4_000_000_000),
        (FUEL_DEPOSIT, 4_000_000_000),
        (ORE, 50_000_000),
        (FUEL, 60_000_000),
        (MATERIAL, 100_000_000),
        (POWER, 20),
        (FACTORY, 20),
        (STORE, 20),
        (SCHOOL, 20),
        (EXTRACTOR, 20),
    ] {
        h.initial[k] = n;
    }
    for p in &mut h.processes {
        p.inputs.resize(R, 0);
        p.outputs.resize(R, 0);
    }
    h.processes.extend([
        recipe(&[(ORE_DEPOSIT, 1), (ENERGY, 1)], ORE, 1, 1),
        recipe(&[(FUEL_DEPOSIT, 1), (ENERGY, 1)], FUEL, 1, 1),
        recipe(&[(FUEL, 1)], ENERGY, 1000, 1),
        recipe(&[(ORE, 1), (ENERGY, 1)], MATERIAL, 1, 1),
        recipe(&[(MATERIAL, 1), (ENERGY, 1)], PACK, 1000, 1),
        recipe(&[(MATERIAL, 1), (ENERGY, 1)], SPARE, 1000, 1),
        recipe(&[(MATERIAL, 1000), (ENERGY, 1000)], KIT, 1, 2),
        recipe(&[(MATERIAL, 10), (ENERGY, 1000)], PLACE, 1, 1),
        recipe(&[(MATERIAL, 10000), (ENERGY, 1_000_000)], POWER, 1, 3),
        recipe(&[(MATERIAL, 10000), (ENERGY, 1_000_000)], FACTORY, 1, 3),
        recipe(&[(MATERIAL, 10000), (ENERGY, 1_000_000)], STORE, 1, 3),
        recipe(&[(MATERIAL, 10000), (ENERGY, 1_000_000)], SCHOOL, 1, 3),
        recipe(&[(MATERIAL, 10000), (ENERGY, 1_000_000)], EXTRACTOR, 1, 3),
    ]);
    for p in &mut h.periods {
        p.supplies = vec![0; R];
        p.needs.resize(R, 0);
        p.capacities.resize(J, 100_000_000);
    }
    for f in &mut s.closing_floors {
        f.resize(R, 0);
    }
    for k in [
        PACK, SPARE, ORE, FUEL, MATERIAL, POWER, FACTORY, STORE, SCHOOL, EXTRACTOR,
    ] {
        s.closing_floors[59][k] = h.initial[k];
    }
    for l in &mut limits {
        l.activity_use.resize(J, 0);
    }
    for j in 29..J {
        limits[0].activity_use[j] = 1;
    }
    for j in 35..J {
        limits[0].activity_use[j] = 1000;
    }
    for (name, resource, capacity, indices) in [
        ("fuel_generation", POWER, 2_000_000, vec![31]),
        (
            "shared_factory",
            FACTORY,
            5_000_000,
            vec![32, 33, 34, 35, 37, 38, 39, 40, 41],
        ),
        ("voluntary_intakes", SCHOOL, 100, vec![36]),
        ("shared_extraction", EXTRACTOR, 5_000_000, vec![29, 30]),
    ] {
        let mut use_of = vec![0; J];
        for j in indices {
            use_of[j] = 1;
        }
        limits.push(StockCapacity {
            name: name.into(),
            resource,
            units_per_stock: capacity,
            activity_use: use_of,
        });
    }
    for p in &mut plans {
        p.activities.resize(J, 0);
        p.deliveries.resize(R, 0);
        p.closing.resize(R, 0);
        p.discarded.resize(R, 0);
    }
    // Replacement orders cover known service-life expiries, never future failures.
    if replacement {
        for t in 24..60 {
            plans[t].activities[27] += plans[t - 24].activities[27];
        }
    }
    for t in 0..60 {
        if t < 59 {
            plans[t].activities[29] = 25_000_000;
            plans[t].activities[30] = 30_000_000;
            plans[t].activities[31] = 30_000_000;
            plans[t].activities[32] = 25_000_000;
            plans[t].activities[33] = 8_000_000;
            plans[t].activities[34] = 12_000_000;
        }
        if t + 2 < 60 {
            plans[t].activities[35] = plans[t + 2].activities[27];
        }
        if t + 1 < 60 {
            plans[t].activities[36] = plans[t + 1].activities[28];
        }
    }
    if replacement {
        for (process, starts) in [
            (37, vec![14, 38]),
            (38, vec![20, 56]),
            (39, vec![26]),
            (40, vec![20, 50]),
            (41, vec![16, 46]),
        ] {
            for t in starts {
                plans[t].activities[process] = 20;
            }
        }
    }
    let lives = [
        (ROBOT, 24, 24),
        (POWER, 24, 18),
        (FACTORY, 36, 24),
        (STORE, 36, 30),
        (SCHOOL, 30, 24),
        (EXTRACTOR, 30, 20),
    ]
    .map(
        |(resource, service_periods, initial_remaining_periods)| AssetLife {
            resource,
            service_periods,
            initial_remaining_periods,
        },
    )
    .to_vec();
    let m = CapitalCycle {
        seasonal: s,
        limits,
        lives,
        stores: vec![StoragePool {
            name: "shared_grain_and_preserved_storage".into(),
            asset_resource: STORE,
            units_per_asset: 5_000_000_000,
            stored_resources: (0..9).collect(),
        }],
    };
    (m, plans)
}
fn balances(m: &CapitalCycle, p: &mut [PeriodPlan]) {
    let h = &m.seasonal.physical;
    let retire = m.retirements(p).unwrap();
    let mut stock = h.initial.clone();
    let mut arrivals = vec![vec![0; R]; p.len()];
    for (t, row) in p.iter_mut().enumerate() {
        for k in 0..R {
            stock[k] += arrivals[t][k] + h.periods[t].supplies[k];
        }
        for (j, r) in h.processes.iter().enumerate() {
            let x = row.activities[j];
            if x == 0 {
                continue;
            }
            assert!(t + r.lead_periods < 60);
            for k in 0..R {
                stock[k] -= x * r.inputs[k];
                arrivals[t + r.lead_periods][k] += x * r.outputs[k];
            }
        }
        row.discarded = retire[t].clone();
        for (k, s) in stock.iter_mut().enumerate() {
            *s -= row.deliveries[k] + row.discarded[k];
        }
        // Electricity is consumed within its output period; none is banked across months.
        row.discarded[ENERGY] = stock[ENERGY].max(0);
        stock[ENERGY] -= row.discarded[ENERGY];
        // Preserve only explicitly specified input buffers, including the next opening stock.
        for k in [PACK, SPARE, KIT, PLACE] {
            let target = if t == 59 { h.initial[k] } else { 0 };
            let excess = (stock[k] - target).max(0);
            row.discarded[k] = excess;
            stock[k] -= excess;
        }
        row.closing = stock.clone();
    }
}
// Bounded integer fixed point for this recipe's 1000:1 fuel-to-energy ratio.
// Work backwards over known needs, then restore the first-period fuel draw at
// the terminal receipt. It is not a generic optimizer or a forecast of outages.
fn match_generation(m: &CapitalCycle, p: &mut [PeriodPlan]) {
    for _ in 0..64 {
        let old: Vec<_> = p
            .iter()
            .map(|r| (r.activities[30], r.activities[31]))
            .collect();
        let opening_draw = p[0].activities[31];
        for t in (0..59).rev() {
            p[t].activities[30] = p[t + 1].activities[31] + if t == 58 { opening_draw } else { 0 };
            let need: i64 = p[t + 1]
                .activities
                .iter()
                .zip(&m.seasonal.physical.processes)
                .map(|(&x, r)| x * r.inputs[ENERGY])
                .sum();
            p[t].activities[31] = need / 1000 + i64::from(need % 1000 != 0);
        }
        if p.iter()
            .map(|r| (r.activities[30], r.activities[31]))
            .eq(old)
        {
            return;
        }
    }
    panic!("synthetic fuel/generation iteration did not converge");
}
fn case(name: &str, m: CapitalCycle, mut p: Vec<PeriodPlan>) -> Value {
    balances(&m, &mut p);
    let matrix = m.compile().unwrap();
    let vector = m.seasonal.physical.vector(&p).unwrap();
    let exact = verify_linear(&matrix, &vector, None);
    let replay = m.replay(&p);
    assert_eq!(exact.is_ok(), replay.is_ok());
    let dated = super::automation_study::dated(&m.seasonal, &p, 18);
    let uses: Vec<i64> = (0..R)
        .map(|k| {
            p.iter()
                .flat_map(|row| row.activities.iter().zip(&m.seasonal.physical.processes))
                .map(|(&x, r)| x * r.inputs[k])
                .sum()
        })
        .collect();
    let deficits: Vec<_> = p
        .iter()
        .enumerate()
        .flat_map(|(t, row)| {
            row.closing
                .iter()
                .enumerate()
                .filter(|(_, v)| **v < 0)
                .map(move |(r, &v)| json!({"month":t,"resource":r,"gap":-v}))
        })
        .collect();
    let retire = m.retirements(&p).unwrap();
    let mut capability_gaps = Vec::new();
    let mut storage_gaps = Vec::new();
    for (t, row) in p.iter().enumerate() {
        let opening = if t == 0 {
            &m.seasonal.physical.initial
        } else {
            &p[t - 1].closing
        };
        for limit in &m.limits {
            let used: i64 = row
                .activities
                .iter()
                .zip(&limit.activity_use)
                .map(|(x, a)| x * a)
                .sum();
            let available =
                (opening[limit.resource] - retire[t][limit.resource]) * limit.units_per_stock;
            if used > available {
                capability_gaps.push(json!({"month":t,"capacity":limit.name,"used":used,"available":available,"gap":used-available}));
            }
        }
        for store in &m.stores {
            let held: i64 = store.stored_resources.iter().map(|&k| row.closing[k]).sum();
            let available = (opening[store.asset_resource] - retire[t][store.asset_resource])
                * store.units_per_asset;
            if held > available {
                storage_gaps.push(json!({"month":t,"store":store.name,"held":held,"available":available,"gap":held-available}));
            }
        }
    }

    json!({"name":name,"feasible":exact.is_ok()&&dated.is_ok(),"matrix_rows":matrix.rows.len(),"matrix_columns":matrix.objective.len(),
        "exact_matrix":exact.map_err(|e|e.to_string()),"independent_replay":replay.map_err(|e|e.to_string()),"dated_food_replay":dated.map_err(|e|e.to_string()),
        "planned_total_inputs_by_resource":uses,"negative_stock_gaps":deficits,"active_capability_gaps":capability_gaps,"constructed_storage_gaps":storage_gaps,"retirements":retire,"curtailed_energy_ticks":p.iter().map(|r|r.discarded[ENERGY]).sum::<i64>(),
        "monthly_planned_energy_use":p.iter().map(|row|row.activities.iter().zip(&m.seasonal.physical.processes).map(|(&x,r)|x*r.inputs[ENERGY]).sum::<i64>()).collect::<Vec<_>>(),
        "model":m,"candidate":p})
}
pub fn study(automation: &Value) -> Value {
    let base = &automation["cases"][3];
    let mut cases = Vec::new();
    let (m, p) = build(base, true);
    cases.push(case("endogenous_upstream_with_replacement", m, p));
    let (mut m, mut p) = build(base, true);
    m.seasonal.physical.initial[FUEL_DEPOSIT] = 1_000_000_000;
    match_generation(&m, &mut p);
    cases.push(case("demand_matched_power_with_smaller_fuel_deposit", m, p));
    let (m, p) = build(base, false);
    cases.push(case("no_capital_replacement", m, p));
    let (mut m, p) = build(base, true);
    m.seasonal.physical.initial[FUEL_DEPOSIT] = 1_000_000_000;
    cases.push(case("finite_fuel_deposit_exhaustion", m, p));
    let (mut m, p) = build(base, true);
    m.seasonal.physical.periods[14].capacities[37] = 0;
    cases.push(case("first_power_replacement_unavailable", m, p));
    let (m, mut p) = build(base, true);
    p[15].activities[37] = p[14].activities[37];
    p[14].activities[37] = 0;
    cases.push(case("one_month_late_power_replacement", m, p));
    let (m, mut p) = build(base, true);
    p[26].activities[39] = 0;
    cases.push(case("warehouse_replacement_omitted", m, p));
    let (m, mut p) = build(base, true);
    p[10].activities[35] = 0;
    cases.push(case("robot_kit_manufacture_omitted", m, p));
    let (mut m, p) = build(base, true);
    m.limits
        .iter_mut()
        .find(|l| l.name == "shared_factory")
        .unwrap()
        .units_per_stock = 1_000_000;
    cases.push(case("shared_factory_bottleneck", m, p));
    let (mut m, p) = build(base, true);
    for t in 30..=32 {
        m.seasonal.physical.periods[t].capacities[31] = 0;
    }
    cases.push(case("three_month_generation_outage_fixed_plan", m, p));
    json!({"kind":"finite_deposit_upstream_and_capital_reproduction","months":60,"model_calls":0,"cases":cases,
        "scope":["Same rapid automation food demand and production schedule as schema 13, extended rather than substituted",
        "No recurring boundary supplies of energy, packaging, upkeep material, kits or training places; finite initial deposits, assets and operating buffers remain explicit endowments",
        "Synthetic ore/fuel extraction, generation, refining, packaging, upkeep, robot-kit manufacture and training intake share finite installed capabilities and human work",
        "Three-month construction, calendar service life and start-of-period retirement govern power, factories, warehouses, training centres, extraction modules and robots",
        "Assets can operate only after their completion period; deterministic expiry is exact, not statistical reliability or measured engineering wear",
        "No electricity inventory is carried between months: initial energy is a stated first-month output endowment; subsequent generation output is available one month after fuel commitment, and unused output is curtailed",
        "Month-aggregated fuel commitment/generation is not an electrical dispatch or grid-stability simulation; first-month endowment, intramonth peaks, efficiency and network losses require engineering calibration",
        "Food original expiry and terminal raw/pipeline stocks are retained; food storage is shared across nine grain/preserved/destination resources and expires with its warehouse modules",
        "Deposits are finite and depleted by extraction; no private ownership, commodity sale or money creation is implied",
        "Rejected-case traces describe proposed schedules, not delivered goods. Diagnostic gaps can reveal earlier failures than matrix row ordering",
        "All upstream coefficients use synthetic ticks, not kWh/tonnes/labour hours or public budget amounts. Identity food routes and 18-month life remain assumptions",
        "This is a fixed-candidate feasibility test, not optimization, a causal controller for unknown failures, or a claim that all real supply chains have closed",
        "Foreign trade, renewable fuel cycles, transport of upstream goods, emissions, asset recycling, unplanned repair and post-horizon rolling commitments remain open",
        "Retirements beyond month 59 are not certified by a stock floor; repeated rolling-horizon continuity requires a separate gate. No runtime or monetary authority changes"]})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Value {
        super::super::automation_study::study(&super::super::seasonal_study::study(
            &super::super::nutrition_study::study(),
        ))
    }
    #[test]
    fn upstream_closure_and_failure_candidates_agree() {
        let r = study(&base());
        let c = r["cases"].as_array().unwrap();
        assert_eq!(c[0]["feasible"], true, "{}", c[0]["independent_replay"]);
        assert_eq!(c[1]["feasible"], true, "{}", c[1]["independent_replay"]);
        for case in &c[2..] {
            assert_eq!(case["feasible"], false, "{}", case["name"]);
        }
        let m: CapitalCycle = serde_json::from_value(c[0]["model"].clone()).unwrap();
        assert!(
            m.seasonal
                .physical
                .periods
                .iter()
                .all(|p| p.supplies.iter().all(|&x| x == 0))
        );
        let p: Vec<PeriodPlan> = serde_json::from_value(c[0]["candidate"].clone()).unwrap();
        assert!(p.iter().all(|r| r.closing[ENERGY] == 0));
        assert_eq!(p[59].closing[FUEL_DEPOSIT], 4_000_000_000 - 59 * 30_000_000);
        assert!(p[59].closing[ROBOT] >= 3000);
    }
    #[test]
    fn demand_following_restores_fuel_buffer_and_exposes_rounding() {
        let a = base();
        let (m, mut p) = build(&a["cases"][3], true);
        match_generation(&m, &mut p);
        balances(&m, &mut p);
        assert!(m.replay(&p).is_ok());
        assert_eq!(p[59].closing[FUEL], m.seasonal.physical.initial[FUEL]);
        for row in &p[1..] {
            assert!((0..1000).contains(&row.discarded[ENERGY]));
        }
        let extraction: i64 = p.iter().map(|r| r.activities[30]).sum();
        assert!(extraction < 1_000_000_000);
        let once = p.iter().map(|r| r.activities.clone()).collect::<Vec<_>>();
        match_generation(&m, &mut p);
        assert_eq!(
            once,
            p.iter().map(|r| r.activities.clone()).collect::<Vec<_>>()
        );
    }
    #[test]
    fn failure_cases_keep_the_same_food_entitlements() {
        let r = study(&base());
        let c = r["cases"].as_array().unwrap();
        for case in c {
            assert_eq!(
                case["model"]["seasonal"]["physical"]["periods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["needs"].clone())
                    .collect::<Vec<_>>(),
                c[0]["model"]["seasonal"]["physical"]["periods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["needs"].clone())
                    .collect::<Vec<_>>()
            );
        }
    }
}
