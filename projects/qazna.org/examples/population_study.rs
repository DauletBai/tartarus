//! Reproducible illustrative population inputs and conditional demand projections.
//! No national data, clinical prescription or eligibility policy is inferred.
use qazna::{
    coupled::{CoupledCheckpoint, CoupledDefinition},
    model_scenarios::{ModelCheckpoint, ModelDefinition, ModelInput},
    population::*,
};
use serde_json::{Value, json};
use std::{env, fs};
use uuid::Uuid;
fn ratio(n: u64, d: u64) -> Ratio {
    Ratio {
        numerator: n,
        denominator: d,
    }
}
fn parameter(r: &mut Registry, id: &str, unit: &str, values: Option<Estimate>) {
    r.parameters.push(Parameter {
        id: id.into(),
        unit: unit.into(),
        region: "Synthetic 100-million population".into(),
        reference_period: "2027-2031 scenario; not an observation".into(),
        recorded_on: "2026-11-12".into(),
        source: "qazna:population-study:v1".into(),
        source_version: "1".into(),
        basis: if values.is_some() {
            "Illustrative test coefficient, not national statistics or medical guidance"
        } else {
            "Missing age-specific evidence; remains unknown, never zero"
        }
        .into(),
        kind: if values.is_some() {
            EvidenceKind::Synthetic
        } else {
            EvidenceKind::Unknown
        },
        estimate: values,
    });
}
fn fixed(r: &mut Registry, id: &str, unit: &str, n: u64, d: u64) {
    let v = ratio(n, d);
    parameter(
        r,
        id,
        unit,
        Some(Estimate {
            lower: v,
            central: v,
            upper: v,
        }),
    );
}
fn build() -> PopulationStudy {
    let mut registry = Registry {
        version: "population-inputs-v1".into(),
        mode: InputMode::Illustrative,
        parameters: vec![],
    };
    let definitions = [
        ("early_childhood", 0, 36, 4_000_000, 50),
        ("preschool", 36, 72, 4_000_000, 10),
        ("school_age", 72, 216, 16_000_000, 20),
        ("adult", 216, 780, 60_000_000, 200),
        ("older", 780, LAST_AGE + 1, 16_000_000, 3000),
    ];
    let mut initial = vec![];
    let mut groups = vec![];
    for (id, start, end, count, death) in definitions {
        let occupied_end = end.min(1080);
        let width = occupied_end - start;
        let base = count / width as u64;
        let extra = count % width as u64;
        for plus in 0..=1 {
            fixed(
                &mut registry,
                &format!("{id}_opening_{plus}"),
                "person",
                base + plus,
                1,
            );
        }
        for age in start..occupied_end {
            initial.push(InitialCohort {
                age_months: age,
                people_parameter: format!(
                    "{id}_opening_{}",
                    u64::from((age - start) < extra as usize)
                ),
            });
        }
        let mortality = format!("{id}_mortality");
        parameter(
            &mut registry,
            &mortality,
            "death/person/month",
            Some(Estimate {
                lower: ratio(death * 8, 10_000_000),
                central: ratio(death, 1_000_000),
                upper: ratio(death * 12, 10_000_000),
            }),
        );
        groups.push(AgeGroup {
            id: id.into(),
            start_month: start,
            end_month_exclusive: end,
            mortality_parameter: mortality,
        });
    }
    parameter(
        &mut registry,
        "births",
        "person/month",
        Some(Estimate {
            lower: ratio(100_000, 1),
            central: ratio(150_000, 1),
            upper: ratio(200_000, 1),
        }),
    );
    parameter(
        &mut registry,
        "arrivals",
        "person/month",
        Some(Estimate {
            lower: ratio(5000, 1),
            central: ratio(10000, 1),
            upper: ratio(15000, 1),
        }),
    );
    parameter(
        &mut registry,
        "departures",
        "person/month",
        Some(Estimate {
            lower: ratio(4000, 1),
            central: ratio(7000, 1),
            upper: ratio(10000, 1),
        }),
    );
    let months = (0..60)
        .map(|_| MonthInput {
            births_parameter: "births".into(),
            migration: vec![Migration {
                age_months: 360,
                arrivals_parameter: "arrivals".into(),
                departures_parameter: "departures".into(),
            }],
        })
        .collect();
    let items = [
        ("rice_component", "gross-equivalent kg"),
        ("oats_component", "gross-equivalent kg"),
        ("water_component", "litre"),
        ("preschool_capacity", "place-day"),
        ("school_capacity", "place-day"),
        ("care_time", "minute"),
        ("protein", "g"),
        ("vitamin_d", "microgram"),
    ]
    .map(|(id, unit)| NeedItem {
        id: id.into(),
        unit: unit.into(),
    })
    .to_vec();
    let mut rules = vec![];
    for (g, group) in groups.iter().enumerate() {
        for (j, item) in items.iter().enumerate() {
            let id = format!("{}_{}", group.id, item.id);
            let unit = format!("{}/person/day", item.unit);
            let value = match j {
                0 => Some([80, 180, 250, 300, 250][g]),
                1 => Some([50, 100, 150, 180, 150][g]),
                2 => Some(2000),
                3 => Some(if g == 1 { 1000 } else { 0 }),
                4 => Some(if g == 2 { 1000 } else { 0 }),
                _ => None,
            };
            if let Some(n) = value {
                let e = if j < 3 {
                    Estimate {
                        lower: ratio(n * 8, 10000),
                        central: ratio(n, 1000),
                        upper: ratio(n * 12, 10000),
                    }
                } else {
                    Estimate {
                        lower: ratio(n, 1000),
                        central: ratio(n, 1000),
                        upper: ratio(n, 1000),
                    }
                };
                parameter(&mut registry, &id, &unit, Some(e));
            } else {
                parameter(&mut registry, &id, &unit, None);
            }
            rules.push(NeedRule {
                group: group.id.clone(),
                item: item.id.clone(),
                parameter: id,
            });
        }
    }
    PopulationStudy {
        registry,
        variant: Variant::Reference,
        start_year: 2027,
        start_month: 1,
        initial,
        groups,
        months,
        items,
        need_rules: rules,
    }
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "fixture path and results path required");
    let study = build();
    let mut cases = vec![];
    for variant in [
        Variant::LowerPopulation,
        Variant::Reference,
        Variant::HigherPopulation,
    ] {
        let mut input = study.clone();
        input.variant = variant;
        cases.push(input.project().unwrap());
    }
    let original: Value =
        serde_json::from_slice(&fs::read("fixtures/extended-scenario.json").unwrap()).unwrap();
    let mut definition: ModelDefinition =
        serde_json::from_value(original["definition"].clone()).unwrap();
    definition.time_origin = "2027-01".into();
    let mut input: ModelInput = serde_json::from_value(original["input"].clone()).unwrap();
    let binding = PopulationBinding {
        study: study.clone(),
        mappings: vec![
            NeedMapping {
                item: "rice_component".into(),
                resource: 6,
            },
            NeedMapping {
                item: "oats_component".into(),
                resource: 7,
            },
        ],
    };
    for (t, month) in cases[1].months.iter().enumerate() {
        for map in &binding.mappings {
            let amount = month
                .demands
                .iter()
                .find(|d| d.item == map.item)
                .unwrap()
                .central
                .as_ref()
                .unwrap()
                .parse::<i64>()
                .unwrap();
            input.model.seasonal.physical.periods[t].needs[map.resource] = amount;
        }
    }
    input.population = Some(binding);
    let prospective = input
        .population
        .as_ref()
        .unwrap()
        .audit(&definition, &input.model, &input.plan)
        .unwrap();
    assert!(
        prospective
            .coverage
            .iter()
            .all(|c| c.unmet.parse::<u64>().unwrap() > 0)
    );
    let rejection = qazna::model_scenarios::preview(definition, input)
        .unwrap_err()
        .to_string();
    // A small explicit opening-stock fixture exercises the accepted runtime path.
    // This is a bounded water-component test, not a replenishing national supply system.
    let water: Vec<i64> = cases[1]
        .months
        .iter()
        .map(|m| {
            m.demands
                .iter()
                .find(|d| d.item == "water_component")
                .unwrap()
                .central
                .as_ref()
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect();
    let initial: i64 = water.iter().sum();
    let mut remaining = initial;
    let periods: Vec<_> = water
        .iter()
        .map(|&n| json!({"supplies":[0,0,0],"needs":[n,0,0],"capacities":[0]}))
        .collect();
    let plan:Vec<_>=water.iter().map(|&n|{remaining-=n;json!({"activities":[0],"deliveries":[n,0,0],"closing":[remaining,1,0],"discarded":[0,0,0]})}).collect();
    let runtime_definition:ModelDefinition=serde_json::from_value(json!({"id":Uuid::nil(),"format_version":1,"resources":[{"name":"water_component","unit":"litre"},{"name":"test_store","unit":"unit"},{"name":"unused_raw","unit":"unit"}],"period_unit":"month","time_origin":"2027-01","dated_bindings":[],"basis":"Synthetic finite opening stock; no refill or actual water supply claim"})).unwrap();
    let runtime_input:ModelInput=serde_json::from_value(json!({"model":{"seasonal":{"physical":{"resources":["water_component","test_store","unused_raw"],"initial":[initial,1,0],"storage_caps":[initial,1,0],"processes":[{"inputs":[0,0,1],"outputs":[1,0,0],"lead_periods":1}],"periods":periods,"objective_resource":0},"land":[{"name":"unused test land","available_hectares":vec![0;60]}],"crops":[],"irrigation_m3":vec![0;60],"closing_floors":vec![vec![0;3];60]},"limits":[],"lives":[{"resource":1,"service_periods":120,"initial_remaining_periods":120}],"stores":[]},"plan":plan,"boundary":18,"material_bases":[],"dated_generations":[],"population":{"study":study,"mappings":[{"item":"water_component","resource":0}]}})).unwrap();
    let checked =
        qazna::model_scenarios::preview(runtime_definition.clone(), runtime_input.clone()).unwrap();
    let checkpoint: ModelCheckpoint =
        serde_json::from_str(checked.checkpoint.as_ref().unwrap()).unwrap();
    let physical = qazna::model_scenarios::preview(
        serde_json::from_value(original["definition"].clone()).unwrap(),
        serde_json::from_value(original["input"].clone()).unwrap(),
    )
    .unwrap();
    let coupled = qazna::coupled::preview(
        CoupledDefinition {
            id: Uuid::nil(),
            format_version: 1,
            model_id: physical.definition.id,
            policy: serde_json::from_value(original["policy"].clone()).unwrap(),
            basis:
                "Original production and needs only; not approval of increased population demand"
                    .into(),
        },
        physical,
        vec![],
    )
    .unwrap();
    let money: CoupledCheckpoint =
        serde_json::from_str(coupled.checkpoint.as_ref().unwrap()).unwrap();
    let results = json!({"schema_version":1,"cases":cases,"upstream_counterexample":{"prospective_demand":prospective,"runtime_rejection":rejection,"original_payments_settle_for_original_needs":money.evidence.all_scheduled_payments_settled},"runtime_fixture":{"definition":runtime_definition,"input":runtime_input,"population_evidence":checkpoint.population},"limitations":["Illustrative coefficients, not a national projection or nutrition prescription","Opening-of-month exposure; births and arrivals enter needs from next month","Whole-person demographic apportionment and carried mortality fractions are modelling choices","Age-only cohorts do not model sex, fertility mechanisms, pregnancy, health/disability, household choice or regional flows","Unknown care/protein/vitamin data remain missing; prospective upstream mapping covers only two component needs","The old upstream plan fails full-needs approval for new demand; accepted runtime fixture uses a declared finite water stock only","Scenario variants and need intervals are conditional assumptions, not statistical confidence bounds"]});
    fs::write(&args[1], serde_json::to_vec_pretty(&study).unwrap()).unwrap();
    fs::write(&args[2], serde_json::to_vec_pretty(&results).unwrap()).unwrap();
    println!(
        "Verified 100-million opening population, 60 months, 3 variants and two physical need mappings"
    );
}
