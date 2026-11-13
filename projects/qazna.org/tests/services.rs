use qazna::{model_scenarios::*, population::*, services::*};
use serde_json::{Value, json};
fn fixture() -> (ModelDefinition, ModelInput) {
    let v: Value = serde_json::from_str(include_str!("../research/service-results.json")).unwrap();
    (
        serde_json::from_value(v["runtime_fixture"]["definition"].clone()).unwrap(),
        serde_json::from_value(v["runtime_fixture"]["input"].clone()).unwrap(),
    )
}
fn audit(
    d: &ModelDefinition,
    i: &ModelInput,
) -> Result<ServiceEvidence, qazna::mathematics::MathError> {
    let p = i.population.as_ref().unwrap().audit(d, &i.model, &i.plan)?;
    i.services.as_ref().unwrap().audit(d, &i.model, &i.plan, &p)
}
#[test]
fn selected_service_demand_is_fully_covered_and_not_stored() {
    let (d, i) = fixture();
    let e = audit(&d, &i).unwrap();
    assert!(e.all_selected_services_feasible);
    assert_eq!(e.coverage.len(), 360);
    assert_eq!(e.pools.len(), 720);
    assert!(e.coverage.iter().all(|r| r.required == r.delivered));
    assert!(e.unbound_items.contains(&"protein".to_owned()));
    assert!(
        i.model
            .seasonal
            .physical
            .resources
            .iter()
            .all(|r| !matches!(
                r.as_str(),
                "care_time" | "school_capacity" | "preschool_capacity"
            ))
    );
    assert!(preview(d, i).is_ok());
}
#[test]
fn independent_capacity_and_care_totals_match_every_month() {
    let (d, i) = fixture();
    let e = audit(&d, &i).unwrap();
    let s = i.services.as_ref().unwrap();
    for (pi, pool) in s.pools.iter().enumerate() {
        for t in 0..60 {
            let row = &e.pools[t * s.pools.len() + pi];
            let stock = if t == 0 {
                i.model.seasonal.physical.initial[pool.resource]
            } else {
                i.plan[t - 1].closing[pool.resource]
            };
            let daily = if pool.unit == "place-day" { 1 } else { 480 };
            assert_eq!(
                row.available,
                (stock * u64::from(pool.working_days[t]) as i64 * daily).to_string()
            );
            if pi % 6 == 0 {
                let care = s
                    .rules
                    .iter()
                    .find(|r| r.region == pool.region && r.item == "care_time")
                    .unwrap();
                assert_eq!(row.service_use, care.delivered[t].to_string());
            }
        }
    }
}
#[test]
fn trainees_use_instructors_through_course_but_not_after_completion() {
    let (d, i) = fixture();
    let e = audit(&d, &i).unwrap();
    let s = i.services.as_ref().unwrap();
    let k = s
        .pools
        .iter()
        .enumerate()
        .find(|(k, _)| k % 6 == 0 && i.plan[0].activities[k + 1] > 0)
        .unwrap()
        .0;
    let n = i.plan[0].activities[k + 1];
    let lag = i.model.seasonal.physical.processes[k + 1].lead_periods;
    for t in 0..60 {
        let row = &e.pools[t * s.pools.len() + k];
        assert_eq!(
            row.pipeline_use,
            if t < lag {
                (n * 120).to_string()
            } else {
                "0".into()
            }
        );
        let initial = i.model.seasonal.physical.initial[s.pools[k].resource];
        assert_eq!(
            row.active_stock,
            (initial + if t > lag { n } else { 0 }).to_string()
        );
    }
}
#[test]
fn regional_absence_cannot_borrow_other_region_or_reduce_need() {
    let (d, mut i) = fixture();
    let baseline = audit(&d, &i).unwrap();
    i.services.as_mut().unwrap().pools[0].availability_bps[20] = 0;
    let e = audit(&d, &i).unwrap();
    assert!(!e.all_selected_services_feasible);
    assert_eq!(json!(baseline.coverage), json!(e.coverage));
    assert!(
        e.pools
            .iter()
            .any(|p| p.period == 20 && p.pool == "north_care_staff" && p.shortage != "0")
    );
    assert!(preview(d, i).is_err());
}
#[test]
fn extra_training_work_cannot_double_book_service_staff() {
    let (d, mut i) = fixture();
    let p = i
        .services
        .as_mut()
        .unwrap()
        .registry
        .parameters
        .iter_mut()
        .find(|p| p.id == "course_work")
        .unwrap();
    let q = Ratio {
        numerator: 1_000_000,
        denominator: 1,
    };
    p.estimate = Some(Estimate {
        lower: q,
        central: q,
        upper: q,
    });
    let e = audit(&d, &i).unwrap();
    assert!(!e.all_selected_services_feasible);
    assert!(e.pools.iter().any(|p| p.period < 6 && p.shortage != "0"));
    assert!(preview(d, i).is_err());
}
#[test]
fn balanced_but_missing_operating_energy_is_rejected() {
    let (d, mut i) = fixture();
    i.model.seasonal.physical.periods[10].needs[3] -= 1;
    i.plan[10].deliveries[3] -= 1;
    for row in &mut i.plan[10..] {
        row.closing[3] += 1;
    }
    assert!(i.model.replay(&i.plan).is_ok());
    assert!(audit(&d, &i).is_err());
    assert!(preview(d, i).is_err());
}
#[test]
fn missing_service_does_not_roll_into_next_month() {
    let (d, mut i) = fixture();
    i.services.as_mut().unwrap().rules[0].delivered[0] -= 1;
    let rules = &i.services.as_ref().unwrap().rules;
    let care: u64 = rules
        .iter()
        .filter(|r| r.item == "care_time")
        .map(|r| r.delivered[0])
        .sum();
    let education: u64 = rules
        .iter()
        .filter(|r| r.item != "care_time")
        .map(|r| r.delivered[0])
        .sum();
    let energy = i64::try_from(care.div_ceil(1000) + education.div_ceil(10)).unwrap();
    let saved = i.plan[0].deliveries[3] - energy;
    i.plan[0].deliveries[3] = energy;
    i.model.seasonal.physical.periods[0].needs[3] = energy;
    for row in &mut i.plan {
        row.closing[3] += saved;
    }
    let e = audit(&d, &i).unwrap();
    assert!(!e.all_selected_services_feasible);
    assert_eq!(e.coverage[0].unmet, "1");
    assert!(
        e.coverage
            .iter()
            .filter(|r| r.period > 0)
            .all(|r| r.unmet == "0")
    );
    assert!(preview(d, i).is_err());
}
#[test]
fn duplicate_cross_region_and_incomplete_bindings_fail() {
    for mode in 0..6 {
        let (d, mut i) = fixture();
        let s = i.services.as_mut().unwrap();
        match mode {
            0 => s.pools[1].resource = s.pools[0].resource,
            1 => s.rules[0].pools[0].pool = "south_care_staff".into(),
            2 => {
                s.rules.pop();
            }
            3 => s.rules[1] = s.rules[0].clone(),
            4 => s.rules[0].operating[0].resource = s.pools[0].resource,
            _ => s.pools[0]
                .process_load_parameters
                .pop()
                .map(|_| ())
                .unwrap(),
        };
        assert!(audit(&d, &i).is_err(), "{mode}");
    }
}
#[test]
fn work_calendar_and_unknown_coefficients_are_not_silently_accepted() {
    for mode in 0..5 {
        let (d, mut i) = fixture();
        let s = i.services.as_mut().unwrap();
        match mode {
            0 => s.pools[0].working_days[1] = 29,
            1 => s.pools[0].availability_bps[0] = 10001,
            2 => s.registry.parameters[0].unit = "wrong".into(),
            3 => {
                s.registry.parameters[0].kind = EvidenceKind::Unknown;
                s.registry.parameters[0].estimate = None;
            }
            _ => s.registry.mode = InputMode::SourceAttributed,
        };
        assert!(audit(&d, &i).is_err());
    }
}
#[test]
fn retired_capacity_cannot_keep_delivering_services() {
    let (d, mut i) = fixture();
    let r = i.services.as_ref().unwrap().pools[0].resource;
    let initial = i.model.seasonal.physical.initial[r];
    i.model
        .lives
        .iter_mut()
        .find(|a| a.resource == r)
        .unwrap()
        .initial_remaining_periods = 8;
    i.plan[8].discarded[r] = initial;
    for row in &mut i.plan[8..] {
        row.closing[r] -= initial;
    }
    assert!(i.model.replay(&i.plan).is_ok());
    let e = audit(&d, &i).unwrap();
    let row = e
        .pools
        .iter()
        .find(|p| p.pool == "north_care_staff" && p.period == 8)
        .unwrap();
    assert_eq!(row.active_stock, i.plan[0].activities[1].to_string());
    assert_ne!(row.shortage, "0");
    assert!(preview(d, i).is_err());
}
#[test]
fn fractional_available_capacity_is_floored() {
    let (d, mut i) = fixture();
    let s = i.services.as_mut().unwrap();
    let p = s
        .registry
        .parameters
        .iter_mut()
        .find(|p| p.id == "work_rate")
        .unwrap();
    let q = Ratio {
        numerator: 1,
        denominator: 3,
    };
    p.estimate = Some(Estimate {
        lower: q,
        central: q,
        upper: q,
    });
    s.pools[0].availability_bps[0] = 3333;
    let e = audit(&d, &i).unwrap();
    let n = i.model.seasonal.physical.initial[6] as u128;
    assert_eq!(e.pools[0].available, (n * 20 * 3333 / 30000).to_string());
    assert!(!e.all_selected_services_feasible);
}
#[test]
fn unknown_fields_and_negative_quantity_fail_deserialization() {
    let (_, i) = fixture();
    let mut v = json!(i.services);
    v["pools"][0]["availability_bps"][0] = json!(-1);
    assert!(serde_json::from_value::<ServicePlan>(v).is_err());
    let mut v = json!(i.services);
    v["rules"][0]["execute_transfer"] = json!(true);
    assert!(serde_json::from_value::<ServicePlan>(v).is_err());
}

