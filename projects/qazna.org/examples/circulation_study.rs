//! An explicit candidate for shared-production entitlements, not enacted policy.
//! All numbers are synthetic accounting measures, not observed prices or wages.
use qazna::mathematics::*;
use serde_json::json;
use std::{env, fs};

fn edge(
    events: &mut Vec<ScheduledMovement>,
    period: u32,
    from: usize,
    to: usize,
    amount: u64,
    basis: &str,
) {
    events.push(ScheduledMovement {
        period,
        movement: Movement { from, to, amount },
        basis: basis.into(),
    });
}

// Accounts: three need groups, retail, production, raw materials, equipment,
// three segregated in-transit balances. No daily treasury replenishment.
fn schedule(days: u32, delay: u32, growth: bool) -> Vec<ScheduledMovement> {
    let mut events = Vec::new();
    let mut releases: std::collections::BTreeMap<u32, Vec<(usize, u64)>> = Default::default();
    for day in 0..days + delay {
        if day < days {
            let scale = if growth {
                100 + 5 * u64::from(day / 365)
            } else {
                100
            };
            for (i, share) in [40, 30, 30].into_iter().enumerate() {
                edge(
                    &mut events,
                    day,
                    i,
                    3,
                    share * scale,
                    "Need-based household consumption; no employment condition",
                );
            }
            edge(
                &mut events,
                day,
                3,
                4,
                80 * scale,
                "Retail purchases production output",
            );
            edge(
                &mut events,
                day,
                4,
                5,
                40 * scale,
                "Production purchases raw materials",
            );
            edge(
                &mut events,
                day,
                4,
                6,
                10 * scale,
                "Production purchases equipment service",
            );
            edge(
                &mut events,
                day,
                5,
                6,
                10 * scale,
                "Raw-material unit purchases equipment service",
            );
            for (provider, residual) in [(3, 20), (4, 30), (5, 30), (6, 20)] {
                for (i, share) in [40, 30, 30].into_iter().enumerate() {
                    // Need-weighted claim on the entire accounted residual, with
                    // no separate shareholder dividend or wage counted twice.
                    let amount = residual * share * scale / 100;
                    edge(
                        &mut events,
                        day,
                        provider,
                        7 + i,
                        amount,
                        "Experimental unconditional share of common production",
                    );
                    releases.entry(day + delay).or_default().push((i, amount));
                }
            }
        }
        for (i, amount) in releases.remove(&day).unwrap_or_default() {
            edge(
                &mut events,
                day,
                7 + i,
                i,
                amount,
                "Settle segregated entitlement; no interest or fee",
            );
        }
    }
    assert!(releases.is_empty());
    events
}
// A single shared treasury holds timing reserves, not personal saving accounts.
// Enterprises account for a common entitlement after intermediate obligations.
fn pooled(events: &[ScheduledMovement]) -> Vec<ScheduledMovement> {
    let mut result = Vec::new();
    for event in events {
        if event.movement.from < 3 && event.movement.to == 3 {
            edge(
                &mut result,
                event.period,
                10,
                event.movement.from,
                event.movement.amount,
                "Daily common entitlement routed through one treasury",
            );
        }
        let mut e = event.clone();
        if e.movement.from >= 7 && e.movement.to < 3 {
            e.movement.to = 10;
        }
        result.push(e);
    }
    result
}
fn run(
    name: &str,
    events: &[ScheduledMovement],
    opening: Vec<u64>,
    denied: Vec<usize>,
) -> serde_json::Value {
    let result = replay_schedule(&opening, events, &denied).unwrap();
    json!({"name":name,"opening":opening,"blocked_event_indices":denied,"result":result})
}
fn main() {
    let base = schedule(1825, 0, false);
    let delayed = schedule(1825, 2, false);
    let growth = schedule(1825, 0, true);
    let opening = |events: &[ScheduledMovement]| {
        schedule_opening(10, events)
            .unwrap()
            .minimum_opening_by_account
            .iter()
            .map(|s| s.parse::<u64>().unwrap())
            .collect::<Vec<_>>()
    };
    let b = opening(&base);
    let d = opening(&delayed);
    let g = opening(&growth);
    assert_eq!(b, vec![4000, 3000, 3000, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(d, vec![12000, 9000, 9000, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(g, vec![4800, 3600, 3600, 0, 0, 0, 0, 0, 0, 0]);
    let denied: Vec<_> = base
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            (100..103).contains(&e.period) && e.movement.from == 4 && e.movement.to == 5
        })
        .map(|(i, _)| i)
        .collect();
    let cases = vec![
        run("closed_stationary", &base, b.clone(), vec![]),
        run("two_period_delay_unbuffered", &delayed, b.clone(), vec![]),
        run("two_period_delay_buffered", &delayed, d.clone(), vec![]),
        run("growth_unbuffered", &growth, b.clone(), vec![]),
        run("growth_prepositioned_measure", &growth, g.clone(), vec![]),
        run(
            "three_period_upstream_link_outage",
            &base,
            b.clone(),
            denied,
        ),
    ];
    for i in [0, 2, 4] {
        assert!(cases[i]["result"]["first_blocked_period"].is_null());
    }
    for i in [1, 3, 5] {
        assert!(!cases[i]["result"]["first_blocked_period"].is_null());
    }
    // Two disconnected self-sufficient regions remain feasible without transferring
    // their reserves between regions. This says nothing about missing real supplies.
    let mut regions = Vec::new();
    for e in &base {
        for offset in [0, 10] {
            let mut e = e.clone();
            e.movement.from += offset;
            e.movement.to += offset;
            regions.push(e);
        }
    }
    let mut regional = b.clone();
    regional.extend(&b);
    let isolated = run(
        "two_disconnected_complete_circuits",
        &regions,
        regional,
        vec![],
    );
    assert!(isolated["result"]["first_blocked_period"].is_null());
    let mut pooled_cases = Vec::new();
    for (name, events, expected) in [
        ("pooled_same_day", &base, 10000u64),
        ("pooled_two_period_delay", &delayed, 30000),
        ("pooled_growth", &growth, 12000),
    ] {
        let e = pooled(events);
        let bound = schedule_opening(11, &e).unwrap();
        let mut initial = vec![0; 11];
        initial[10] = expected;
        assert_eq!(
            bound.minimum_opening_by_account,
            initial.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
        let result = replay_schedule(&initial, &e, &[]).unwrap();
        assert!(result.first_blocked_period.is_none());
        assert_eq!(
            result.closing,
            initial.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
        let mut closing = initial.clone();
        for day in e.chunk_by(|a, b| a.period == b.period) {
            let day_result = replay_schedule(&closing, day, &[]).unwrap();
            assert!(day_result.first_blocked_period.is_none());
            closing = day_result
                .closing
                .iter()
                .map(|n| n.parse().unwrap())
                .collect();
            assert!(closing[..3].iter().all(|&b| b == 0));
        }
        pooled_cases.push(json!({"name":name,"opening":initial,"bound":bound,"result":result,"residents_end_each_period_zero_by_schedule":true}));
    }
    // Same daily quantities and unit measure, two delivery/payment waves.
    // Floor in the first wave, exact complement in the second; no unit is lost.
    let full = pooled(&growth);
    let mut waves = Vec::new();
    for day in full.chunk_by(|a, b| a.period == b.period) {
        for part in 0..2 {
            for event in day {
                let mut e = event.clone();
                e.movement.amount =
                    e.movement.amount / 2 + if part == 1 { e.movement.amount % 2 } else { 0 };
                if e.movement.amount > 0 {
                    waves.push(e);
                }
            }
        }
    }
    let bound = schedule_opening(11, &waves).unwrap();
    let mut initial = vec![0; 11];
    initial[10] = 6000;
    assert_eq!(
        bound.minimum_opening_by_account,
        initial.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
    let result = replay_schedule(&initial, &waves, &[]).unwrap();
    assert!(result.first_blocked_period.is_none());
    let mut closing = initial.clone();
    for day in waves.chunk_by(|a, b| a.period == b.period) {
        let day_result = replay_schedule(&closing, day, &[]).unwrap();
        assert!(day_result.first_blocked_period.is_none());
        closing = day_result
            .closing
            .iter()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(closing, initial);
    }
    assert_eq!(
        result.settled_volume,
        pooled_cases[2]["result"]["settled_volume"]
            .as_str()
            .unwrap()
    );
    pooled_cases.push(json!({"name":"pooled_growth_two_intraday_waves","opening":initial,"bound":bound,"result":result,"residents_end_each_period_zero_by_schedule":true}));
    let report = json!({"version":1,"kind":"synthetic_closed_circulation_candidate","model_calls":0,
        "accounts":["children_need_group","unable_to_work_need_group","working_age_need_group","retail","production","raw_materials","equipment","transit_children","transit_unable_to_work","transit_working_age"],
        "periods":1825,"unit":"synthetic minor accounting measure; fixed across scenarios",
        "pooled_cases":pooled_cases,"shared_treasury_account_index":10,
        "candidate_rule":"Enterprise receipts first cover declared intermediate inputs; the accounted residual is distributed by need-weighted unconditional common-production entitlements. No labour eligibility test, tax, loan, interest, deposit yield or automatic issuance. The pooled variant credits the explicitly assigned common entitlement to one treasury; it is not a levy on independently owned private revenue. This is a policy candidate, not a proven right or activated API rule.",
        "limitations":["Aggregate need groups; not 100 million individual actors","Accounting flows alone do not prove physical delivery, price formation or sufficiency","Physical production and capital replacement need separate coupled constraints","Growth is in quantity at unchanged unit measure; extra opening reserve is prepositioned, not issued at a shock","Delayed payments reside in explicit transit accounts; pooled cases keep bridging reserves in the common treasury, not personal accounts","Transfer failures preserve balances but miss scheduled deadlines; no automatic retry policy","Need weights are synthetic; additional contribution rewards and changing individual choice are not resolved"],
        "first_period_schedule":base.iter().filter(|e|e.period==0).collect::<Vec<_>>(),
        "bounds":{"stationary":schedule_opening(10,&base).unwrap(),"two_period_delay":schedule_opening(10,&delayed).unwrap(),"growth":schedule_opening(10,&growth).unwrap()},
        "cases":cases,"disconnected_regions":isolated});
    let output = env::args()
        .nth(1)
        .unwrap_or("research/circulation-model.json".into());
    fs::write(
        &output,
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("Verified closed circulation and timing counterexamples: {output}");
}
