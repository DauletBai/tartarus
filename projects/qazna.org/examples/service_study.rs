use qazna::{
    capital::AssetLife,
    horizon::{PeriodPlan, Process},
    model_scenarios::*,
    population::*,
    rolling::RollingCapital,
    service_feedback::{defer_batch, repair_service_batch},
    services::*,
};
use serde_json::{Value, json};
use std::{env, fs};
fn parameter(r: &mut Registry, id: &str, unit: &str, n: u64, d: u64) {
    let q = Ratio {
        numerator: n,
        denominator: d,
    };
    r.parameters.push(Parameter {
        id: id.into(),
        unit: unit.into(),
        region: "scenario regions".into(),
        reference_period: "2027-2031".into(),
        recorded_on: "2026-11-12".into(),
        source: "qazna:service-capacity:v1".into(),
        source_version: "1".into(),
        basis: "Service capacity planning coefficient; docs/input-assumptions.md".into(),
        kind: EvidenceKind::Synthetic,
        estimate: Some(Estimate {
            lower: q,
            central: q,
            upper: q,
        }),
    });
}
fn circulation(before: &ModelVersion, after: &ModelVersion) -> Value {
    use qazna::coupled::{self, CirculationPolicy, CoupledCheckpoint, CoupledDefinition};
    fn evidence(v: &coupled::CoupledVersion) -> coupled::CoupledEvidence {
        serde_json::from_str::<CoupledCheckpoint>(v.checkpoint.as_ref().unwrap())
            .unwrap()
            .evidence
    }
    let c: ModelCheckpoint = serde_json::from_str(before.checkpoint.as_ref().unwrap()).unwrap();
    let h = &c.input.model.seasonal.physical;
    let r = h.resources.len();
    let measure: Vec<String> = (0..r)
        .map(|k| if [0, 3, 4].contains(&k) { "1" } else { "0" }.into())
        .collect();
    let mut d=CoupledDefinition {
        id:uuid::Uuid::from_u128(0x5131),format_version:1,model_id:before.definition.id,
        policy:CirculationPolicy {
            accounts:std::iter::once("common treasury".into()).chain(std::iter::once("aggregate provision".into())).chain(h.resources.iter().map(|r|format!("unit {r}"))).collect(),
            resource_suppliers:(2..r+2).collect(),
            process_operators:h.processes.iter().map(|p|p.outputs.iter().position(|&n|n>0).unwrap()+2).collect(),
            process_order:(0..h.processes.len()).collect(),
            capital_resources:c.input.model.lives.iter().map(|a|a.resource).collect(),
            common_measure:measure.clone(),unit_measure:measure,opening:vec!["0".into();r+2],
            measure_unit:"fixed normative measure".into(),
            basis:"Water, energy and construction material only; capacity slots and people are not priced".into(),
        },basis:"Fixed common opening provision for baseline and reviewed service repair".into(),
    };
    // Derive one common opening vector for both schedules before creating the
    // scenario. No top-up, norm change or issue is permitted during continuation.
    let a = evidence(&coupled::preview(d.clone(), before.clone(), vec![]).unwrap());
    let b = evidence(&coupled::preview(d.clone(), after.clone(), vec![]).unwrap());
    d.policy.opening = a
        .bound
        .minimum_opening_by_account
        .iter()
        .zip(&b.bound.minimum_opening_by_account)
        .map(|(a, b)| {
            a.parse::<u64>()
                .unwrap()
                .max(b.parse::<u64>().unwrap())
                .to_string()
        })
        .collect();
    let base = coupled::preview(d.clone(), before.clone(), vec![]).unwrap();
    let revised = coupled::preview_continuation(base.clone(), after.clone(), vec![]).unwrap();
    let a = evidence(&base);
    let b = evidence(&revised);
    assert!(a.all_scheduled_payments_settled && b.all_scheduled_payments_settled);
    assert!(a.resident_zero_each_period && b.resident_zero_each_period);
    let prefix: Vec<_> = a.events.iter().filter(|e| e.period < 4).cloned().collect();
    assert_eq!(
        prefix,
        b.events
            .iter()
            .filter(|e| e.period < 4)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(a.money.opening_total, b.money.opening_total);
    let blocked = b
        .events
        .iter()
        .position(|e| e.period >= 4 && e.kind == coupled::EventKind::Goods)
        .unwrap();
    let failed =
        evidence(&coupled::preview_continuation(base, after.clone(), vec![blocked]).unwrap());
    assert!(!failed.all_scheduled_payments_settled);
    json!({"policy":d.policy,"baseline_events":a.events,"repaired_events":b.events,
        "opening_total":a.money.opening_total,"baseline_money":a.money,"repaired_money":b.money,
        "unchanged_event_prefix":prefix.len(),"boundary":4,
        "blocked_event":blocked,"blocked_failures":failed.failed,
        "all_scheduled_payments_settled":true,"resident_zero_each_period":true})
}
fn feedback(definition: &ModelDefinition, input: &ModelInput) -> Value {
    // The baseline reserves current capacity and schedules northern staff training
    // in period four. Other development has already started at the revision boundary.
    let mut fresh = input.clone();
    fresh.boundary = 0;
    let batch = fresh.plan[0].activities[1];
    for period in &mut fresh.model.seasonal.physical.periods {
        period.capacities[1] = batch;
    }
    let baseline = defer_batch(&fresh, 1, 0, 4).unwrap();
    let before = preview(definition.clone(), baseline.clone()).unwrap();
    let c: ModelCheckpoint = serde_json::from_str(before.checkpoint.as_ref().unwrap()).unwrap();
    let mut observed = baseline.clone();
    observed.boundary = 4;
    let services = observed.services.as_mut().unwrap();
    let mut changes = Vec::new();
    // Temporary absences leave enough time for care but not the scheduled course.
    // Device availability is reduced at the same boundary without moving need.
    for k in [0, 4] {
        for t in [4, 5] {
            let e = &c.services.as_ref().unwrap().pools[t * services.pools.len() + k];
            let full: u64 = e.available.parse().unwrap();
            let use_: u64 = e.service_use.parse().unwrap();
            let bps = u16::try_from((use_ * 10000).div_ceil(full)).unwrap();
            let pool = &mut services.pools[k];
            changes.push(AvailabilityChange {
                pool: pool.id.clone(),
                period: t,
                previous_bps: pool.availability_bps[t],
                available_bps: bps,
            });
            pool.availability_bps[t] = bps;
        }
    }
    services.revisions.push(ServiceRevision {
        boundary: 4,
        basis: "Declared two-period mentor and device unavailability; retain care needs".into(),
        source_reference: "qazna:service-feedback:temporary-absence:v1".into(),
        changes,
    });
    let population = observed
        .population
        .as_ref()
        .unwrap()
        .audit(definition, &observed.model, &observed.plan)
        .unwrap();
    let shortage = observed
        .services
        .as_ref()
        .unwrap()
        .audit(definition, &observed.model, &observed.plan, &population)
        .unwrap();
    assert!(!shortage.all_selected_services_feasible);
    assert!(preview_continuation(before.clone(), observed.clone()).is_err());
    let repair = repair_service_batch(&before, &observed, 1, 4, 12).unwrap();
    let repaired = repair.input.as_ref().unwrap();
    assert_eq!(repair.attempts.last().unwrap().start, 6);
    let after = preview_continuation(before.clone(), repaired.clone()).unwrap();
    let checkpoint: ModelCheckpoint =
        serde_json::from_str(after.checkpoint.as_ref().unwrap()).unwrap();
    assert!(
        checkpoint
            .services
            .as_ref()
            .unwrap()
            .all_selected_services_feasible
    );
    let prefix = RollingCapital::checkpoint(&baseline.model, &baseline.plan, 4).unwrap();
    assert!(!prefix.snapshot().pending.is_empty());
    assert_eq!(json!(prefix.snapshot()), json!(checkpoint.rolling));
    assert_eq!(
        json!(population),
        json!(checkpoint.population.as_ref().unwrap())
    );
    let mut unavailable = observed.clone();
    let s = unavailable.services.as_mut().unwrap();
    s.pools[0].availability_bps[4] = 0;
    s.revisions[0].changes[0].available_bps = 0;
    let failed = repair_service_batch(&before, &unavailable, 1, 4, 12).unwrap();
    assert!(failed.input.is_none());
    let circulation = circulation(&before, &after);
    json!({"circulation":circulation,"baseline_input":baseline, "observed_input":observed,
        "repair":repair, "service_evidence":checkpoint.services,
        "preserved_boundary":checkpoint.rolling,
        "initial_shortages":shortage.pools.into_iter().filter(|r|r.shortage!="0").collect::<Vec<_>>(),
        "unrecoverable_attempts":failed.attempts,
        "policy":"Earliest feasible deferral of one unstarted batch; at most 24 dates; no norm, need or endowment change"})
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 2);
    let old: Value =
        serde_json::from_slice(&fs::read("research/need-profile-results.json").unwrap()).unwrap();
    let mut definition: ModelDefinition =
        serde_json::from_value(old["runtime_fixture"]["definition"].clone()).unwrap();
    let mut input: ModelInput =
        serde_json::from_value(old["runtime_fixture"]["input"].clone()).unwrap();
    let pop = input
        .population
        .as_ref()
        .unwrap()
        .audit(&definition, &input.model, &input.plan)
        .unwrap();
    let profiles = pop.profiles.as_ref().unwrap();
    let mut s = ServicePlan {
        revisions: vec![],
        registry: Registry {
            version: "service-capacity-v1".into(),
            mode: InputMode::Illustrative,
            parameters: vec![],
        },
        pools: vec![],
        rules: vec![],
    };
    for (id, unit, n, d) in [
        ("work_rate", "minute/slot/day", 480, 1),
        ("seat_rate", "place-day/slot/day", 1, 1),
        ("care_staff", "minute/minute", 1, 1),
        ("care_room", "minute/minute", 1, 1),
        ("care_device", "minute/minute", 1, 2),
        ("preschool_teacher", "minute/place-day", 20, 1),
        ("school_teacher", "minute/place-day", 15, 1),
        ("seat_use", "place-day/place-day", 1, 1),
        ("education_device", "minute/place-day", 1, 1),
        ("no_work", "minute/activity/month", 0, 1),
        ("no_seat_work", "place-day/activity/month", 0, 1),
        ("course_work", "minute/activity/month", 120, 1),
        ("build_work", "minute/activity/month", 60, 1),
        ("care_energy", "kWh/minute", 1, 1000),
        ("education_energy", "kWh/place-day", 1, 10),
    ] {
        parameter(&mut s.registry, id, unit, n, d);
    }
    for (name, unit) in [
        ("service_energy", "kWh"),
        ("construction_material", "unit"),
        ("eligible_training_places", "place"),
    ] {
        definition.resources.push(ModelResource {
            name: name.into(),
            unit: unit.into(),
        });
    }
    let kinds = [
        "care_staff",
        "educators",
        "care_rooms",
        "teaching_seats",
        "devices",
        "builders",
    ];
    for region in ["north", "south"] {
        for (k, kind) in kinds.iter().enumerate() {
            let resource = definition.resources.len();
            definition.resources.push(ModelResource {
                name: format!("{region}_{kind}"),
                unit: "slot".into(),
            });
            let seats = k == 3;
            s.pools.push(ServicePool {
                id: format!("{region}_{kind}"),
                region: region.into(),
                resource,
                unit: if seats { "place-day" } else { "minute" }.into(),
                rate_parameter: if seats { "seat_rate" } else { "work_rate" }.into(),
                working_days: pop
                    .projection
                    .months
                    .iter()
                    .map(|m| if seats { m.days } else { 20 })
                    .collect(),
                availability_bps: vec![10000; 60],
                process_load_parameters: vec![],
            });
        }
    }
    for region in ["north", "south"] {
        for item in ["care_time", "preschool_capacity", "school_capacity"] {
            let care = item == "care_time";
            let requirements = if care {
                vec![
                    ("care_staff", "care_staff"),
                    ("care_rooms", "care_room"),
                    ("devices", "care_device"),
                ]
            } else {
                vec![
                    (
                        "educators",
                        if item == "preschool_capacity" {
                            "preschool_teacher"
                        } else {
                            "school_teacher"
                        },
                    ),
                    ("teaching_seats", "seat_use"),
                    ("devices", "education_device"),
                ]
            };
            s.rules.push(ServiceRule {
                region: region.into(),
                item: item.into(),
                pools: requirements
                    .into_iter()
                    .map(|(kind, param)| PoolRequirement {
                        pool: format!("{region}_{kind}"),
                        parameter: param.into(),
                    })
                    .collect(),
                operating: vec![OperatingRequirement {
                    resource: 3,
                    parameter: if care {
                        "care_energy"
                    } else {
                        "education_energy"
                    }
                    .into(),
                }],
                delivered: profiles
                    .months
                    .iter()
                    .map(|m| {
                        m.regions
                            .iter()
                            .find(|r| r.region == region)
                            .unwrap()
                            .demands
                            .iter()
                            .find(|d| d.item == item)
                            .unwrap()
                            .central
                            .as_ref()
                            .unwrap()
                            .parse()
                            .unwrap()
                    })
                    .collect(),
            });
        }
    }
    let mut used = vec![vec![0u64; 60]; s.pools.len()];
    let mut energy = vec![0i64; 60];
    for t in 0..60 {
        let mut care = 0u64;
        let mut education = 0u64;
        for rule in &s.rules {
            if rule.item == "care_time" {
                care += rule.delivered[t]
            } else {
                education += rule.delivered[t]
            };
            for u in &rule.pools {
                let i = s.pools.iter().position(|p| p.id == u.pool).unwrap();
                let q = s
                    .registry
                    .parameters
                    .iter()
                    .find(|p| p.id == u.parameter)
                    .unwrap()
                    .estimate
                    .as_ref()
                    .unwrap()
                    .central;
                used[i][t] += (rule.delivered[t] * q.numerator).div_ceil(q.denominator);
            }
        }
        energy[t] = i64::try_from(care.div_ceil(1000) + education.div_ceil(10)).unwrap();
    }
    let mut initial = vec![0i64; s.pools.len()];
    let mut growth = vec![0i64; s.pools.len()];
    let mut lags = vec![];
    for (i, p) in s.pools.iter().enumerate() {
        let kind = i % 6;
        let lag = if kind < 2 {
            6
        } else if kind < 4 {
            4
        } else {
            2
        };
        lags.push(lag);
        if kind == 5 {
            continue;
        }
        let rate = if kind == 3 { 1 } else { 480 };
        let counts: Vec<u64> = used[i]
            .iter()
            .enumerate()
            .map(|(t, n)| n.div_ceil(u64::from(p.working_days[t]) * rate))
            .collect();
        let target = *counts.iter().max().unwrap();
        let first = *counts[..=lag].iter().max().unwrap();
        let buffer = if kind < 2 {
            (target * 120).div_ceil(20 * 480)
        } else {
            0
        };
        initial[i] = i64::try_from(first + buffer).unwrap();
        growth[i] = i64::try_from(target.saturating_sub(first + buffer)).unwrap();
    }
    for region in 0..2 {
        let start = region * 6;
        let work: i64 = growth[start + 2..start + 5].iter().sum::<i64>() * 60;
        initial[start + 5] = (work.div_euclid(9600) + i64::from(work % 9600 != 0)).max(1);
    }
    let r = definition.resources.len();
    let j = 1 + s.pools.len();
    let h = &mut input.model.seasonal.physical;
    h.resources = definition
        .resources
        .iter()
        .map(|r| r.name.clone())
        .collect();
    h.initial.resize(r, 0);
    h.storage_caps.resize(r, 0);
    for p in &mut h.processes {
        p.inputs.resize(r, 0);
        p.outputs.resize(r, 0);
    }
    for (i, pool) in s.pools.iter().enumerate() {
        let mut inputs = vec![0; r];
        inputs[3] = 2;
        inputs[4] = 5;
        if i % 6 < 2 {
            inputs[5] = 1;
        }
        let mut outputs = vec![0; r];
        outputs[pool.resource] = 1;
        h.processes.push(Process {
            inputs,
            outputs,
            lead_periods: lags[i],
        });
        h.initial[pool.resource] = initial[i];
        h.storage_caps[pool.resource] = initial[i] + growth[i];
        input.model.lives.push(AssetLife {
            resource: pool.resource,
            service_periods: 120,
            initial_remaining_periods: 120,
        });
    }
    h.initial[3] = energy.iter().sum::<i64>() + growth.iter().sum::<i64>() * 2;
    h.initial[4] = growth.iter().sum::<i64>() * 5;
    h.initial[5] = growth
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 6 < 2)
        .map(|(_, n)| *n)
        .sum();
    for k in 3..6 {
        h.storage_caps[k] = h.initial[k];
    }
    for (i, pool) in s.pools.iter_mut().enumerate() {
        pool.process_load_parameters = (0..j)
            .map(|process| {
                {
                    if i % 6 == 3 {
                        "no_seat_work"
                    } else if process == i + 1 && i % 6 < 2 {
                        "course_work"
                    } else if process > 0
                        && i % 6 == 5
                        && (process - 1) / 6 == i / 6
                        && (2..5).contains(&((process - 1) % 6))
                    {
                        "build_work"
                    } else {
                        "no_work"
                    }
                }
                .to_owned()
            })
            .collect();
    }
    let old_plan = input.plan.clone();
    let mut stock = h.initial.clone();
    let mut plan = vec![];
    for t in 0..60 {
        let mut row = PeriodPlan {
            activities: vec![0; j],
            deliveries: vec![0; r],
            closing: vec![0; r],
            discarded: vec![0; r],
        };
        row.deliveries[0] = old_plan[t].deliveries[0];
        row.deliveries[3] = energy[t];
        if t == 0 {
            for (i, n) in growth.iter().enumerate() {
                row.activities[i + 1] = *n;
            }
        }
        for (i, process) in h.processes.iter().enumerate() {
            for (value, coefficient) in stock.iter_mut().zip(&process.inputs) {
                *value -= row.activities[i] * coefficient;
            }
            if t >= process.lead_periods {
                let n = plan
                    .get(t - process.lead_periods)
                    .map(|row: &PeriodPlan| row.activities[i])
                    .unwrap_or(0);
                for (k, value) in stock.iter_mut().enumerate() {
                    *value += n * process.outputs[k];
                }
            }
        }
        for (k, value) in stock.iter_mut().enumerate() {
            *value -= row.deliveries[k];
            assert!(*value >= 0);
        }
        row.closing = stock.clone();
        h.periods[t].supplies.resize(r, 0);
        h.periods[t].needs.resize(r, 0);
        h.periods[t].needs[3] = energy[t];
        h.periods[t].capacities = vec![0; j];
        if t == 0 {
            h.periods[t].capacities[1..].copy_from_slice(&growth);
        }
        plan.push(row);
    }
    input.plan = plan;
    input.model.seasonal.closing_floors = vec![vec![0; r]; 60];
    input.services = Some(s);
    let population = input
        .population
        .as_ref()
        .unwrap()
        .audit(&definition, &input.model, &input.plan)
        .unwrap();
    let evidence = input
        .services
        .as_ref()
        .unwrap()
        .audit(&definition, &input.model, &input.plan, &population)
        .unwrap();
    if !evidence.all_selected_services_feasible {
        eprintln!(
            "shortages: {:?}",
            evidence
                .pools
                .iter()
                .filter(|p| p.shortage != "0")
                .collect::<Vec<_>>()
        );
    }
    assert!(evidence.all_selected_services_feasible);
    assert!(growth.iter().sum::<i64>() > 0);
    let accepted = preview(definition.clone(), input.clone()).unwrap();
    let checkpoint: ModelCheckpoint =
        serde_json::from_str(accepted.checkpoint.as_ref().unwrap()).unwrap();
    let mut interrupted = input.clone();
    interrupted.services.as_mut().unwrap().pools[0].availability_bps[20] = 0;
    let outage = interrupted
        .services
        .as_ref()
        .unwrap()
        .audit(
            &definition,
            &interrupted.model,
            &interrupted.plan,
            &population,
        )
        .unwrap();
    let rejection = preview(definition.clone(), interrupted)
        .unwrap_err()
        .to_string();
    let summary:Vec<_>=input.services.as_ref().unwrap().pools.iter().enumerate().map(|(i,p)|json!({"pool":p.id,"initial_slots":initial[i],"new_slots":growth[i],"completion_period":lags[i],"first_usable_period":lags[i]+1})).collect();
    let feedback = feedback(&definition, &input);
    let output = json!({"schema_version":3,"feedback":feedback,"assumptions":"docs/input-assumptions.md#care-and-education-capacity","runtime_fixture":{"definition":definition,"input":input,"population_evidence":checkpoint.population,"service_evidence":checkpoint.services},"capacity_summary":summary,"outage_counterexample":{"gaps":outage.pools.into_iter().filter(|p|p.shortage!="0").collect::<Vec<_>>(),"runtime_rejection":rejection}});
    fs::write(&args[1], serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    println!(
        "Verified regional care/education, shared development workload, operating resources and delayed capacity"
    );
}
