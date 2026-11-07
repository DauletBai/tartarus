//! Coupled physical capabilities and payments. Synthetic coefficients only.
use qazna::{horizon::*, mathematics::*};
use serde_json::json;
use std::{env, fs};
const R: usize = 16;
const J: usize = 14;
const SCALE: i64 = 100_000; // 1,000 synthetic residents scaled to 100 million.
const DAYS: usize = 1825;
const VALUE: [i64; R] = [
    0, 0, 0, 100, 100, 100, 500, 10000, 0, 0, 200, 300, 600, 700, 10000, 200,
];
fn process(inputs: &[(usize, i64)], output: usize, quantity: i64, lead: usize) -> Process {
    let mut a = vec![0; R];
    let mut b = vec![0; R];
    for &(k, q) in inputs {
        a[k] = q;
    }
    b[output] = quantity;
    Process {
        inputs: a,
        outputs: b,
        lead_periods: lead,
    }
}
fn movement(
    events: &mut Vec<ScheduledMovement>,
    day: usize,
    from: usize,
    to: usize,
    amount: i64,
    basis: &str,
) {
    if amount > 0 && from != to {
        events.push(ScheduledMovement {
            period: day as u32,
            movement: Movement {
                from,
                to,
                amount: amount as u64,
            },
            basis: basis.into(),
        });
    }
}
fn study(
    name: &str,
    trim: bool,
    reroute: bool,
) -> (serde_json::Value, Vec<PeriodPlan>, Vec<ScheduledMovement>) {
    let resources = [
        "ore_in_ground",
        "fuel_in_ground",
        "water",
        "energy",
        "fuel",
        "ore",
        "metal",
        "equipment_units",
        "qualified_capacity",
        "eligible_training_places",
        "grain_farm",
        "grain_city",
        "meals_kitchen",
        "meals_delivered",
        "building_units",
        "housing_day",
    ];
    let names = [
        "fuel_extraction",
        "power",
        "ore_extraction",
        "metal",
        "equipment",
        "farming",
        "grain_transport",
        "cooking",
        "meal_delivery",
        "construction",
        "training",
        "housing_service",
        "reserve_grain_transport",
        "reserve_meal_delivery",
    ];
    let processes = vec![
        process(&[(1, 10), (3, 1)], 4, 10, 1),
        process(&[(4, 1), (2, 1)], 3, 10, 1),
        process(&[(0, 10), (3, 1)], 5, 10, 1),
        process(&[(5, 5), (3, 5)], 6, 5, 2),
        process(&[(6, 10), (3, 10)], 7, 1, 20),
        process(&[(2, 10), (3, 1)], 10, 10, 5),
        process(&[(10, 10), (3, 2)], 11, 10, 1),
        process(&[(11, 10), (3, 5)], 12, 10, 1),
        process(&[(12, 10), (3, 2)], 13, 10, 1),
        process(&[(6, 10), (3, 10)], 14, 1, 60),
        process(
            &[(9, 1), (3, 20)],
            8,
            1,
            if name == "training_delay" { 720 } else { 180 },
        ),
        process(&[(2, 1), (3, 1)], 15, 10, 1),
        process(&[(10, 10), (3, 2)], 11, 10, 2),
        process(&[(12, 10), (3, 2)], 13, 10, 2),
    ];
    let initial = [
        1_000_000, 1_000_000, 0, 4000, 1000, 1000, 2000, 300, 200, 120, 6000, 4000, 4000, 4000,
        1000, 1000,
    ];
    let storage = [
        1_000_000, 1_000_000, 0, 0, 2000, 2000, 3000, 1000, 1000, 120, 15000, 6000, 6000, 6000,
        2000, 0,
    ];
    let limits = vec![
        StockCapacity {
            name: "qualified_hours".into(),
            resource: 8,
            units_per_stock: 8,
            activity_use: vec![1, 1, 1, 2, 8, 2, 2, 2, 2, 8, 4, 1, 4, 4],
        },
        StockCapacity {
            name: "equipment_use".into(),
            resource: 7,
            units_per_stock: 4,
            activity_use: vec![1, 1, 1, 1, 2, 1, 2, 1, 2, 2, 0, 0, 4, 4],
        },
        StockCapacity {
            name: "housing_use".into(),
            resource: 14,
            units_per_stock: 1,
            activity_use: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0],
        },
    ];
    let exits = vec![
        Withdrawal {
            period: 400,
            resource: 7,
            quantity: 180 * SCALE,
            basis: "Synthetic equipment retirement shock".into(),
        },
        Withdrawal {
            period: 400,
            resource: 8,
            quantity: 100 * SCALE,
            basis: "Synthetic withdrawal of qualified availability; not loss of human rights"
                .into(),
        },
    ];
    let mut model = Horizon {
        resources: resources.iter().map(|s| s.to_string()).collect(),
        initial: initial.iter().map(|n| n * SCALE).collect(),
        storage_caps: storage.iter().map(|n| n * SCALE).collect(),
        processes,
        periods: vec![],
        objective_resource: 13,
    };
    let mut margin = [0i64; R];
    let mut owner = [0usize; R];
    for (j, p) in model.processes.iter().enumerate() {
        let k = p.outputs.iter().position(|&q| q > 0).unwrap();
        if owner[k] == 0 {
            owner[k] = j + 2;
        }
        if VALUE[k] > 0 {
            let cost: i64 = p.inputs.iter().zip(VALUE).map(|(q, v)| q * v).sum();
            assert_eq!(cost % p.outputs[k], 0);
            let recipe_margin = VALUE[k] - cost / p.outputs[k];
            if j >= 12 {
                assert_eq!(recipe_margin, margin[k]);
            }
            margin[k] = recipe_margin;
            assert!(margin[k] >= 0);
        }
    }
    let mut plan: Vec<PeriodPlan> = vec![];
    let mut previous = model.initial.clone();
    let mut pipeline = qazna::replanning::ProductionPipeline::new(R).unwrap();
    for day in 0..DAYS {
        let demand = (1000 + 50 * (day / 365) as i64) * SCALE;
        let mut data = Period {
            supplies: vec![0; R],
            needs: vec![0; R],
            capacities: vec![500 * SCALE; J],
        };
        data.supplies[2] = 10000 * SCALE;
        data.needs[13] = demand;
        data.needs[15] = demand;
        if (name == "transport_outage" && (365..395).contains(&day))
            || (matches!(
                name,
                "long_transport_outage" | "compound_outage" | "compound_delayed"
            ) && (365..455).contains(&day))
        {
            data.capacities[6] = 0;
            data.capacities[8] = 0;
        }
        if (name == "power_outage" && (600..615).contains(&day))
            || (name == "compound_outage" && (390..405).contains(&day))
            || (name == "compound_delayed" && (390..420).contains(&day))
        {
            data.capacities[1] = 0;
        }
        if name == "equipment_replacement_disabled" {
            data.capacities[4] = 0;
        }
        let mut opening = previous.clone();
        for e in exits.iter().filter(|e| e.period == day) {
            assert!(opening[e.resource] >= e.quantity);
            opening[e.resource] -= e.quantity;
        }
        let arriving = pipeline.advance(day).unwrap();
        let mut available: Vec<_> = (0..R)
            .map(|k| opening[k] + data.supplies[k] + arriving[k])
            .collect();
        let deliveries: Vec<_> = (0..R).map(|k| available[k].min(data.needs[k])).collect();
        for k in 0..R {
            available[k] -= deliveries[k];
        }
        let mut budget: Vec<_> = limits
            .iter()
            .map(|l| opening[l.resource] * l.units_per_stock)
            .collect();
        let target = [
            0, 0, 0, 2200, 1000, 1000, 2000, 400, 320, 0, 12000, 5000, 5000, 5000, 1300, 1200,
        ];
        let mut activities = vec![0; J];
        // Published annual growth is known ahead; outages are not anticipated.
        // Preserve core energy first, then
        // provisioning; capacity expansion/training use remaining shared resources.
        for j in [1, 0, 11, 8, 13, 7, 6, 12, 5, 2, 3, 4, 9, 10] {
            let p = &model.processes[j];
            if j >= 12 && !reroute {
                continue;
            }
            if day + p.lead_periods >= DAYS {
                continue;
            }
            let k = p.outputs.iter().position(|&q| q > 0).unwrap();
            let desired = if k == 3 {
                220 * SCALE
            } else if k == 15 {
                ((1000 + 50 * ((day + 1) / 365) as i64) * SCALE + 9) / 10
            } else {
                ((target[k] * SCALE - available[k] - pipeline.pending()[k]).max(0) + p.outputs[k]
                    - 1)
                    / p.outputs[k]
            };
            let mut n = desired.min(data.capacities[j]);
            if j == 4 || j == 9 || j == 10 {
                n = n.min(SCALE);
            }
            for (k, &q) in p.inputs.iter().enumerate() {
                if q > 0 {
                    n = n.min(available[k] / q);
                }
            }
            for (l, lim) in limits.iter().enumerate() {
                if lim.activity_use[j] > 0 {
                    n = n.min(budget[l] / lim.activity_use[j]);
                }
            }
            activities[j] = n;
            for (k, &q) in p.inputs.iter().enumerate() {
                available[k] -= q * n;
            }
            for (l, lim) in limits.iter().enumerate() {
                budget[l] -= lim.activity_use[j] * n;
            }
            if n > 0 {
                assert!(pipeline.commit(day, j, p, n).unwrap());
            }
        }
        let waste: Vec<_> = (0..R)
            .map(|k| (available[k] - model.storage_caps[k]).max(0))
            .collect();
        let closing: Vec<_> = (0..R).map(|k| available[k] - waste[k]).collect();
        previous = closing.clone();
        model.periods.push(data);
        plan.push(PeriodPlan {
            activities,
            deliveries,
            closing,
            discarded: waste,
        });
    }
    assert!(pipeline.pending().iter().all(|&q| q == 0));
    let terminal = plan.last().unwrap().closing.clone();
    let trim_evidence = if trim {
        let (improved, evidence) = qazna::optimization::trim_plan(
            &model,
            &plan,
            &limits,
            &exits,
            &terminal,
            &[9, 4, 10, 11, 8, 13, 7, 6, 12, 5, 3, 2, 1, 0],
            8,
        )
        .unwrap();
        assert!(
            improved
                .iter()
                .zip(&plan)
                .all(|(a, b)| a.deliveries == b.deliveries)
        );
        plan = improved;
        Some(evidence)
    } else {
        None
    };
    let mut events = Vec::new();
    let mut links = Vec::new();
    let mut missing = [0i128; R];
    let mut discarded = [0i128; R];
    let mut first = None;
    let mut years = vec![];
    let mut starts = [0i128; J];
    let mut measures = [0i128; 3]; // requested, delivered, capital procurement
    let mut first_departure = vec![None; J];
    // Input payments follow actual use; suppliers circulate their declared common
    // entitlement. Natural stocks and qualified people are not priced as property.
    let trade = |events: &mut Vec<ScheduledMovement>,
                 links: &mut Vec<CashLink>,
                 source: PaymentSource,
                 day: usize,
                 buyer: usize,
                 k: usize,
                 q: i64| {
        if q > 0 && VALUE[k] > 0 {
            links.push(CashLink {
                period: day,
                source,
                quantity: q,
                amount: (i128::from(q) * i128::from(VALUE[k])).to_string(),
            });
        }
        movement(
            events,
            day,
            buyer,
            owner[k],
            q * VALUE[k],
            "Actual input release or completed provision at fixed normative measure",
        );
        movement(
            events,
            day,
            owner[k],
            0,
            q * margin[k],
            "Declared common-production entitlement on actual receipt",
        );
    };
    // Rebuild all goods links, money events and measures from the chosen plan.
    // No seed-plan payment survives an activity change without recalculation.
    for (day, period_plan) in plan.iter().enumerate() {
        let data = &model.periods[day];
        let deliveries = &period_plan.deliveries;
        let mut completed = [0i64; R];
        for (j, proc) in model.processes.iter().enumerate() {
            if day >= proc.lead_periods {
                for (k, q) in completed.iter_mut().enumerate() {
                    *q += proc.outputs[k] * plan[day - proc.lead_periods].activities[j];
                }
            }
        }
        for k in [7, 14] {
            trade(
                &mut events,
                &mut links,
                PaymentSource::Capital {
                    process: owner[k] - 2,
                    resource: k,
                },
                day,
                0,
                k,
                completed[k],
            );
            measures[2] += i128::from(completed[k]) * i128::from(VALUE[k]);
        }
        for k in 0..R {
            missing[k] += i128::from(data.needs[k] - deliveries[k]);
            if deliveries[k] < data.needs[k] && first.is_none() {
                first = Some(day);
            }
            measures[0] += i128::from(data.needs[k]) * i128::from(VALUE[k]);
            measures[1] += i128::from(deliveries[k]) * i128::from(VALUE[k]);
            if deliveries[k] > 0 && VALUE[k] > 0 {
                movement(
                    &mut events,
                    day,
                    0,
                    1,
                    deliveries[k] * VALUE[k],
                    "Common provision for actual delivery; no labour condition",
                );
                trade(
                    &mut events,
                    &mut links,
                    PaymentSource::Provision { resource: k },
                    day,
                    1,
                    k,
                    deliveries[k],
                );
            }
        }
        for j in [1, 0, 11, 8, 13, 7, 6, 12, 5, 2, 3, 4, 9, 10] {
            let p = &model.processes[j];
            let n = period_plan.activities[j];
            starts[j] += i128::from(n);
            if n > 0 && first_departure[j].is_none() {
                first_departure[j] = Some(day);
            }
            for (k, &q) in p.inputs.iter().enumerate() {
                if q > 0 && VALUE[k] > 0 {
                    trade(
                        &mut events,
                        &mut links,
                        PaymentSource::Input {
                            process: j,
                            resource: k,
                        },
                        day,
                        if j == 10 {
                            0
                        } else if j == 12 {
                            8
                        } else if j == 13 {
                            10
                        } else {
                            j + 2
                        },
                        k,
                        q * n,
                    );
                }
            }
        }
        for (total, &q) in discarded.iter_mut().zip(&period_plan.discarded) {
            *total += i128::from(q);
        }
        if (day + 1) % 365 == 0 {
            years.push(json!({"year":(day+1)/365,"stock":period_plan.closing,"discarded":discarded.iter().map(ToString::to_string).collect::<Vec<_>>(),"unmet":missing.iter().map(ToString::to_string).collect::<Vec<_>>()}));
        }
    }
    let matrix = model.compile_capabilities(&limits, &exits).unwrap();
    let vector = model.vector(&plan).unwrap();
    model
        .verify_cash_links(&plan, &VALUE, &[7, 14], &links)
        .unwrap();
    let mut forged = links.clone();
    forged[0].amount = (forged[0].amount.parse::<i128>().unwrap() + 1).to_string();
    assert!(
        model
            .verify_cash_links(&plan, &VALUE, &[7, 14], &forged)
            .is_err()
    );
    let verified = verify_linear(&matrix, &vector, None).unwrap();
    let dual = model.full_coverage_dual(&matrix);
    let optimal = verify_linear(&matrix, &vector, Some(&dual)).is_ok();
    let mut infeasibility_bound = None;
    if name == "transport_outage" && !reroute {
        let mut required = matrix.clone();
        for t in 0..DAYS {
            required.rows.push(Row {
                name: format!("fixed_route_policy:{t}"),
                terms: vec![(t * (3 * R + J) + 3 * R + 13, 1)],
                sense: Sense::AtMost,
                rhs: 0,
            });
        }
        let mut weights = vec![0i64; required.rows.len()];
        let index: std::collections::BTreeMap<_, _> = required
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| (r.name.clone(), i))
            .collect();
        for t in 365..395 {
            weights[index[&format!("balance:{t}:13")]] = 1;
            weights[index[&format!("capacity:{}:8", t - 1)]] = 10;
            weights[index[&format!("fixed_route_policy:{}", t - 2)]] = 10;
        }
        weights[index["storage:364:13"]] = 1;
        for t in 365..395 {
            required.rows.push(Row {
                name: format!("mandatory_meals:{t}"),
                terms: vec![(t * (3 * R + J) + 13, -1)],
                sense: Sense::AtMost,
                rhs: -model.periods[t].needs[13],
            });
            weights.push(1);
        }
        verify_infeasible(
            &required,
            &RationalVector {
                numerators: weights,
                denominator: 1,
            },
        )
        .unwrap();
        let required_food: i64 = model.periods[365..395].iter().map(|p| p.needs[13]).sum();
        infeasibility_bound =
            Some(required_food - model.storage_caps[13] - model.periods[364].capacities[8] * 10);
    }
    let bound = schedule_opening(J + 2, &events).unwrap();
    let funds: Vec<u64> = bound
        .minimum_opening_by_account
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let money = replay_schedule(&funds, &events, &[]).unwrap();
    assert!(money.first_blocked_period.is_none());
    assert_eq!(funds[1], 0);
    assert_eq!(money.closing[1], "0");
    // Independent per-period checks: physical equations and shared capacity uses.
    let mut prior = model.initial.clone();
    for (day, p) in plan.iter().enumerate() {
        for (k, &opening_stock) in prior.iter().enumerate() {
            let removed = exits
                .iter()
                .filter(|e| e.period == day && e.resource == k)
                .map(|e| e.quantity)
                .sum::<i64>();
            let incoming = model
                .processes
                .iter()
                .enumerate()
                .filter(|(_, proc)| day >= proc.lead_periods)
                .map(|(j, proc)| proc.outputs[k] * plan[day - proc.lead_periods].activities[j])
                .sum::<i64>();
            let used = model
                .processes
                .iter()
                .enumerate()
                .map(|(j, proc)| proc.inputs[k] * p.activities[j])
                .sum::<i64>();
            assert_eq!(
                opening_stock - removed + model.periods[day].supplies[k] + incoming,
                p.closing[k] + p.deliveries[k] + p.discarded[k] + used
            );
        }
        for limit in &limits {
            let removed = exits
                .iter()
                .filter(|e| e.period == day && e.resource == limit.resource)
                .map(|e| e.quantity)
                .sum::<i64>();
            let used: i64 = limit
                .activity_use
                .iter()
                .zip(&p.activities)
                .map(|(a, x)| a * x)
                .sum();
            assert!(used <= (prior[limit.resource] - removed) * limit.units_per_stock);
        }
        prior = p.closing.clone();
    }
    eprintln!("Verified scenario {name}, trim={trim}");
    (
        json!({"scenario":name,"rerouting":reroute,"trim_evidence":trim_evidence,"input_status":"synthetic_uncalibrated","population_scale":100_000_000,"periods":DAYS,"resources":resources,"process_names":names,"processes":model.processes,"initial":model.initial,"storage_caps":model.storage_caps,"limits":limits,"withdrawals":exits,
        "fixed_unit_measure":VALUE,"common_entitlement_per_sold_unit":margin,"owner_account":owner,
        "matrix":{"variables":matrix.objective.len(),"rows":matrix.rows.len(),"nonzero_terms":matrix.rows.iter().map(|r|r.terms.len()).sum::<usize>()},"minimum_outage_unmet_proven":infeasibility_bound,"outage_bound_scope":if infeasibility_bound.is_some(){Some("fixed_route_policy_no_reserve_meal_dispatch")}else{None},"cash_link_count":links.len(),"forged_payment_rejected":true,"physical_verification":verified,"food_objective_optimal":optimal,"first_unmet_day":first,"discarded":discarded.iter().map(ToString::to_string).collect::<Vec<_>>(),"unmet":missing.iter().map(ToString::to_string).collect::<Vec<_>>(),"activity_totals":starts.iter().map(ToString::to_string).collect::<Vec<_>>(),"first_activity":first_departure,"years":years,
        "requested_provision_measure":measures[0].to_string(),"delivered_provision_measure":measures[1].to_string(),"completed_capital_procurement_measure":measures[2].to_string(),"liquidity":bound,"money":money}),
        plan,
        events,
    )
}
fn main() {
    let cases: Vec<_> = [
        "baseline",
        "transport_outage",
        "power_outage",
        "equipment_replacement_disabled",
        "training_delay",
    ]
    .into_iter()
    .map(|name| study(name, false, false).0)
    .collect();
    assert!(cases[0]["first_unmet_day"].is_null());
    assert!(cases[0]["food_objective_optimal"].as_bool().unwrap());
    assert!(
        cases
            .iter()
            .skip(1)
            .all(|c| !c["first_unmet_day"].is_null())
    );
    let improved: Vec<_> = cases
        .iter()
        .map(|c| study(c["scenario"].as_str().unwrap(), true, false).0)
        .collect();
    let comparisons: Vec<_> = cases.iter().zip(&improved).map(|(before,after)| {
        assert_eq!(before["unmet"], after["unmet"]);
        assert_eq!(before["delivered_provision_measure"],after["delivered_provision_measure"]);
        let old: Vec<i128> = before["activity_totals"].as_array().unwrap().iter().map(|x|x.as_str().unwrap().parse().unwrap()).collect();
        let new: Vec<i128> = after["activity_totals"].as_array().unwrap().iter().map(|x|x.as_str().unwrap().parse().unwrap()).collect();
        assert!(old.iter().zip(&new).all(|(a,b)| b<=a));
        assert!(new[0] < old[0] && new[1] < old[1]);
        assert!(after["discarded"][3].as_str().unwrap().parse::<i128>().unwrap()
            < before["discarded"][3].as_str().unwrap().parse::<i128>().unwrap());
        assert!(after["trim_evidence"]["coordinate_irreducible"].as_bool().unwrap());

        json!({"scenario":before["scenario"],"deliveries_preserved":true,
            "energy_discard_before":before["discarded"][3],"energy_discard_after":after["discarded"][3],
            "fuel_extraction_batches_before":old[0].to_string(),"fuel_extraction_batches_after":new[0].to_string(),
            "opening_measure_before":before["liquidity"],"opening_measure_after":after["liquidity"],
            "trim_evidence":after["trim_evidence"]})
    }).collect();
    let (normal, normal_trace, normal_events) = study("baseline", false, true);
    let (recovered, recovered_trace, recovered_events) = study("transport_outage", false, true);
    let (long_fixed, _, _) = study("long_transport_outage", false, false);
    let (long_recovered, long_trace, long_events) = study("long_transport_outage", false, true);
    assert_eq!(
        serde_json::to_value(&normal_trace[..365]).unwrap(),
        serde_json::to_value(&recovered_trace[..365]).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&recovered_trace[..395]).unwrap(),
        serde_json::to_value(&long_trace[..395]).unwrap()
    );
    let recovered_food: i128 = recovered["unmet"][13].as_str().unwrap().parse().unwrap();
    let fixed_food: i128 = cases[1]["unmet"][13].as_str().unwrap().parse().unwrap();
    assert!(recovered_food < fixed_food);
    let (power, power_trace, power_events) = study("power_outage", false, true);
    let (compound, compound_trace, compound_events) = study("compound_outage", false, true);
    let (delayed, delayed_trace, delayed_events) = study("compound_delayed", false, true);
    for (a, b, end) in [
        (&normal_trace, &power_trace, 600),
        (&long_trace, &compound_trace, 390),
        (&compound_trace, &delayed_trace, 405),
    ] {
        assert_eq!(
            serde_json::to_value(&a[..end]).unwrap(),
            serde_json::to_value(&b[..end]).unwrap()
        );
    }
    let family = [
        ("normal", &normal, &normal_events),
        ("transport_30", &recovered, &recovered_events),
        ("transport_90", &long_recovered, &long_events),
        ("power_15", &power, &power_events),
        ("compound_15", &compound, &compound_events),
        ("compound_30", &delayed, &delayed_events),
    ];
    let schedules: Vec<_> = family
        .iter()
        .map(|(name, _, e)| (*name, e.as_slice()))
        .collect();
    let envelope = common_schedule_opening(J + 2, &schedules).unwrap();
    let common_funds: Vec<u64> = envelope
        .minimum_opening_by_account
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let normal_funds: Vec<u64> = normal["liquidity"]["minimum_opening_by_account"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().parse().unwrap())
        .collect();
    let mut trials = vec![];
    for (name, result, events) in &family {
        for field in [
            "initial",
            "storage_caps",
            "processes",
            "limits",
            "withdrawals",
            "fixed_unit_measure",
            "owner_account",
        ] {
            assert_eq!(
                result[field], normal[field],
                "family mismatch {name}/{field}"
            );
        }
        let run = replay_schedule(&common_funds, events, &[]).unwrap();
        assert!(run.first_blocked_event.is_none());
        assert_eq!(run.closing[1], "0");
        let insufficient_baseline = replay_schedule(&normal_funds, events, &[]).unwrap();
        trials.push(json!({"scenario":name,"common_opening_run":run,"normal_opening_run":insufficient_baseline,
            "unmet":result["unmet"],"first_unmet_day":result["first_unmet_day"],
            "discarded":result["discarded"],"physical_verification":result["physical_verification"],
            "scenario_specific_opening":result["liquidity"]["total_minimum_opening"]}));
    }
    // Necessity witnesses: one less unit in any positive coordinate must fail
    // in at least one attaining scenario. No global total-only claim is made.
    let mut necessity = vec![];
    for (account, witness) in envelope.witnesses.iter().enumerate() {
        if let Some(w) = witness {
            let (_, _, events) = family.iter().find(|(n, _, _)| *n == w.scenario).unwrap();
            let mut reduced = common_funds.clone();
            reduced[account] -= 1;
            let run = replay_schedule(&reduced, events, &[]).unwrap();
            assert_eq!(run.first_blocked_event, Some(w.event));
            necessity.push(json!({"account":account,"witness":w,"one_unit_less_blocked":true}));
        }
    }
    let common_funding = json!({"scope":"six_declared_causal_schedules_no_later_issuance_or_rebalancing",
        "envelope":envelope,"same_initial_physical_basis":true,"future_observation_prefix_tests_passed":true,
        "trials":trials,"necessity_checks":necessity,
        "compound_outage":compound,"compound_delayed":delayed,"power_outage":power});
    let recovery = json!({"policy":"causal_reserve_routes_v1","future_observation_prefix_tests_passed":true,
        "normal":normal,"transport_outage":recovered,"long_fixed":long_fixed,"long_recovery":long_recovered});
    let report = json!({"version":4,"kind":"synthetic_coupled_capability_study","model_calls":0,
        "assumptions":["Synthetic aggregate coefficients, not demographic/engineering norms","Water is an external boundary supply; deposits and initial assets/stocks are explicit","Training places represent consenting eligible capacity, not people owned or traded","Qualified capacity and equipment are shared use limits, not consumed ingredients","Public asset procurement is paid only for completed equipment/buildings","Input payments follow actual consumption; incomplete work ties up opening working measure","Natural resources and people have zero commodity price in this example","All monetary norms fixed; no interest, tax, loan or later issuance","Actual missing provision remains recorded; no claim that missing goods cancel human entitlements","Money is prefunded using exact schedule bounds, not optimized globally","Trimmed cases are retrospective fixed-delivery improvements, not causal forecasts or global optima","Recovery decisions use current observed capacities, stocks and committed output; no future outage duration is provided","Reserve routes take two periods and four shared equipment/skill units per batch; synthetic capacity is declared in every comparison","The original terminal stock is a lower bound in trimmed cases; all intermediate safety-stock policy remains open","Energy/fuel/material quantities are abstract recipe units, not measured kWh/tonnes; external water left unused is reported separately from energy discard","Inventory age/expiry, health demography, individual choice, teacher specializations and foreign trade remain open","Heuristic is not a global optimum except when a valid stated objective certificate exists"],"cases":cases,"improved_cases":improved,"comparisons":comparisons,"recovery":recovery,"common_funding":common_funding});
    let path = env::args()
        .nth(1)
        .unwrap_or("research/coupled-results.json".into());
    fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    println!("Verified coupled physical and monetary traces: {path}");
}
