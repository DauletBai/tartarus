use qazna::{model_scenarios::*, need_profiles::*, population::*};
use serde_json::{Value, json};
fn fixture() -> (ModelDefinition, ModelInput) {
    let r: Value =
        serde_json::from_str(include_str!("../research/need-profile-results.json")).unwrap();
    (
        serde_json::from_value(r["runtime_fixture"]["definition"].clone()).unwrap(),
        serde_json::from_value(r["runtime_fixture"]["input"].clone()).unwrap(),
    )
}
fn data() -> (PopulationStudy, ProfilePlan) {
    let (_, i) = fixture();
    let b = i.population.unwrap();
    (b.study, b.profiles.unwrap())
}
fn amount(e: &ProfileEvidence, t: usize, item: &str) -> u64 {
    e.months[t]
        .total
        .iter()
        .find(|d| d.item == item)
        .unwrap()
        .central
        .as_ref()
        .unwrap()
        .parse()
        .unwrap()
}
#[test]
fn complete_partition_conserves_each_age_group_and_calendar() {
    let (s, p) = data();
    let pop = s.project().unwrap();
    let e = p.project(&s).unwrap();
    for (t, m) in pop.months.iter().enumerate() {
        for (g, n) in s.groups.iter().zip(&m.group_people) {
            assert_eq!(
                p.cells
                    .iter()
                    .filter(|c| c.group == g.id)
                    .map(|c| c.person_days[t])
                    .sum::<u64>(),
                n.parse::<u64>().unwrap() * u64::from(m.days)
            );
        }
        let independent: u64 = p
            .cells
            .iter()
            .map(|c| {
                c.person_days[t]
                    * (if c.region == "north" { 2 } else { 3 }
                        + u64::from(c.profile.starts_with("pregnancy")))
            })
            .sum();
        assert_eq!(amount(&e, t, "water_component"), independent);
        assert_eq!(e.months[t].calendar, m.calendar);
    }
    assert_eq!(pop.months[13].days, 29);
    assert!(!e.all_selected_needs_known);
}
#[test]
fn joint_pregnancy_and_assisted_profile_is_counted_once() {
    let (s, p) = data();
    let e = p.project(&s).unwrap();
    let t = 0;
    let expected: u64 = p
        .cells
        .iter()
        .map(|c| {
            let gi = s.groups.iter().position(|g| g.id == c.group).unwrap();
            let coefficient = match c.profile.as_str() {
                "assisted" => 60,
                "pregnancy_assisted" => 62,
                "postpartum" => 5,
                "pregnancy_1" | "pregnancy_2" | "pregnancy_3" => 2,
                _ => [10, 0, 0, 0, 15][gi],
            };
            c.person_days[t] * coefficient
        })
        .sum();
    assert_eq!(amount(&e, t, "care_time"), expected);
    assert!(
        p.cells
            .iter()
            .any(|c| c.profile == "pregnancy_assisted" && c.person_days[0] > 0)
    );
}
#[test]
fn local_deficit_blocks_approval_despite_unchanged_national_delivery() {
    let (d, mut i) = fixture();
    assert!(preview(d.clone(), i.clone()).is_ok());
    let before = i.plan[0].deliveries.clone();
    let local = &mut i
        .population
        .as_mut()
        .unwrap()
        .profiles
        .as_mut()
        .unwrap()
        .local_delivery;
    local[0].quantities[0] += 1;
    local[1].quantities[0] -= 1;
    let a = i
        .population
        .as_ref()
        .unwrap()
        .audit(&d, &i.model, &i.plan)
        .unwrap();
    assert!(a.coverage.iter().all(|c| c.unmet == "0"));
    assert_eq!(
        a.profiles
            .unwrap()
            .local_coverage
            .iter()
            .filter(|c| c.unmet != "0")
            .count(),
        1
    );
    assert_eq!(before, i.plan[0].deliveries);
    assert!(
        preview(d, i)
            .unwrap_err()
            .to_string()
            .contains("regional provision")
    );
}
#[test]
fn regional_delivery_cannot_be_invented_or_omitted() {
    for mode in 0..4 {
        let (d, mut i) = fixture();
        let p = i.population.as_mut().unwrap().profiles.as_mut().unwrap();
        match mode {
            0 => p.local_delivery[0].quantities[0] += 1,
            1 => {
                p.local_delivery.pop();
            }
            2 => p.local_delivery[1] = p.local_delivery[0].clone(),
            _ => {
                p.local_delivery[0].quantities.pop();
            }
        }
        assert!(preview(d, i).is_err());
    }
}
#[test]
fn missing_extra_or_overflowed_exposure_is_rejected() {
    for mode in 0..3 {
        let (s, mut p) = data();
        p.cells[0].person_days[0] = match mode {
            0 => p.cells[0].person_days[0] - 1,
            1 => p.cells[0].person_days[0] + 1,
            _ => u64::MAX,
        };
        assert!(p.project(&s).is_err());
    }
}
#[test]
fn duplicate_and_incomplete_dimensions_are_rejected() {
    for mode in 0..7 {
        let (s, mut p) = data();
        match mode {
            0 => p.cells[1] = p.cells[0].clone(),
            1 => p.rules[1] = p.rules[0].clone(),
            2 => {
                p.rules.pop();
            }
            3 => p.regions[1] = p.regions[0].clone(),
            4 => p.profiles[1] = p.profiles[0].clone(),
            5 => p.cells[0].group = "missing".into(),
            _ => {
                p.cells[0].person_days.pop();
            }
        }
        assert!(p.project(&s).is_err(), "mode {mode}");
    }
}
#[test]
fn unknown_local_norm_propagates_and_cannot_become_zero() {
    let (d, mut i) = fixture();
    let p = i.population.as_mut().unwrap().profiles.as_mut().unwrap();
    let parameter = p
        .registry
        .parameters
        .iter_mut()
        .find(|p| p.id == "water_2")
        .unwrap();
    parameter.kind = EvidenceKind::Unknown;
    parameter.estimate = None;
    let study = &i.population.as_ref().unwrap().study;
    let p = i.population.as_ref().unwrap().profiles.as_ref().unwrap();
    let e = p.project(study).unwrap();
    assert!(
        e.months[0]
            .total
            .iter()
            .find(|d| d.item == "water_component")
            .unwrap()
            .central
            .is_none()
    );
    assert!(preview(d, i).is_err());
}
#[test]
fn zero_exposure_does_not_manufacture_unknown_need() {
    let (s, mut p) = data();
    let ix = p
        .cells
        .iter()
        .position(|c| c.profile == "pregnancy_assisted" && c.region == "north")
        .unwrap();
    let dest = p
        .cells
        .iter()
        .position(|c| c.group == p.cells[ix].group && c.region == "north" && c.profile == "general")
        .unwrap();
    for t in 0..s.months.len() {
        p.cells[dest].person_days[t] += p.cells[ix].person_days[t];
        p.cells[ix].person_days[t] = 0;
    }
    let mut unknown = p.registry.parameters[0].clone();
    unknown.id = "missing_zero_profile".into();
    unknown.kind = EvidenceKind::Unknown;
    unknown.estimate = None;
    p.registry.parameters.push(unknown);
    p.rules
        .iter_mut()
        .find(|r| r.cell == ix && r.item == "care_time")
        .unwrap()
        .norm = Norm::Replace {
        parameter: "missing_zero_profile".into(),
    };
    assert!(
        p.project(&s).unwrap().months[0]
            .total
            .iter()
            .find(|d| d.item == "care_time")
            .unwrap()
            .central
            .is_some()
    );
}
#[test]
fn same_norm_is_rounded_after_exposure_aggregation() {
    let (s, mut p) = data();
    let parameter = p
        .registry
        .parameters
        .iter_mut()
        .find(|p| p.id == "water_2")
        .unwrap();
    let ratio = Ratio {
        numerator: 1,
        denominator: 3,
    };
    parameter.estimate = Some(Estimate {
        lower: ratio,
        central: ratio,
        upper: ratio,
    });
    for r in &mut p.rules {
        if r.item == "water_component" {
            r.norm = Norm::Replace {
                parameter: "water_2".into(),
            };
        }
    }
    let e = p.project(&s).unwrap();
    for t in 0..s.months.len() {
        let expected: u64 = p
            .regions
            .iter()
            .map(|region| {
                p.cells
                    .iter()
                    .filter(|c| &c.region == region)
                    .map(|c| c.person_days[t])
                    .sum::<u64>()
                    .div_ceil(3)
            })
            .sum();
        assert_eq!(amount(&e, t, "water_component"), expected);
    }
}
#[test]
fn source_mode_units_and_date_are_validated() {
    for mode in 0..5 {
        let (s, mut p) = data();
        match mode {
            0 => p.exposure_source.recorded_on = "2027-02-29".into(),
            1 => p.exposure_source.kind = EvidenceKind::Unknown,
            2 => p.registry.parameters[0].unit = "wrong".into(),
            3 => p.registry.mode = InputMode::SourceAttributed,
            _ => p.exposure_source.source_version = "".into(),
        };
        assert!(p.project(&s).is_err());
    }
}
#[test]
fn profile_subdivision_preserves_demand_for_equal_norms() {
    let (s, p) = data();
    let before = p.project(&s).unwrap();
    let a = p
        .cells
        .iter()
        .position(|c| c.region == "north" && c.profile == "general" && c.group == s.groups[1].id)
        .unwrap();
    let b = p
        .cells
        .iter()
        .position(|c| c.region == "north" && c.profile == "assisted" && c.group == s.groups[1].id)
        .unwrap();
    for shift in 0..32 {
        let mut q = p.clone();
        q.cells[a].person_days[0] -= shift;
        q.cells[b].person_days[0] += shift;
        let after = q.project(&s).unwrap();
        assert_eq!(
            amount(&before, 0, "water_component"),
            amount(&after, 0, "water_component")
        );
        assert_eq!(
            amount(&before, 0, "preschool_capacity"),
            amount(&after, 0, "preschool_capacity")
        );
        assert_eq!(
            amount(&after, 0, "care_time") - amount(&before, 0, "care_time"),
            shift * 60
        );
    }
}
#[test]
fn imported_unknown_fields_and_negative_exposure_are_rejected() {
    let (_, p) = data();
    let mut v = json!(p);
    v["cells"][0]["person_days"][0] = json!(-1);
    assert!(serde_json::from_value::<ProfilePlan>(v).is_err());
    let mut v = json!(p);
    v["rules"][0]["norm"]["unreviewed"] = json!(true);
    assert!(serde_json::from_value::<ProfilePlan>(v).is_err());
}
