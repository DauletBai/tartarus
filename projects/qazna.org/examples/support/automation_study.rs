//! Conditional automation scenarios, coupled to the existing seasonal crop graph.
//! Engineering coefficients are synthetic, not forecasts of robot capabilities.
use qazna::{
    horizon::{PeriodPlan, Process, StockCapacity},
    lifecycle::DatedStock,
    mathematics::{MathError, verify_linear},
    seasonal::Seasonal,
};
use serde_json::{Value, json};
const R: usize = 17;
const J: usize = 29;
const ENERGY: usize = 9;
const PACK: usize = 10;
const SPARE: usize = 11;
const KIT: usize = 12;
const PLACE: usize = 13;
const ROBOT: usize = 14;
const SKILL: usize = 15;
const WORK: usize = 16;
const CELL: i64 = 5_000_000;
const WORK_BUDGET: i64 = 90_000_000_000;
fn process(inputs: &[(usize, i64)], outputs: &[(usize, i64)], lag: usize) -> Process {
    let mut p = Process {
        inputs: vec![0; R],
        outputs: vec![0; R],
        lead_periods: lag,
    };
    for &(k, n) in inputs {
        p.inputs[k] = n;
    }
    for &(k, n) in outputs {
        p.outputs[k] = n;
    }
    p
}
fn limits() -> Vec<StockCapacity> {
    let mut auto = vec![0; J];
    let mut work = vec![0; J];
    for k in 0..3 {
        let j = 15 + 4 * k;
        work[j..j + 4].copy_from_slice(&[8, 2, 4, 1]);
        auto[j + 1] = 1;
        auto[j + 3] = 1;
    }
    work[27] = 10_000;
    work[28] = 10_000;
    vec![
        StockCapacity {
            name: "shared_human_work_ticks".into(),
            resource: WORK,
            units_per_stock: 1,
            activity_use: work,
        },
        StockCapacity {
            name: "commissioned_robot_cells".into(),
            resource: ROBOT,
            units_per_stock: CELL,
            activity_use: auto.clone(),
        },
        StockCapacity {
            name: "qualified_available_crews".into(),
            resource: SKILL,
            units_per_stock: CELL,
            activity_use: auto,
        },
    ]
}
fn build(
    base: &Value,
    deployment: i64,
    shares: [i64; 5],
    outage: bool,
) -> (Seasonal, Vec<PeriodPlan>) {
    let mut m: Seasonal = serde_json::from_value(base["model"].clone()).unwrap();
    let old: Vec<PeriodPlan> = serde_json::from_value(base["candidate"].clone()).unwrap();
    let demand: Vec<Vec<i64>> = m.physical.periods.iter().map(|p| p.needs.clone()).collect();
    let h = &mut m.physical;
    h.resources.extend(
        [
            "preserved_rice_gross_equivalent_kg",
            "preserved_oats_gross_equivalent_kg",
            "preserved_feed_equivalent_kg",
            "delivered_rice_gross_equivalent_kg",
            "delivered_oats_gross_equivalent_kg",
            "delivered_feed_equivalent_kg",
            "energy_ticks",
            "packaging_ticks",
            "upkeep_material_ticks",
            "robot_cell_kits",
            "voluntary_training_places",
            "commissioned_cells",
            "qualified_available_crews",
            "available_human_work_ticks",
        ]
        .map(str::to_string),
    );
    h.initial.resize(R, 0);
    h.storage_caps.resize(R, 10_000_000_000_000);
    h.initial[3..6].copy_from_slice(&demand[1]);
    h.initial[6..9].copy_from_slice(&demand[0]);
    h.initial[WORK] = WORK_BUDGET;
    for p in &mut h.processes {
        p.inputs.resize(R, 0);
        p.outputs.resize(R, 0);
    }
    for k in 0..3 {
        // Identity gross-equivalent routes deliberately avoid claiming a milling yield.
        h.processes
            .push(process(&[(k, 1), (ENERGY, 1), (PACK, 1)], &[(3 + k, 1)], 1));
        h.processes.push(process(
            &[(k, 1), (ENERGY, 2), (PACK, 1), (SPARE, 1)],
            &[(3 + k, 1)],
            1,
        ));
        h.processes
            .push(process(&[(3 + k, 1), (ENERGY, 1)], &[(6 + k, 1)], 1));
        h.processes.push(process(
            &[(3 + k, 1), (ENERGY, 2), (SPARE, 1)],
            &[(6 + k, 1)],
            1,
        ));
    }
    h.processes
        .push(process(&[(KIT, 1), (ENERGY, 1_000_000)], &[(ROBOT, 1)], 3));
    h.processes
        .push(process(&[(PLACE, 1), (ENERGY, 100_000)], &[(SKILL, 1)], 6));
    for (t, p) in h.periods.iter_mut().enumerate() {
        p.needs = vec![0; R];
        p.needs[6..9].copy_from_slice(&demand[t]);
        p.supplies = vec![0; R];
        p.supplies[ENERGY] = 30_000_000_000;
        p.supplies[PACK] = 8_000_000_000;
        p.supplies[SPARE] = 12_000_000_000;
        if t % 12 == 0 {
            p.supplies[KIT] = deployment;
            p.supplies[PLACE] = deployment;
        }
        p.capacities.resize(J, 10_000_000_000);
        // Planned annual line shutdown plus a common software/control outage.
        if t % 12 == 11 || (outage && (30..=32).contains(&t)) {
            for k in 0..3 {
                p.capacities[16 + 4 * k] = 0;
                p.capacities[18 + 4 * k] = 0;
            }
        }
    }
    for row in &mut m.closing_floors {
        row.resize(R, 0);
    }
    m.closing_floors[59][3..9].copy_from_slice(&h.initial[3..9]);
    let cap = limits();
    let mut stock = h.initial.clone();
    let mut receipts = vec![vec![0; R]; 60];
    let mut plans = Vec::new();
    for t in 0..60 {
        // Opening capacities do not include this period's completions.
        let mut robot_budget = stock[ROBOT].min(stock[SKILL]) * CELL;
        let mut x = old[t].activities.clone();
        x.resize(J, 0);
        if t % 12 == 0 {
            x[27] = deployment;
            x[28] = deployment;
        }
        for (k, _) in demand[0].iter().enumerate() {
            let shipping = if t < 59 {
                demand[t + 1][k] + if t == 58 { demand[0][k] } else { 0 }
            } else {
                0
            };
            let processing = if t < 58 {
                demand[t + 2][k] + if t == 57 { demand[0][k] } else { 0 }
            } else if t == 58 {
                demand[1][k]
            } else {
                0
            };
            for (offset, quantity) in [(0, processing), (2, shipping)] {
                let j = 15 + 4 * k + offset;
                let automated = (quantity * shares[t / 12] / 100)
                    .min(robot_budget)
                    .min(h.periods[t].capacities[j + 1]);
                x[j] = quantity - automated;
                x[j + 1] = automated;
                robot_budget -= automated;
            }
        }
        for k in 0..R {
            stock[k] += receipts[t][k] + h.periods[t].supplies[k];
        }
        for (j, p) in h.processes.iter().enumerate() {
            if x[j] == 0 {
                continue;
            }
            assert!(t + p.lead_periods < 60);
            for k in 0..R {
                stock[k] -= x[j] * p.inputs[k];
                receipts[t + p.lead_periods][k] += x[j] * p.outputs[k];
            }
        }
        let mut discarded = vec![0; R];
        // Unused external per-period energy and consumable budgets are exposed,
        // not silently carried as unlimited free inventory.
        for k in ENERGY..=PLACE {
            discarded[k] = stock[k].max(0);
            stock[k] -= discarded[k];
        }
        for (s, n) in stock.iter_mut().zip(&h.periods[t].needs) {
            *s -= n;
        }
        plans.push(PeriodPlan {
            activities: x,
            deliveries: h.periods[t].needs.clone(),
            closing: stock.clone(),
            discarded,
        });
    }
    assert_eq!(cap[0].activity_use.len(), J);
    (m, plans)
}
/// FEFO companion of this example's fixed identity processing/transport routes.
/// Original expiry survives both legs; preservation is not assumed to renew it.
fn dated(m: &Seasonal, plan: &[PeriodPlan], life: u32) -> Result<Value, MathError> {
    m.physical.vector(plan)?;
    if plan.iter().any(|p| {
        p.activities
            .iter()
            .chain(&p.deliveries)
            .chain(&p.closing[..9])
            .any(|&x| x < 0)
    }) {
        return Err(MathError::Invalid("negative food lifecycle quantity"));
    }
    let mut stocks = vec![DatedStock::default(); 9];
    for (k, s) in stocks.iter_mut().enumerate() {
        s.add(life, m.physical.initial[k] as u64)?;
    }
    let mut pending = vec![Vec::<(usize, u32, u64)>::new(); 60];
    let mut total_delivery = 0u64;
    for (t, p) in plan.iter().enumerate() {
        for s in &mut stocks {
            if s.expire(t as u32) > 0 {
                return Err(MathError::Infeasible(format!("dated_expiry:{t}")));
            }
        }
        for &(k, expiry, n) in &pending[t] {
            if expiry <= t as u32 {
                return Err(MathError::Infeasible(format!("transit_expiry:{t}")));
            }
            stocks[k].add(expiry, n)?;
        }
        for j in 0..15 {
            let a = p.activities[j] as u64;
            if a == 0 {
                continue;
            }
            let k = j % 3;
            let recipe = &m.physical.processes[j];
            stocks[k].take(a * recipe.inputs[k] as u64, t as u32)?;
            let arrival = t + recipe.lead_periods;
            pending[arrival].push((k, arrival as u32 + life, a * recipe.outputs[k] as u64));
        }
        for k in 0..3 {
            let j = 15 + 4 * k;
            for (source, target, quantity) in [
                (k, 3 + k, p.activities[j] + p.activities[j + 1]),
                (3 + k, 6 + k, p.activities[j + 2] + p.activities[j + 3]),
            ] {
                if quantity == 0 {
                    continue;
                }
                for (expiry, n) in stocks[source].take(quantity as u64, (t + 1) as u32)? {
                    pending[t + 1].push((target, expiry, n));
                }
            }
            stocks[6 + k].take(p.deliveries[6 + k] as u64, t as u32)?;
            total_delivery += p.deliveries[6 + k] as u64;
        }
        for (k, s) in stocks.iter().enumerate() {
            if s.total() != p.closing[k] as u64 {
                return Err(MathError::Infeasible(format!("dated_stock:{t}:{k}")));
            }
        }
    }
    Ok(
        json!({"full_declared_delivery_gross_equivalent_kg":total_delivery,"expiry_loss_kg":0,
        "expiry_renewed_in_processing_or_transport":false,"assumed_remaining_life_months":life}),
    )
}
fn case(name: &str, m: Seasonal, mut p: Vec<PeriodPlan>) -> Value {
    // Recompute diagnostic balances under changed boundary assumptions, retaining
    // the proposed activities. A shortage stays negative, never fictitiously filled.
    let mut stock = m.physical.initial.clone();
    let mut receipts = vec![vec![0; R]; p.len()];
    for (t, row) in p.iter_mut().enumerate() {
        for k in 0..R {
            stock[k] += receipts[t][k] + m.physical.periods[t].supplies[k];
        }
        for (j, recipe) in m.physical.processes.iter().enumerate() {
            let x = row.activities[j];
            if x == 0 {
                continue;
            }
            for k in 0..R {
                stock[k] -= x * recipe.inputs[k];
                receipts[t + recipe.lead_periods][k] += x * recipe.outputs[k];
            }
        }
        row.discarded.fill(0);
        for (k, s) in stock.iter_mut().enumerate().take(PLACE + 1).skip(ENERGY) {
            row.discarded[k] = (*s).max(0);
            *s -= row.discarded[k];
        }
        for (s, n) in stock.iter_mut().zip(&row.deliveries) {
            *s -= n;
        }
        row.closing = stock.clone();
    }
    let limits = limits();
    let matrix = m.compile_with_capabilities(&limits).unwrap();
    let check = verify_linear(&matrix, &m.physical.vector(&p).unwrap(), None);
    let replay = m.replay_with_capabilities(&p, &limits);
    assert_eq!(check.is_ok(), replay.is_ok());
    let lots = dated(&m, &p, 18);
    let yearly:Vec<Value>=(0..5).map(|y| {
        let rows=&p[y*12..y*12+12];
        let use_of=|k:usize| rows.iter().map(|r| r.activities.iter().zip(&m.physical.processes).map(|(&x,pr)| x*pr.inputs[k]).sum::<i64>()).sum::<i64>();
        let work:i64=rows.iter().map(|r|r.activities.iter().zip(&limits[0].activity_use).map(|(x,a)|x*a).sum::<i64>()).sum();
        let auto:i64=rows.iter().map(|r|r.activities.iter().zip(&limits[1].activity_use).map(|(x,a)|x*a).sum::<i64>()).sum();
        let delivered:Vec<i64>=(6..9).map(|k|rows.iter().map(|r|r.deliveries[k]).sum()).collect();
        json!({"year_index":y+1,"human_work_ticks":work,"energy_ticks":use_of(ENERGY),"upkeep_material_ticks":use_of(SPARE),
            "automated_route_kg":auto,"total_route_kg":rows.iter().map(|r|r.activities[15..27].iter().sum::<i64>()).sum::<i64>(),"installed_cells_at_close":rows[11].closing[ROBOT],"qualified_crews_at_close":rows[11].closing[SKILL],
            "delivered_gross_equivalent_kg":delivered})
    }).collect();
    json!({"name":name,"feasible":check.is_ok()&&lots.is_ok(),"exact_matrix":check.map_err(|e|e.to_string()),
        "independent_replay":replay.map_err(|e|e.to_string()),"dated_replay":lots.map_err(|e|e.to_string()),
        "rows":matrix.rows.len(),"columns":matrix.objective.len(),"annual":yearly,"model":m,"shared_capacities":limits,"candidate":p})
}
pub fn study(seasonal: &Value) -> Value {
    let base = &seasonal["cases"][0];
    let mut cases = Vec::new();
    for (name, rate, shares, outage) in [
        ("manual_reference", 0, [0; 5], false),
        ("slow_adoption", 100, [10, 15, 30, 40, 50], false),
        ("reference_adoption", 300, [20, 25, 45, 65, 80], false),
        ("rapid_adoption", 600, [30, 40, 65, 85, 95], false),
        (
            "rapid_common_control_outage_manual_fallback",
            600,
            [30, 40, 65, 85, 95],
            true,
        ),
    ] {
        let (m, p) = build(base, rate, shares, outage);
        cases.push(case(name, m, p));
    }
    let (mut m, p) = build(base, 600, [30, 40, 65, 85, 95], false);
    // Keep the same proposed plan; expose its failure under tighter boundaries.
    m.physical.periods[40].supplies[ENERGY] = 1;
    cases.push(case("rapid_plan_energy_budget_cut", m, p));
    let (mut m, p) = build(base, 600, [30, 40, 65, 85, 95], false);
    m.physical.periods[0].capacities[28] = 0;
    cases.push(case("rapid_plan_first_training_intake_unavailable", m, p));
    let (mut m, p) = build(base, 600, [30, 40, 65, 85, 95], true);
    for t in 30..=32 {
        for k in 0..3 {
            m.physical.periods[t].capacities[15 + 4 * k] = 0;
            m.physical.periods[t].capacities[17 + 4 * k] = 0;
        }
    }
    cases.push(case("manual_fallback_capacity_removed", m, p));
    let (mut m, p) = build(base, 600, [30, 40, 65, 85, 95], false);
    m.physical.periods[0].supplies[KIT] = 0;
    cases.push(case("rapid_plan_initial_robot_kits_missing", m, p));
    let (mut m, p) = build(base, 600, [30, 40, 65, 85, 95], false);
    for period in &mut m.physical.periods {
        period.supplies[PACK] = 5_000_000_000;
    }
    cases.push(case(
        "average_packaging_budget_misses_terminal_replenishment",
        m,
        p,
    ));
    let (mut m, p) = build(base, 600, [30, 40, 65, 85, 95], false);
    m.physical.periods[40].supplies[SPARE] = 0;
    cases.push(case("rapid_plan_upkeep_material_missing", m, p));
    let (m, p) = build(base, 600, [30, 40, 65, 85, 95], false);
    json!({"kind":"conditional_automation_seasonal_processing_and_transport","population_from_order_snapshot":100_000_000,
        "months":60,"model_calls":0,"evidence_review_date":"2026-11-11",
        "method_context_sources":["https://ifr.org/","https://www.ilo.org/resource/news/one-four-jobs-risk-being-transformed-genai-new-ilo%E2%80%93nask-global-index-shows","https://www.iea.org/reports/key-questions-on-energy-and-ai/executive-summary"],
        "cases":cases,"short_shelf_life_counterexample":dated(&m,&p,2).map_err(|e|e.to_string()),
        "shelf_life_sensitivity": ([6u32,9,12,15,18,24].into_iter().map(|life|json!({"assumed_months":life,"replay":dated(&m,&p,life).map_err(|e|e.to_string())})).collect::<Vec<_>>()),
        "scope":["Same dated needs for all adoption scenarios; no reduction in entitlement from reduced work",
        "One 60-month graph: seeds/land/irrigation/harvest, two one-month downstream routes, energy, packaging, upkeep material, cell installation and consenting training capacity",
        "18-month preserved-stock assumption is synthetic, NOT a certified shelf life; original expiry is never reset by either route",
        "Rice/oats/feed remain gross-equivalent units. Identity routes are not measured milling yields, complete diets, household cooking or animal rations",
        "Starting preserved/delivered pipeline stocks are explicit additional endowments; both pipeline stocks and original raw reserves must be replenished at horizon close",
        "A cell is a synthetic aggregate processing/logistics module, NOT an individual robot. Work/energy/material ticks are not people/hours/kWh or money",
        "Commissioning takes three months, qualification six, plus next-period availability; shared module and crew throughput is five million route-kg/month",
        "Adoption percentages and deployment rates are scenarios, not IFR/ILO/IEA forecasts. One common control outage and annual shutdown test manual fallback",
        "Per-use upkeep material is charged; equipment age/replacement, measured service durations, full raw-material and grid supply chains remain open",
        "External energy/material/kit/training-place budgets are declared equally across comparable adoption runs except scenario-specific deployment kits/places; unused budgets are disclosed as discard, not stored energy",
        "No AI is required to calculate or verify this study. AI-enabled operational capacity is a conditional scenario, not an autonomous ledger actor",
        "No new payments or emission are authorized. Workload reduction is possible time released for people, not a forecast of forced unemployment or government savings"]})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Value {
        super::super::seasonal_study::study(&super::super::nutrition_study::study())
    }
    #[test]
    fn equal_provision_lower_work_and_explicit_resource_tradeoffs() {
        let s = study(&base());
        let c = s["cases"].as_array().unwrap();
        for case in &c[..5] {
            assert_eq!(case["feasible"], true, "{}", case["name"]);
        }
        for case in &c[5..] {
            assert_eq!(case["feasible"], false);
        }
        for year in 0..5 {
            for case in &c[..5] {
                assert_eq!(
                    case["annual"][year]["delivered_gross_equivalent_kg"],
                    c[0]["annual"][year]["delivered_gross_equivalent_kg"]
                );
            }
        }
        assert!(
            c[3]["annual"][4]["human_work_ticks"].as_i64().unwrap()
                < c[0]["annual"][4]["human_work_ticks"].as_i64().unwrap()
        );
        assert!(
            c[3]["annual"][4]["energy_ticks"].as_i64().unwrap()
                > c[0]["annual"][4]["energy_ticks"].as_i64().unwrap()
        );
        assert!(s["short_shelf_life_counterexample"]["Err"].is_string());
    }
    #[test]
    fn no_automation_before_qualification_and_shared_capacity_is_not_duplicated() {
        let b = base();
        let (m, p) = build(&b["cases"][0], 600, [100; 5], false);
        let cap = limits();
        for row in &p[..7] {
            assert_eq!(
                row.activities
                    .iter()
                    .zip(&cap[1].activity_use)
                    .map(|(x, a)| x * a)
                    .sum::<i64>(),
                0
            );
        }
        assert!(m.replay_with_capabilities(&p, &cap).is_ok());
        let mut small = cap;
        small[1].units_per_stock = 1;
        assert!(m.replay_with_capabilities(&p, &small).is_err());
        assert!(
            verify_linear(
                &m.compile_with_capabilities(&small).unwrap(),
                &m.physical.vector(&p).unwrap(),
                None
            )
            .is_err()
        );
    }
    #[test]
    fn common_outage_has_no_anticipatory_actions_and_restores_pipeline_stocks() {
        let b = base();
        let (m, a) = build(&b["cases"][0], 600, [30, 40, 65, 85, 95], false);
        let (_, z) = build(&b["cases"][0], 600, [30, 40, 65, 85, 95], true);
        for t in 0..30 {
            assert_eq!(a[t].activities, z[t].activities);
            assert_eq!(a[t].closing, z[t].closing);
        }
        for k in 0..9 {
            assert!(z[59].closing[k] >= m.closing_floors[59][k]);
        }
        let cap = limits();
        for row in &z[30..=32] {
            assert_eq!(
                row.activities
                    .iter()
                    .zip(&cap[1].activity_use)
                    .map(|(x, a)| x * a)
                    .sum::<i64>(),
                0
            );
        }
        let reduction = |p: &[PeriodPlan]| {
            p[30..=32]
                .iter()
                .map(|r| {
                    r.activities
                        .iter()
                        .zip(&cap[0].activity_use)
                        .map(|(x, a)| x * a)
                        .sum::<i64>()
                })
                .sum::<i64>()
        };
        assert!(reduction(&z) > reduction(&a));
    }
    #[test]
    fn dated_route_cannot_hide_an_extra_delivery_or_renew_expiry() {
        let b = base();
        let (m, mut p) = build(&b["cases"][0], 0, [0; 5], false);
        assert!(dated(&m, &p, 18).is_ok());
        assert!(dated(&m, &p, 2).is_err());
        p[20].deliveries[6] += 1;
        assert!(dated(&m, &p, 18).is_err());
    }
}