fn feedback_fixture() -> (ModelDefinition, ModelInput, ModelInput, ModelInput) {
    let v: Value = serde_json::from_str(include_str!("../research/service-results.json")).unwrap();
    (
        serde_json::from_value(v["runtime_fixture"]["definition"].clone()).unwrap(),
        serde_json::from_value(v["feedback"]["baseline_input"].clone()).unwrap(),
        serde_json::from_value(v["feedback"]["observed_input"].clone()).unwrap(),
        serde_json::from_value(v["feedback"]["repair"]["input"].clone()).unwrap(),
    )
}
#[test]
fn feedback_finds_earliest_feasible_training_date_without_new_resources() {
    use qazna::service_feedback::repair_service_batch;
    let (d, baseline, observed, repaired) = feedback_fixture();
    let before = preview(d.clone(), baseline.clone()).unwrap();
    let saved = json!(before);
    let result = repair_service_batch(&before, &observed, 1, 4, 12).unwrap();
    assert_eq!(
        result.attempts.iter().map(|r| r.start).collect::<Vec<_>>(),
        vec![4, 5, 6]
    );
    assert!(result.attempts[..2].iter().all(|r| r.rejection.is_some()));
    assert_eq!(json!(result.input.unwrap()), json!(repaired));
    assert_eq!(json!(before), saved);
    let after = preview_continuation(before, repaired.clone()).unwrap();
    let c: ModelCheckpoint = serde_json::from_str(after.checkpoint.as_ref().unwrap()).unwrap();
    assert!(c.services.as_ref().unwrap().all_selected_services_feasible);
    assert_eq!(json!(baseline.model), json!(repaired.model));
    assert_eq!(json!(baseline.population), json!(repaired.population));
    assert_eq!(json!(&baseline.plan[..4]), json!(&repaired.plan[..4]));
    assert!(!c.rolling.pending.is_empty());
    assert_eq!(
        json!(audit(&d, &baseline).unwrap().coverage),
        json!(c.services.unwrap().coverage)
    );
}
#[test]
fn deferral_stock_delta_matches_independent_two_event_balance() {
    let (_, baseline, _, repaired) = feedback_fixture();
    let p = &baseline.model.seasonal.physical.processes[1];
    let n = baseline.plan[4].activities[1];
    for t in 0..60 {
        assert_eq!(baseline.plan[t].deliveries, repaired.plan[t].deliveries);
        assert_eq!(baseline.plan[t].discarded, repaired.plan[t].discarded);
        for k in 0..baseline.plan[t].closing.len() {
            let saved_input = if (4..6).contains(&t) {
                n * p.inputs[k]
            } else {
                0
            };
            let delayed_output = if (10..12).contains(&t) {
                n * p.outputs[k]
            } else {
                0
            };
            assert_eq!(
                repaired.plan[t].closing[k],
                baseline.plan[t].closing[k] + saved_input - delayed_output
            );
        }
        for j in 0..baseline.plan[t].activities.len() {
            if j != 1 {
                assert_eq!(
                    baseline.plan[t].activities[j],
                    repaired.plan[t].activities[j]
                );
            }
        }
    }
    assert_eq!(
        baseline.plan.last().unwrap().closing,
        repaired.plan.last().unwrap().closing
    );
}
#[test]
fn unexplained_or_rewritten_availability_changes_are_rejected() {
    let (d, baseline, _, repaired) = feedback_fixture();
    let before = preview(d, baseline).unwrap();
    for mode in 0..12 {
        let mut next = repaired.clone();
        let s = next.services.as_mut().unwrap();
        match mode {
            0 => s.revisions.clear(),
            1 => s.revisions[0].basis.clear(),
            2 => s.revisions[0].source_reference.clear(),
            3 => s.revisions[0].changes[0].previous_bps -= 1,
            4 => {
                s.revisions[0].changes.pop();
            }
            5 => s.revisions[0].changes.swap(0, 1),
            6 => s.revisions[0].boundary = 3,
            7 => s.pools[0].availability_bps[3] -= 1,
            8 => s.pools[0].working_days[4] += 1,
            9 => s.rules[0].delivered[4] -= 1,
            10 => s.pools[0].availability_bps[4] = 10001,
            _ => s.registry.parameters[0].source_version = "2".into(),
        }
        assert!(
            preview_continuation(before.clone(), next).is_err(),
            "mode {mode}"
        );
    }
}
#[test]
fn service_revision_cannot_be_injected_as_initial_history() {
    let (d, _, _, repaired) = feedback_fixture();
    assert!(preview(d, repaired).is_err());
}
#[test]
fn unserviceable_outage_keeps_need_and_has_no_successful_repair() {
    use qazna::service_feedback::repair_service_batch;
    let (d, baseline, mut observed, _) = feedback_fixture();
    let before = preview(d, baseline).unwrap();
    let s = observed.services.as_mut().unwrap();
    s.pools[0].availability_bps[4] = 0;
    s.revisions[0].changes[0].available_bps = 0;
    let saved = json!(observed);
    let result = repair_service_batch(&before, &observed, 1, 4, 12).unwrap();
    assert!(result.input.is_none());
    assert_eq!(result.attempts.len(), 9);
    assert!(result.attempts.iter().all(|r| r.rejection.is_some()));
    assert_eq!(json!(observed), saved);
}
#[test]
fn committed_work_and_prior_physical_rows_cannot_be_repaired_away() {
    use qazna::service_feedback::defer_batch;
    let (d, baseline, _, repaired) = feedback_fixture();
    let before = preview(d, baseline).unwrap();
    for mode in 0..3 {
        let mut next = repaired.clone();
        match mode {
            0 => next.plan[0].activities[7] -= 1,
            1 => next.plan[3].closing[3] += 1,
            _ => next.model.lives[0].initial_remaining_periods += 1,
        }
        assert!(preview_continuation(before.clone(), next).is_err());
    }
    let mut late = repaired;
    late.boundary = 7;
    assert!(defer_batch(&late, 1, 6, 8).is_err());
}
#[test]
fn second_revision_appends_without_rewriting_prior_evidence() {
    let (d, baseline, _, repaired) = feedback_fixture();
    let before = preview(d, baseline).unwrap();
    let second = preview_continuation(before, repaired.clone()).unwrap();
    let saved = json!(second);
    let mut next = repaired;
    next.boundary = 6;
    let s = next.services.as_mut().unwrap();
    s.pools[0].availability_bps[8] = 9999;
    s.revisions.push(ServiceRevision {
        boundary: 6,
        basis: "Declared additional absence".into(),
        source_reference: "qazna:test:absence:2".into(),
        changes: vec![AvailabilityChange {
            pool: s.pools[0].id.clone(),
            period: 8,
            previous_bps: 10000,
            available_bps: 9999,
        }],
    });
    assert!(preview_continuation(second.clone(), next.clone()).is_ok());
    for mode in 0..3 {
        let mut bad = next.clone();
        let revisions = &mut bad.services.as_mut().unwrap().revisions;
        match mode {
            0 => revisions[0].source_reference = "rewritten".into(),
            1 => {
                revisions.remove(0);
            }
            _ => {
                revisions.push(revisions[1].clone());
            }
        }
        assert!(preview_continuation(second.clone(), bad).is_err());
    }
    assert_eq!(json!(second), saved);
}
#[test]
fn availability_recovery_is_explicit_and_prefix_remains_fixed() {
    let (d, baseline, _, repaired) = feedback_fixture();
    let first = preview(d, baseline).unwrap();
    let second = preview_continuation(first, repaired.clone()).unwrap();
    let mut next = repaired;
    next.boundary = 5;
    let s = next.services.as_mut().unwrap();
    let prior = s.pools[0].availability_bps[5];
    s.pools[0].availability_bps[5] = 10000;
    s.revisions.push(ServiceRevision {
        boundary: 5,
        basis: "Earlier recovery declared".into(),
        source_reference: "qazna:test:recovery".into(),
        changes: vec![AvailabilityChange {
            pool: s.pools[0].id.clone(),
            period: 5,
            previous_bps: prior,
            available_bps: 10000,
        }],
    });
    assert!(preview_continuation(second, next).is_ok());
}
#[test]
fn repair_search_bounds_and_bad_indexes_fail_without_panic() {
    use qazna::service_feedback::{defer_batch, repair_service_batch};
    let (d, baseline, observed, _) = feedback_fixture();
    let before = preview(d, baseline).unwrap();
    for (process, start, latest) in [(999, 4, 6), (1, 4, 28), (1, 3, 6), (1, 4, 3), (1, 4, 60)] {
        assert!(repair_service_batch(&before, &observed, process, start, latest).is_err());
    }
    assert!(defer_batch(&observed, 1, 4, usize::MAX).is_err());
}

