use qazna::{
    model_scenarios::{ModelDefinition, ModelInput},
    population::*,
};
use serde_json::Value;
fn fixture() -> PopulationStudy {
    serde_json::from_str(include_str!("../fixtures/population-study.json")).unwrap()
}
fn fixed(s: &mut PopulationStudy, id: &str, n: u64, d: u64) {
    let p = s
        .registry
        .parameters
        .iter_mut()
        .find(|p| p.id == id)
        .unwrap();
    let v = Ratio {
        numerator: n,
        denominator: d,
    };
    p.kind = EvidenceKind::Synthetic;
    p.estimate = Some(Estimate {
        lower: v,
        central: v,
        upper: v,
    });
}
fn small(ages: &[(usize, u64)], months: usize) -> PopulationStudy {
    let mut s = fixture();
    s.initial.clear();
    s.months.truncate(months);
    let original = s
        .registry
        .parameters
        .iter()
        .find(|p| p.unit == "person")
        .unwrap()
        .clone();
    for (i, &(age, n)) in ages.iter().enumerate() {
        let mut p = original.clone();
        p.id = format!("test_count_{i}");
        s.registry.parameters.push(p);
        fixed(&mut s, &format!("test_count_{i}"), n, 1);
        s.initial.push(InitialCohort {
            age_months: age,
            people_parameter: format!("test_count_{i}"),
        });
    }
    fixed(&mut s, "births", 0, 1);
    for m in &mut s.months {
        m.migration.clear();
    }
    for g in s.groups.clone() {
        fixed(&mut s, &g.mortality_parameter, 0, 1);
    }
    s
}
fn n(s: &str) -> u64 {
    s.parse().unwrap()
}
#[test]
fn all_variants_conserve_people_and_real_calendar_days() {
    let mut finals = vec![];
    for v in [
        Variant::LowerPopulation,
        Variant::Reference,
        Variant::HigherPopulation,
    ] {
        let mut s = fixture();
        s.variant = v;
        let p = s.project().unwrap();
        let mut previous = 100_000_000;
        assert_eq!(n(&p.initial), previous);
        assert_eq!(p.months[13].calendar, "2028-02");
        assert_eq!(p.months[13].days, 29);
        assert_eq!(p.months[1].days, 28);
        for m in p.months {
            assert_eq!(n(&m.opening), previous);
            assert_eq!(m.group_people.iter().map(|x| n(x)).sum::<u64>(), previous);
            assert_eq!(
                previous + n(&m.births) + n(&m.arrivals),
                n(&m.closing) + n(&m.deaths) + n(&m.departures)
            );
            previous = n(&m.closing);
        }
        assert_eq!(
            p.final_age_counts.iter().map(|x| n(x)).sum::<u64>(),
            previous
        );
        finals.push(previous);
    }
    assert!(finals[0] < finals[1] && finals[1] < finals[2]);
}
#[test]
fn month_age_boundary_changes_profile_without_losing_people() {
    let s = small(&[(35, 10)], 2);
    let p = s.project().unwrap();
    assert_eq!(p.months[0].group_people[0], "10");
    assert_eq!(p.months[1].group_people[1], "10");
    let d = |i: usize| {
        p.months[i]
            .demands
            .iter()
            .find(|d| d.item == "preschool_capacity")
            .unwrap()
            .central
            .clone()
            .unwrap()
    };
    assert_eq!(d(0), "0");
    assert_eq!(d(1), "280");
    assert_eq!(p.final_population, "10");
}
#[test]
fn oldest_open_age_bin_is_retained_not_forced_to_die() {
    let p = small(&[(LAST_AGE - 1, 4), (LAST_AGE, 6)], 3)
        .project()
        .unwrap();
    assert_eq!(p.final_population, "10");
    assert_eq!(p.final_age_counts[LAST_AGE], "10");
}
#[test]
fn mortality_fraction_carry_does_not_vanish_in_small_cohorts() {
    let mut s = small(&[(1, 10)], 10);
    fixed(&mut s, "early_childhood_mortality", 1, 100);
    let p = s.project().unwrap();
    assert_eq!(p.final_population, "9");
    assert!(p.months[..9].iter().all(|m| m.deaths == "0"));
    assert_eq!(p.months[9].deaths, "1");
}
#[test]
fn mortality_total_is_independent_of_age_bin_fragmentation() {
    let mut a = small(&[(1, 10)], 10);
    let mut b = small(&[(1, 5), (2, 5)], 10);
    for s in [&mut a, &mut b] {
        fixed(s, "early_childhood_mortality", 1, 100);
    }
    assert_eq!(
        a.project().unwrap().final_population,
        b.project().unwrap().final_population
    );
}
#[test]
fn hamilton_rounding_has_deterministic_largest_remainder_allocation() {
    let mut s = small(&[(1, 5), (2, 4), (3, 1)], 1);
    fixed(&mut s, "early_childhood_mortality", 1, 3);
    let p = s.project().unwrap();
    assert_eq!(p.months[0].deaths, "3");
    assert_eq!(&p.final_age_counts[2..5], &["3", "3", "1"]);
}
#[test]
fn exhaustive_small_mortality_cases_match_independent_count_formula() {
    for a in 0..8 {
        for b in 0..8 {
            for numerator in 0..=4 {
                let mut s = small(&[(1, a), (2, b)], 1);
                fixed(&mut s, "early_childhood_mortality", numerator, 4);
                let p = s.project().unwrap();
                let dead = (a + b) * numerator / 4;
                assert_eq!(n(&p.months[0].deaths), dead);
                assert_eq!(n(&p.final_population), a + b - dead);
            }
        }
    }
}
#[test]
fn births_and_migration_enter_next_month_and_count_conserves() {
    let mut s = small(&[(360, 20)], 2);
    fixed(&mut s, "births", 3, 1);
    fixed(&mut s, "arrivals", 5, 1);
    fixed(&mut s, "departures", 7, 1);
    s.months[0].migration = vec![Migration {
        age_months: 360,
        arrivals_parameter: "arrivals".into(),
        departures_parameter: "departures".into(),
    }];
    let p = s.project().unwrap();
    assert_eq!(p.months[0].opening, "20");
    assert_eq!(p.months[0].closing, "21");
    assert_eq!(p.months[1].group_people[0], "3");
    assert_eq!(p.final_population, "24");
}
#[test]
fn migration_cannot_remove_unavailable_people_or_duplicate_age() {
    let mut s = small(&[(360, 1)], 1);
    fixed(&mut s, "arrivals", 0, 1);
    fixed(&mut s, "departures", 2, 1);
    s.months[0].migration = vec![Migration {
        age_months: 360,
        arrivals_parameter: "arrivals".into(),
        departures_parameter: "departures".into(),
    }];
    assert!(s.project().is_err());
    fixed(&mut s, "departures", 0, 1);
    let duplicate = s.months[0].migration[0].clone();
    s.months[0].migration.push(duplicate);
    assert!(s.project().is_err());
}
#[test]
fn unknown_norms_are_not_zero_and_group_surplus_cannot_fill_them() {
    let mut s = small(&[(10, 10), (300, 10)], 1);
    fixed(&mut s, "adult_protein", 1000, 1);
    let p = s.project().unwrap();
    let d = p.months[0]
        .demands
        .iter()
        .find(|d| d.item == "protein")
        .unwrap();
    assert_eq!(d.known_lower, "310000");
    assert!(d.central.is_none() && d.upper.is_none());
    assert_eq!(d.unknown_parameters, vec!["early_childhood_protein"]);
    assert!(!p.all_selected_needs_known);
}
#[test]
fn empty_population_has_zero_need_without_inventing_missing_coefficients() {
    let p = small(&[(0, 0)], 1).project().unwrap();
    assert!(p.all_selected_needs_known);
    assert!(
        p.months[0]
            .demands
            .iter()
            .all(|d| d.central.as_deref() == Some("0"))
    );
}
#[test]
fn rational_need_is_rounded_up_only_after_profile_exposure() {
    let mut s = small(&[(10, 10)], 1);
    fixed(&mut s, "early_childhood_rice_component", 1, 3);
    let p = s.project().unwrap();
    assert_eq!(p.months[0].demands[0].central.as_deref(), Some("104"));
}
#[test]
fn registry_rejects_units_dates_duplicates_unknowns_and_bad_intervals() {
    for mode in 0..7 {
        let mut s = fixture();
        match mode {
            0 => s.registry.parameters[0].recorded_on = "2027-02-29".into(),
            1 => s.registry.parameters.push(s.registry.parameters[0].clone()),
            2 => s.registry.parameters[0].unit = "kg".into(),
            3 => {
                s.registry.parameters[0]
                    .estimate
                    .as_mut()
                    .unwrap()
                    .central
                    .denominator = 0
            }
            4 => s.registry.parameters[0].kind = EvidenceKind::Unknown,
            5 => {
                s.registry.parameters[0]
                    .estimate
                    .as_mut()
                    .unwrap()
                    .lower
                    .numerator = u64::MAX
            }
            _ => {
                let p = s
                    .registry
                    .parameters
                    .iter_mut()
                    .find(|p| p.id == "births")
                    .unwrap();
                p.estimate.as_mut().unwrap().upper.numerator = 1;
            }
        }
        assert!(s.project().is_err(), "mode {mode}");
    }
}
#[test]
fn attributed_mode_never_accepts_synthetic_coefficients_by_changing_mode_alone() {
    let mut s = fixture();
    s.registry.mode = InputMode::SourceAttributed;
    assert!(s.project().is_err());
}
#[test]
fn partition_and_need_matrix_require_complete_unique_coverage() {
    for mode in 0..6 {
        let mut s = fixture();
        match mode {
            0 => s.groups[1].start_month += 1,
            1 => s.groups[1].start_month -= 1,
            2 => s.groups[4].end_month_exclusive -= 1,
            3 => {
                s.need_rules.pop();
            }
            4 => s.need_rules[1] = s.need_rules[0].clone(),
            _ => s.need_rules[0].parameter = "missing".into(),
        };
        assert!(s.project().is_err());
    }
}
#[test]
fn death_rate_above_one_fractional_people_and_oversized_inputs_fail() {
    for mode in 0..5 {
        let mut s = small(&[(10, 10)], 1);
        match mode {
            0 => fixed(&mut s, "early_childhood_mortality", 2, 1),
            1 => fixed(&mut s, "births", 1, 2),
            2 => s.initial[0].age_months = LAST_AGE + 1,
            3 => fixed(&mut s, "test_count_0", 20_000_000_001, 1),
            _ => s.months = vec![s.months[0].clone(); 121],
        };
        assert!(s.project().is_err());
    }
}
fn bound() -> (ModelDefinition, ModelInput) {
    let v: Value =
        serde_json::from_str(include_str!("../fixtures/extended-scenario.json")).unwrap();
    let mut d: ModelDefinition = serde_json::from_value(v["definition"].clone()).unwrap();
    d.time_origin = "2027-01".into();
    let mut i: ModelInput = serde_json::from_value(v["input"].clone()).unwrap();
    let study = fixture();
    let p = study.project().unwrap();
    for (t, m) in p.months.iter().enumerate() {
        i.model.seasonal.physical.periods[t].needs[6] =
            n(m.demands[0].central.as_ref().unwrap()) as i64;
    }
    i.population = Some(PopulationBinding {
        profiles: None,
        study,
        mappings: vec![NeedMapping {
            item: "rice_component".into(),
            resource: 6,
        }],
    });
    (d, i)
}
#[test]
fn prospective_binding_exposes_unmet_need_and_full_model_rejects_it() {
    let (d, i) = bound();
    let a = i
        .population
        .as_ref()
        .unwrap()
        .audit(&d, &i.model, &i.plan)
        .unwrap();
    assert_eq!(a.coverage.len(), 60);
    assert!(a.coverage.iter().all(|c| n(&c.unmet) > 0));
    assert_eq!(a.unmapped_items.len(), 7);
    assert!(qazna::model_scenarios::preview(d, i).is_err());
}
#[test]
fn physical_binding_rejects_lowered_needs_calendar_units_unknowns_and_duplicate_mapping() {
    for mode in 0..5 {
        let (mut d, mut i) = bound();
        match mode {
            0 => i.model.seasonal.physical.periods[0].needs[6] -= 1,
            1 => d.time_origin = "2027-02".into(),
            2 => d.resources[6].unit = "g".into(),
            3 => i.population.as_mut().unwrap().mappings[0].item = "protein".into(),
            _ => {
                let b = i.population.as_mut().unwrap();
                b.mappings.push(b.mappings[0].clone());
            }
        }
        assert!(
            i.population
                .as_ref()
                .unwrap()
                .audit(&d, &i.model, &i.plan)
                .is_err()
        );
    }
}
#[test]
fn unexpected_fields_and_negative_quantities_are_not_accepted() {
    let mut v = serde_json::to_value(fixture()).unwrap();
    v["override"] = "trusted".into();
    assert!(serde_json::from_value::<PopulationStudy>(v).is_err());
    let mut v = serde_json::to_value(fixture()).unwrap();
    v["initial"][0]["age_months"] = (-1).into();
    assert!(serde_json::from_value::<PopulationStudy>(v).is_err());
}
