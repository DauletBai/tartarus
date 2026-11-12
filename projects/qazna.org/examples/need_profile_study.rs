use qazna::{model_scenarios::*, need_profiles::*, population::*};
use serde_json::{Value, json};
use std::{env, fs};
fn parameter(registry: &mut Registry, id: &str, unit: &str, n: u64) {
    let r = Ratio {
        numerator: n,
        denominator: 1,
    };
    registry.parameters.push(Parameter {
        id: id.into(),
        unit: format!("{unit}/person/day"),
        region: "scenario regions".into(),
        reference_period: "2027-2031".into(),
        recorded_on: "2026-11-12".into(),
        source: "qazna:need-profiles:v1".into(),
        source_version: "1".into(),
        basis: "Planning workload coefficient; see docs/input-assumptions.md".into(),
        kind: EvidenceKind::Synthetic,
        estimate: Some(Estimate {
            lower: r,
            central: r,
            upper: r,
        }),
    });
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 2);
    let old: Value =
        serde_json::from_slice(&fs::read("research/population-results.json").unwrap()).unwrap();
    let definition: ModelDefinition =
        serde_json::from_value(old["runtime_fixture"]["definition"].clone()).unwrap();
    let mut input: ModelInput =
        serde_json::from_value(old["runtime_fixture"]["input"].clone()).unwrap();
    let study = input.population.as_ref().unwrap().study.clone();
    let population = study.project().unwrap();
    let profiles = [
        ("general", LifeStage::General, Support::Standard),
        ("assisted", LifeStage::General, Support::Assisted),
        ("pregnancy_1", LifeStage::PregnancyFirst, Support::Standard),
        ("pregnancy_2", LifeStage::PregnancySecond, Support::Standard),
        ("pregnancy_3", LifeStage::PregnancyThird, Support::Standard),
        ("postpartum", LifeStage::Postpartum, Support::Standard),
        (
            "pregnancy_assisted",
            LifeStage::PregnancySecond,
            Support::Assisted,
        ),
    ]
    .map(|(id, stage, support)| Profile {
        id: id.into(),
        stage,
        support,
    })
    .to_vec();
    let mut plan = ProfilePlan {
        registry: Registry {
            version: "need-profiles-v1".into(),
            mode: InputMode::Illustrative,
            parameters: vec![],
        },
        exposure_source: ExposureSource {
            source: "qazna:need-profiles:v1".into(),
            source_version: "1".into(),
            recorded_on: "2026-11-12".into(),
            kind: EvidenceKind::Synthetic,
        },
        regions: vec!["north".into(), "south".into()],
        profiles,
        cells: vec![],
        rules: vec![],
        local_delivery: vec![],
    };
    for n in [0, 2, 3, 4, 5, 10, 15, 60, 62] {
        parameter(&mut plan.registry, &format!("care_{n}"), "minute", n);
    }
    for n in [2, 3, 4] {
        parameter(&mut plan.registry, &format!("water_{n}"), "litre", n);
    }
    for (gi, g) in study.groups.iter().enumerate() {
        for ri in 0..2 {
            let names = if gi == 3 {
                vec![
                    "general",
                    "assisted",
                    "pregnancy_1",
                    "pregnancy_2",
                    "pregnancy_3",
                    "postpartum",
                    "pregnancy_assisted",
                ]
            } else {
                vec!["general", "assisted"]
            };
            let start = plan.cells.len();
            for name in &names {
                plan.cells.push(ExposureCell {
                    region: plan.regions[ri].clone(),
                    group: g.id.clone(),
                    profile: (*name).into(),
                    person_days: vec![],
                });
            }
            for m in &population.months {
                let total = m.group_people[gi].parse::<u64>().unwrap() * u64::from(m.days);
                let local = if ri == 0 {
                    total * 2 / 5
                } else {
                    total - total * 2 / 5
                };
                let mut extra = 0;
                for (i, name) in names.iter().enumerate().skip(1) {
                    let n = match *name {
                        "assisted" => local / 10,
                        "pregnancy_assisted" => local / 500,
                        _ => local / 100,
                    };
                    plan.cells[start + i].person_days.push(n);
                    extra += n;
                }
                plan.cells[start].person_days.push(local - extra);
            }
            for (i, name) in names.iter().enumerate() {
                for item in &study.items {
                    let norm = match item.id.as_str() {
                        "water_component" => Norm::Replace {
                            parameter: format!(
                                "water_{}",
                                2 + ri + usize::from(name.starts_with("pregnancy"))
                            ),
                        },
                        "care_time" => Norm::Replace {
                            parameter: format!(
                                "care_{}",
                                match *name {
                                    "assisted" => 60,
                                    "pregnancy_assisted" => 62,
                                    "pregnancy_1" | "pregnancy_2" | "pregnancy_3" => 2,
                                    "postpartum" => 5,
                                    _ => [10, 0, 0, 0, 15][gi],
                                }
                            ),
                        },
                        _ => Norm::Inherit {},
                    };
                    plan.rules.push(ProfileRule {
                        cell: start + i,
                        item: item.id.clone(),
                        norm,
                    });
                }
            }
        }
    }
    let evidence = plan.project(&study).unwrap();
    for region in &plan.regions {
        let quantities = evidence
            .months
            .iter()
            .map(|m| {
                m.regions
                    .iter()
                    .find(|r| &r.region == region)
                    .unwrap()
                    .demands
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
        plan.local_delivery.push(LocalDelivery {
            region: region.clone(),
            item: "water_component".into(),
            quantities,
        });
    }
    let water: Vec<i64> = evidence
        .months
        .iter()
        .map(|m| {
            m.total
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
    let mut stock = initial;
    input.model.seasonal.physical.initial[0] = initial;
    input.model.seasonal.physical.storage_caps[0] = initial;
    for (t, &n) in water.iter().enumerate() {
        stock -= n;
        input.model.seasonal.physical.periods[t].needs[0] = n;
        input.plan[t].deliveries[0] = n;
        input.plan[t].closing[0] = stock;
    }
    input.population.as_mut().unwrap().profiles = Some(plan);
    let checked = preview(definition.clone(), input.clone()).unwrap();
    let checkpoint: ModelCheckpoint =
        serde_json::from_str(checked.checkpoint.as_ref().unwrap()).unwrap();
    let mut wrong = input.clone();
    let local = &mut wrong
        .population
        .as_mut()
        .unwrap()
        .profiles
        .as_mut()
        .unwrap()
        .local_delivery;
    local[0].quantities[0] += 1;
    local[1].quantities[0] -= 1;
    let deficit = wrong
        .population
        .as_ref()
        .unwrap()
        .audit(&definition, &wrong.model, &wrong.plan)
        .unwrap();
    let error = preview(definition.clone(), wrong).unwrap_err().to_string();
    let profile = checkpoint
        .population
        .as_ref()
        .unwrap()
        .profiles
        .as_ref()
        .unwrap();
    let totals: Vec<_> = study
        .items
        .iter()
        .map(|item| {
            let known = profile.months.iter().all(|m| {
                m.total
                    .iter()
                    .find(|d| d.item == item.id)
                    .unwrap()
                    .central
                    .is_some()
            });
            let total = known.then(|| {
                profile
                    .months
                    .iter()
                    .map(|m| {
                        m.total
                            .iter()
                            .find(|d| d.item == item.id)
                            .unwrap()
                            .central
                            .as_ref()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    })
                    .sum::<u64>()
                    .to_string()
            });
            json!({"item":item.id,"unit":item.unit,"five_year_total":total})
        })
        .collect();
    let output = json!({"schema_version":1,"assumptions":"docs/input-assumptions.md#regional-and-life-stage-profiles","runtime_fixture":{"definition":definition,"input":input,"population_evidence":checkpoint.population},"totals":totals,"regional_counterexample":{"national_water_delivery_unchanged":true,"local_gaps":deficit.profiles.unwrap().local_coverage.into_iter().filter(|c|c.unmet!="0").collect::<Vec<_>>(),"runtime_rejection":error}});
    fs::write(&args[1], serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    println!(
        "Verified 60 months, 2 regions, 7 joint profiles, complete exposure and regional provision rejection"
    );
}