#[test]
fn availability_grid_matches_independent_workforce_inequalities() {
    use qazna::service_feedback::repair_service_batch;
    let (d, baseline, observed, _) = feedback_fixture();
    let before = preview(d, baseline.clone()).unwrap();
    let n = baseline.plan[4].activities[1] as u128;
    let s = baseline.services.as_ref().unwrap();
    let stock = baseline.model.seasonal.physical.initial[s.pools[0].resource] as u128;
    let needs = &s.rules[0].delivered;
    for bps in [0u16, 9800, 9820, 9830, 9840, 9850, 9860, 9900, 9999, 10000] {
        let mut next = observed.clone();
        let s = next.services.as_mut().unwrap();
        s.pools[0].availability_bps[4] = bps;
        s.pools[0].availability_bps[5] = bps;
        for c in &mut s.revisions[0].changes {
            if c.pool == "north_care_staff" {
                c.available_bps = bps;
            }
        }
        s.revisions[0]
            .changes
            .retain(|c| c.previous_bps != c.available_bps);
        let expected = (4..=12).find(|&start| {
            (0..60).all(|t| {
                let active = stock + if t > start + 6 { n } else { 0 };
                let available =
                    active * 9600 * u128::from(if t == 4 || t == 5 { bps } else { 10000 }) / 10000;
                let mentor = if (start..start + 6).contains(&t) {
                    n * 120
                } else {
                    0
                };
                u128::from(needs[t]) + mentor <= available
            })
        });
        let result = repair_service_batch(&before, &next, 1, 4, 12).unwrap();
        let found = result
            .attempts
            .iter()
            .find(|a| a.rejection.is_none())
            .map(|a| a.start);
        assert_eq!(found, expected, "availability {bps}");
        assert_eq!(result.input.is_some(), expected.is_some());
    }
}

