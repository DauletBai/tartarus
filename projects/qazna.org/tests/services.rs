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