#[test]
fn repaired_service_circulation_conserves_measure_and_moves_only_development_inputs() {
    use qazna::coupled::{self, CoupledCheckpoint, CoupledDefinition, EventKind};
    let v: Value = serde_json::from_str(include_str!("../research/service-results.json")).unwrap();
    let (d, baseline, _, repaired) = feedback_fixture();
    let before = qazna::model_scenarios::preview(d, baseline.clone()).unwrap();
    let after =
        qazna::model_scenarios::preview_continuation(before.clone(), repaired.clone()).unwrap();
    let policy = serde_json::from_value(v["feedback"]["circulation"]["policy"].clone()).unwrap();
    let def = CoupledDefinition {
        id: uuid::Uuid::new_v4(),
        format_version: 1,
        model_id: before.definition.id,
        policy,
        basis: "Fixed policy for conservation test".into(),
    };
    let base = coupled::preview(def.clone(), before, vec![]).unwrap();
    let next = coupled::preview_continuation(base.clone(), after.clone(), vec![]).unwrap();
    let a: CoupledCheckpoint = serde_json::from_str(base.checkpoint.as_ref().unwrap()).unwrap();
    let b: CoupledCheckpoint = serde_json::from_str(next.checkpoint.as_ref().unwrap()).unwrap();
    let initial: Vec<i128> = def
        .policy
        .opening
        .iter()
        .map(|x| x.parse().unwrap())
        .collect();
    let total: i128 = initial.iter().sum();
    for evidence in [&a.evidence, &b.evidence] {
        let mut balances = initial.clone();
        for e in &evidence.events {
            let n: i128 = e.amount.parse().unwrap();
            balances[e.from] -= n;
            balances[e.to] += n;
            assert!(balances.iter().all(|&n| n >= 0));
            assert_eq!(balances.iter().sum::<i128>(), total);
        }
        assert_eq!(
            balances.iter().map(ToString::to_string).collect::<Vec<_>>(),
            evidence.money.closing
        );
    }
    let activity = baseline.plan[4].activities[1];
    let p = &baseline.model.seasonal.physical.processes[1];
    for (resource, supplier) in [(3, 5), (4, 6)] {
        let amount = (activity * p.inputs[resource]).to_string();
        for (evidence, period) in [(&a.evidence, 4), (&b.evidence, 6)] {
            let matches: Vec<_> = evidence
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Goods && e.from == 8 && e.to == supplier)
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].amount, amount);
            assert_eq!(matches[0].period, period);
        }
    }
    assert_eq!(a.evidence.money.closing, b.evidence.money.closing);
    assert_eq!(json!(def.policy), v["feedback"]["circulation"]["policy"]);
    let blocked = v["feedback"]["circulation"]["blocked_event"]
        .as_u64()
        .unwrap() as usize;
    let fail = coupled::preview_continuation(base, after, vec![blocked]).unwrap();
    let fail: CoupledCheckpoint = serde_json::from_str(fail.checkpoint.as_ref().unwrap()).unwrap();
    assert!(!fail.evidence.all_scheduled_payments_settled);
    assert!(
        fail.evidence
            .failed
            .iter()
            .any(|e| e.reason == "dependency")
    );
    assert!(
        fail.evidence
            .failed
            .iter()
            .any(|e| e.reason == "insufficient_measure")
    );
    assert_eq!(
        json!(fail.evidence.failed),
        v["feedback"]["circulation"]["blocked_failures"]
    );
}

#[test]
fn incident_union_is_order_independent_and_preserves_exact_cells() {
    use qazna::service_feedback::combine_service_revisions;
    let (_, baseline, observed, _) = feedback_fixture();
    let base = baseline.services.as_ref().unwrap();
    let all = observed.services.as_ref().unwrap().revisions[0].clone();
    let mut a = all.clone();
    a.changes.truncate(2);
    let mut b = all.clone();
    b.changes.drain(..2);
    let merge = |r: &[ServiceRevision]| {
        combine_service_revisions(base, r, "Combine".into(), "test:union".into()).unwrap()
    };
    let x = merge(&[a.clone(), b.clone()]);
    assert_eq!(x.changes, all.changes);
    assert_eq!(x, merge(&[b.clone(), a.clone()]));
    assert_eq!(x, merge(&[a.clone(), b, a]));
    assert!(base.revisions.is_empty());
    assert_eq!(
        base.with_revision(x).unwrap().pools,
        observed.services.as_ref().unwrap().pools
    );
}
#[test]
fn incident_union_rejects_conflict_and_unexplained_or_historical_cells() {
    use qazna::service_feedback::combine_service_revisions;
    let (_, baseline, observed, _) = feedback_fixture();
    let base = baseline.services.as_ref().unwrap();
    let a = observed.services.as_ref().unwrap().revisions[0].clone();
    let merge = |r: &[ServiceRevision]| {
        combine_service_revisions(base, r, "Combine".into(), "test:union".into())
    };
    let mut b = a.clone();
    b.changes[0].available_bps -= 1;
    assert!(merge(&[a.clone(), b]).is_err());
    let mut b = a.clone();
    b.boundary = 5;
    assert!(merge(&[a.clone(), b]).is_err());
    let mut b = a.clone();
    b.changes[0].previous_bps = 0;
    assert!(merge(&[a.clone(), b]).is_err());
    let mut b = a.clone();
    b.changes.clear();
    assert!(merge(&[a.clone(), b]).is_err());
    assert!(merge(&[]).is_err());
    assert!(merge(std::slice::from_ref(&a)).is_err());
    assert!(merge(&vec![a; 17]).is_err());
}
#[test]
fn compounded_absence_preserves_need_and_reports_failed_repair_without_money() {
    use qazna::service_feedback::{combine_service_revisions, defer_batch, repair_service_batch};
    let v: Value = serde_json::from_str(include_str!("../research/service-results.json")).unwrap();
    let c = &v["feedback"]["compound"];
    let (d, baseline, _, _) = feedback_fixture();
    let declarations: Vec<ServiceRevision> =
        serde_json::from_value(c["declarations"].clone()).unwrap();
    let stored: ServiceRevision = serde_json::from_value(c["revision"].clone()).unwrap();
    let revision = combine_service_revisions(
        baseline.services.as_ref().unwrap(),
        &declarations,
        stored.basis.clone(),
        stored.source_reference.clone(),
    )
    .unwrap();
    assert_eq!(revision, stored);
    let before = preview(d.clone(), baseline.clone()).unwrap();
    let mut input = baseline.clone();
    input.boundary = 4;
    input.services = Some(
        baseline
            .services
            .as_ref()
            .unwrap()
            .with_revision(revision)
            .unwrap(),
    );
    let repair = repair_service_batch(&before, &input, 1, 4, 12).unwrap();
    assert_eq!(json!(repair), c["repair"]);
    assert_eq!(repair.attempts.len(), 9);
    assert!(repair.attempts.iter().all(|a| a.rejection.is_some()));
    assert!(repair.input.is_none());
    let candidate = defer_batch(&input, 1, 4, 8).unwrap();
    assert!(preview_continuation(before.clone(), candidate.clone()).is_err());
    assert_eq!(json!(&candidate.plan[..4]), json!(&baseline.plan[..4]));
    assert_eq!(
        candidate.model.seasonal.physical.initial,
        baseline.model.seasonal.physical.initial
    );
    let population = candidate
        .population
        .as_ref()
        .unwrap()
        .audit(&d, &candidate.model, &candidate.plan)
        .unwrap();
    let checkpoint: ModelCheckpoint =
        serde_json::from_str(before.checkpoint.as_ref().unwrap()).unwrap();
    assert_eq!(
        json!(population),
        json!(checkpoint.population.as_ref().unwrap())
    );
    let evidence = candidate
        .services
        .as_ref()
        .unwrap()
        .audit(&d, &candidate.model, &candidate.plan, &population)
        .unwrap();
    assert_eq!(json!(evidence), c["service_evidence"]);
    assert!(!evidence.all_selected_services_feasible);
    assert!(c["money"].is_null());
    assert_eq!(
        c["policy_unchanged"],
        v["feedback"]["circulation"]["policy"]
    );
}
