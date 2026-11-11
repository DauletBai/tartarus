//! Coupled physical capabilities and payments. Synthetic coefficients only.
#[path = "support/automation_study.rs"]
mod automation_study;
#[path = "support/integrated_lifecycle.rs"]
mod integrated_lifecycle;
#[path = "support/lifecycle_study.rs"]
mod lifecycle_study;
#[path = "support/nutrition_study.rs"]
mod nutrition_study;
#[path = "support/rolling_study.rs"]
mod rolling_study;
#[path = "support/seasonal_study.rs"]
mod seasonal_study;
#[path = "support/upstream_study.rs"]
mod upstream_study;
use qazna::{horizon::*, mathematics::*};
use serde_json::json;
use std::{env, fs};
const R: usize = 22;
const J: usize = 21;
const SCALE: i64 = 100_000; // 1,000 synthetic residents scaled to 100 million.
const DAYS: usize = 1825;
const VALUE: [i64; R] = [
    0, 0, 0, 100, 100, 100, 500, 10000, 0, 0, 200, 300, 600, 700, 10000, 200, 10000, 10000, 0,
    10000, 10000, 0,
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
#[derive(Clone, Copy, Default, serde::Serialize)]
struct ReservePolicy {
    food_buffers: bool,
    equipment_target: bool,
    early_training: bool,
    reserve_days: u32,
    loss_ppm: u32,
    storage_aware: bool,
    integrated_lifecycle: bool,
    age_aware_food: bool,
    durable_food_days: u32,
    preserved_food_days: u32,
    preserved_life_days: u32,
    food_recovery_allowance: bool,
    daily_service: bool,
}
fn study(
    name: &str,
    trim: bool,
    reroute: bool,
) -> (serde_json::Value, Vec<PeriodPlan>, Vec<ScheduledMovement>) {
    study_policy(name, trim, reroute, ReservePolicy::default())
}
fn study_policy(
    name: &str,
    trim: bool,
    reroute: bool,
    policy: ReservePolicy,
) -> (serde_json::Value, Vec<PeriodPlan>, Vec<ScheduledMovement>) {
    let mut resources = [
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
        "reserve_generator",
        "fuel_storage_units",
        "completed_generator_service",
        "dry_grain_storage_units",
        "packaged_food_storage_units",
        "generator_service_stage_one",
    ];
    let mut names = [
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
        "reserve_generator_construction",
        "fuel_storage_construction",
        "reserve_power",
        "generator_service",
        "dry_grain_storage_construction",
        "packaged_food_storage_construction",
        "generator_service_stage_two",
    ];
    if policy.preserved_life_days > 0 {
        resources[12] = "preserved_food_at_packaging";
        resources[13] = "preserved_food_at_collection";
        names[7] = "food_preservation_and_packaging";
        names[8] = "sealed_food_delivery";
        names[13] = "reserve_sealed_food_delivery";
    }
    let mut processes = vec![
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
        process(&[(6, 10), (3, 10)], 16, 1, 30),
        process(&[(6, 10), (3, 10)], 17, 1, 30),
        process(&[(4, 1), (2, 1)], 3, 10, 1),
        process(&[(6, 1), (3, 1)], 18, 1, 2),
        process(&[(6, 10), (3, 10)], 19, 1, 30),
        process(&[(6, 10), (3, 10)], 20, 1, 30),
        process(&[(21, 1)], 18, 1, 1),
    ];
    if policy.preserved_life_days > 0 {
        processes[7] = process(&[(11, 10), (3, 8), (6, 1)], 12, 10, 1);
    }
    if policy.daily_service {
        assert!(policy.integrated_lifecycle);
        processes[17] = process(&[(6, 1), (3, 1)], 21, 1, 1);
        names[17] = "generator_service_stage_one";
    }
    let initial = [
        1_000_000, 1_000_000, 0, 4000, 1000, 1000, 2000, 300, 200, 120, 6000, 4000, 4000, 4000,
        1000, 1000, 0, 0, 0, 0, 0, 0,
    ];
    let storage = [
        1_000_000, 1_000_000, 0, 0, 20000, 2000, 3000, 1000, 1000, 120, 15000, 6000, 6000, 6000,
        2000, 0, 1000, 1000, 0, 1000, 1000, 1000,
    ];
    let limits = vec![
        StockCapacity {
            name: "qualified_hours".into(),
            resource: 8,
            units_per_stock: 8,
            activity_use: vec![
                1, 1, 1, 2, 8, 2, 2, 2, 2, 8, 4, 1, 4, 4, 8, 8, 1, 4, 8, 8, 0,
            ],
        },
        StockCapacity {
            name: "equipment_use".into(),
            resource: 7,
            units_per_stock: 4,
            activity_use: vec![
                1, 1, 1, 1, 2, 1, 2, 1, 2, 2, 0, 0, 4, 4, 2, 2, 1, 1, 2, 2, 0,
            ],
        },
        StockCapacity {
            name: "housing_use".into(),
            resource: 14,
            units_per_stock: 1,
            activity_use: vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        },
    ];
    let mut limits = limits;
    if policy.daily_service {
        limits[0].activity_use[17] = 2;
        limits[0].activity_use[20] = 2;
    }
    let mut generator_use = vec![0; J];
    generator_use[16] = 1;
    limits.push(StockCapacity {
        name: "backup_generator_use".into(),
        resource: 16,
        units_per_stock: 10,
        activity_use: generator_use,
    });
    let mut stores = vec![BuiltStorage {
        name: "fuel_store".into(),
        stored_resource: 4,
        asset_resource: 17,
        base_capacity: 2000 * SCALE,
        capacity_per_asset: 1000,
    }];
    if policy.durable_food_days > 0 {
        stores.push(BuiltStorage {
            name: "dry_grain_store".into(),
            stored_resource: 11,
            asset_resource: 19,
            base_capacity: 6000 * SCALE,
            capacity_per_asset: 1000,
        });
    }
    if policy.preserved_life_days > 0 {
        stores.push(BuiltStorage {
            name: "sealed_food_store".into(),
            stored_resource: 13,
            asset_resource: 20,
            base_capacity: 6000 * SCALE,
            capacity_per_asset: 1000,
        });
    }
    let mut exits = vec![
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
    if policy.durable_food_days > 0 {
        model.storage_caps[11] = 60000 * SCALE;
    }
    if policy.preserved_life_days > 0 {
        model.storage_caps[13] = 60000 * SCALE;
    }
    if policy.reserve_days == 0 && policy.loss_ppm == 0 {
        model.storage_caps[4] = 2000 * SCALE;
    }
    let reserve_target = if policy.reserve_days > 0 {
        qazna::replanning::required_reserve(
            220 * SCALE,
            policy.reserve_days as usize,
            440 * SCALE,
            policy.loss_ppm,
        )
        .unwrap()
    } else {
        1000 * SCALE
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
            if matches!(j, 12 | 13 | 16) {
                assert_eq!(recipe_margin, margin[k]);
            }
            margin[k] = recipe_margin;
            assert!(margin[k] >= 0);
        }
    }
    let mut plan: Vec<PeriodPlan> = vec![];
    let mut previous = model.initial.clone();
    let mut pipeline = qazna::replanning::ProductionPipeline::new(R).unwrap();
    let mut lifecycle = policy.integrated_lifecycle.then(|| {
        if policy.preserved_life_days > 0 {
            integrated_lifecycle::Lifecycle::preserved(&model.initial, policy.preserved_life_days)
        } else {
            integrated_lifecycle::Lifecycle::new(&model.initial)
        }
    });
    if policy.daily_service {
        lifecycle = lifecycle.map(|l| l.with_daily_service());
    }
    for day in 0..DAYS {
        let demand = (1000 + 50 * (day / 365) as i64) * SCALE;
        let mut data = Period {
            supplies: vec![0; R],
            needs: vec![0; R],
            capacities: vec![500 * SCALE; J],
        };
        if policy.reserve_days == 0 {
            for j in 14..J {
                data.capacities[j] = 0;
            }
        }
        if !policy.integrated_lifecycle || name == "maintenance_disabled" {
            data.capacities[17] = 0;
        }
        if policy.durable_food_days == 0 || name == "grain_store_disabled" {
            data.capacities[18] = 0;
        }
        if policy.preserved_life_days == 0 || name == "packaged_store_disabled" {
            data.capacities[19] = 0;
        }
        if !policy.daily_service
            || (name == "technician_30" && (390..420).contains(&day))
            || (name == "technician_60" && (390..450).contains(&day))
            || (name == "technician_repeat"
                && ((390..420).contains(&day) || (440..470).contains(&day)))
        {
            data.capacities[20] = 0;
        }
        // Start no work that cannot finish within the study horizon.
        // Phase two may still remain pending under an unbounded final outage;
        // such scenarios must report unfinished service rather than reset it.
        if policy.daily_service && day + 2 >= DAYS {
            data.capacities[17] = 0;
        }
        let harvest_failure = match name {
            "harvest_30" | "grain_store_disabled" => (390..420).contains(&day),
            "harvest_60" => (390..450).contains(&day),
            "harvest_90" => (390..480).contains(&day),
            "early_harvest" => (10..40).contains(&day),
            _ => false,
        };
        if harvest_failure {
            data.capacities[5] = 0;
        }
        if name == "cooking_outage" && (390..405).contains(&day) {
            data.capacities[7] = 0;
        }
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
        let supply_failure = match name {
            "supply_30"
            | "maintenance_disabled"
            | "lifecycle_compound"
            | "technician_30"
            | "technician_repeat" => (390..420).contains(&day),
            "supply_60" | "technician_60" => (390..450).contains(&day),
            "supply_90" => (390..480).contains(&day),
            "supply_repeat" => (390..420).contains(&day) || (440..470).contains(&day),
            "early_supply" => (10..40).contains(&day),
            _ => false,
        };
        if supply_failure {
            data.capacities[0] = 0;
            data.capacities[1] = 0;
        }
        let loss = qazna::replanning::reserve_loss(previous[4], policy.loss_ppm).unwrap();
        if loss > 0 {
            exits.push(Withdrawal {
                period: day,
                resource: 4,
                quantity: loss,
                basis: "Observed deterministic fuel storage loss; rounded up to physical quantum"
                    .into(),
            });
        }
        let mut opening = previous.clone();
        for e in exits.iter().filter(|e| e.period == day) {
            assert!(opening[e.resource] >= e.quantity);
            opening[e.resource] -= e.quantity;
        }
        let arriving = pipeline.advance(day).unwrap();
        let mut lifecycle_loss = [0i64; 4];
        if let Some(l) = &mut lifecycle {
            lifecycle_loss = l.advance(
                day,
                &arriving,
                matches!(name, "transport_damage" | "lifecycle_compound")
                    && (390..420).contains(&day),
            );
            data.capacities[16] = data.capacities[16].min(l.generator_capacity(day));
            if (matches!(name, "correlated_backup" | "packaged_store_disabled")
                && (390..420).contains(&day))
                || (name == "correlated_60" && (390..450).contains(&day))
            {
                data.capacities[1] = 0;
                data.capacities[16] = 0;
            }
            data.needs[18] = arriving[18];
        }
        let mut available: Vec<_> = (0..R)
            .map(|k| opening[k] + data.supplies[k] + arriving[k])
            .collect();
        for k in 10..14 {
            available[k] -= lifecycle_loss[k - 10];
            if let Some(l) = &lifecycle {
                assert_eq!(available[k], l.food_total(k));
            }
        }
        let deliveries: Vec<_> = (0..R).map(|k| available[k].min(data.needs[k])).collect();
        for k in 0..R {
            available[k] -= deliveries[k];
            if (10..14).contains(&k)
                && let Some(l) = &mut lifecycle
            {
                l.consume(k, deliveries[k], day);
            }
        }
        let mut budget: Vec<_> = limits
            .iter()
            .map(|l| opening[l.resource] * l.units_per_stock)
            .collect();
        let mut target = [
            0, 0, 0, 2200, 1000, 1000, 2000, 400, 320, 0, 12000, 5000, 5000, 5000, 1300, 1200, 0,
            0, 0, 0, 0,
        ];
        if policy.food_buffers {
            target[10..14].copy_from_slice(&storage[10..14]);
        }
        if policy.age_aware_food {
            // Three current-demand days, including committed output. The five-day
            // meal life cannot support the previous six-day static food target.
            target[12] = 3 * (1000 + 50 * (day / 365) as i64);
            target[13] = target[12];
        }
        if policy.durable_food_days > 0 {
            let daily = 1000 + 50 * (day / 365) as i64;
            // Store the reserve upstream. Fresh output is replenished for flow,
            // not used as the strategic buffer. Pending output is included below.
            target[10] = 7 * daily;
            target[11] = (i64::from(policy.durable_food_days) + 1) * daily;
            target[12] = 2 * (1000 + 50 * ((day + 2) / 365) as i64);
            target[13] = (1..=2)
                .map(|offset| 1000 + 50 * ((day + offset) / 365) as i64)
                .sum();
            target[19] = ((target[11] - 6000).max(0) + 999) / 1000;
        }
        if policy.preserved_life_days > 0 {
            let daily = 1000 + 50 * (day / 365) as i64;
            let restart = if policy.food_recovery_allowance {
                model.processes[1].lead_periods
                    + model.processes[7].lead_periods
                    + model.processes[8]
                        .lead_periods
                        .max(model.processes[13].lead_periods)
            } else {
                0
            };
            target[13] = (i64::from(policy.preserved_food_days) + 1 + restart as i64) * daily;
            target[20] = ((target[13] - 6000).max(0) + 999) / 1000;
        }
        if policy.equipment_target {
            target[7] = 580;
        }
        if policy.reserve_days > 0 {
            target[4] = (reserve_target + SCALE - 1) / SCALE;
            target[16] = 22;
            target[17] = ((target[4] - 2000).max(0) + 999) / 1000;
        }
        let order: [usize; J] = if policy.storage_aware && available[4] >= 440 * SCALE {
            [
                20, 17, 1, 16, 11, 8, 13, 7, 6, 12, 5, 0, 2, 3, 4, 9, 10, 14, 15, 18, 19,
            ]
        } else if policy.early_training {
            [
                20, 17, 1, 16, 0, 11, 10, 8, 13, 7, 6, 12, 5, 2, 3, 4, 9, 14, 15, 18, 19,
            ]
        } else {
            [
                20, 17, 1, 16, 0, 11, 8, 13, 7, 6, 12, 5, 2, 3, 4, 9, 10, 14, 15, 18, 19,
            ]
        };
        let mut seen = [false; J];
        for j in order {
            assert!(!seen[j]);
            seen[j] = true;
        }
        assert!(seen.iter().all(|x| *x));
        let mut activities = vec![0; J];
        // Published annual growth is known ahead; outages are not anticipated.
        // Preserve core energy first. The experimental early-training policy
        // then prioritizes qualification; other capital uses residual capacity.
        for j in order {
            let p = &model.processes[j];
            if matches!(j, 12 | 13) && !reroute {
                continue;
            }
            if day + p.lead_periods >= DAYS {
                continue;
            }
            let k = p.outputs.iter().position(|&q| q > 0).unwrap();
            let desired = if j == 20 {
                lifecycle.as_ref().map_or(0, |l| l.second_stage_need(day))
            } else if j == 17 {
                lifecycle.as_ref().map_or(0, |l| l.service_need(day))
            } else if policy.durable_food_days > 0 && matches!(k, 12 | 13) {
                let position = lifecycle.as_ref().unwrap().position_at(k, day + 2);
                ((target[k] * SCALE - position).max(0) + p.outputs[k] - 1) / p.outputs[k]
            } else if k == 3 {
                if j == 16 {
                    (220 * SCALE - activities[1]).max(0)
                } else {
                    220 * SCALE
                }
            } else if k == 4 && policy.storage_aware {
                let next_loss =
                    qazna::replanning::reserve_loss(available[4], policy.loss_ppm).unwrap();
                ((target[4] * SCALE - available[4] + next_loss - pipeline.pending()[4]).max(0) + 9)
                    / 10
            } else if k == 15 {
                ((1000 + 50 * ((day + 1) / 365) as i64) * SCALE + 9) / 10
            } else {
                ((target[k] * SCALE - available[k] - pipeline.pending()[k]).max(0) + p.outputs[k]
                    - 1)
                    / p.outputs[k]
            };
            let mut n = desired.min(data.capacities[j]);
            if matches!(j, 4 | 9 | 10 | 14 | 15 | 18 | 19) {
                n = n.min(SCALE);
            }
            for (k, &q) in p.inputs.iter().enumerate() {
                if q > 0 {
                    n = n.min(available[k] / q);
                }
            }
            if let Some(l) = &lifecycle {
                let valid_at = if matches!(j, 6 | 8 | 12 | 13) {
                    day + p.lead_periods
                } else {
                    day
                };
                for k in 10..14 {
                    if p.inputs[k] > 0 {
                        n = n.min(l.eligible(k, valid_at) / p.inputs[k]);
                    }
                }
            }
            if j == 16 {
                n = n.min((available[4] - 440 * SCALE).max(0));
            }
            if j == 0 && policy.storage_aware {
                let space = model.storage_caps[4].min(2000 * SCALE + 1000 * available[17]);
                let next_loss =
                    qazna::replanning::reserve_loss(available[4], policy.loss_ppm).unwrap();
                let projected = available[4] - next_loss + pipeline.pending()[4];
                n = n.min((space - projected).max(0) / 10);
            }
            if policy.durable_food_days > 0 && matches!(j, 6 | 12) {
                let space = model.storage_caps[11].min(6000 * SCALE + 1000 * available[19]);
                n = n.min((space - available[11] - pipeline.pending()[11]).max(0) / 10);
            }
            if policy.preserved_life_days > 0 && matches!(j, 8 | 13) {
                let space = model.storage_caps[13].min(6000 * SCALE + 1000 * available[20]);
                n = n.min((space - available[13] - pipeline.pending()[13]).max(0) / 10);
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
                if let Some(l) = &mut lifecycle {
                    if matches!(j, 5 | 6 | 7 | 8 | 12 | 13) {
                        l.start_food(day, j, p.lead_periods, p.outputs[k] * n);
                    }
                    if j == 20 {
                        l.finish_service(day, n);
                    }
                    if j == 17 {
                        l.start_service(day, n);
                    }
                }
                assert!(pipeline.commit(day, j, p, n).unwrap());
            }
        }
        let mut effective_storage = model.storage_caps.clone();
        effective_storage[4] = effective_storage[4].min(2000 * SCALE + 1000 * available[17]);
        if policy.durable_food_days > 0 {
            effective_storage[11] = effective_storage[11].min(6000 * SCALE + 1000 * available[19]);
        }
        if policy.preserved_life_days > 0 {
            effective_storage[13] = effective_storage[13].min(6000 * SCALE + 1000 * available[20]);
        }
        let mut waste: Vec<_> = (0..R)
            .map(|k| (available[k] - effective_storage[k]).max(0))
            .collect();
        let closing: Vec<_> = (0..R).map(|k| available[k] - waste[k]).collect();
        for k in 10..14 {
            if let Some(l) = &mut lifecycle {
                l.consume(k, waste[k], day);
                assert_eq!(l.food_total(k), closing[k]);
            }
            waste[k] += lifecycle_loss[k - 10];
        }
        previous = closing.clone();
        model.periods.push(data);
        plan.push(PeriodPlan {
            activities,
            deliveries,
            closing,
            discarded: waste,
        });
    }
    if let Some(l) = &lifecycle {
        assert!(l.is_drained());
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
        for k in [7, 14, 16, 17, 19, 20] {
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
        let payment_order: [usize; J] = [
            20, 17, 1, 16, 0, 11, 8, 13, 7, 6, 12, 5, 2, 3, 4, 9, 10, 14, 15, 18, 19,
        ];
        for j in payment_order {
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
                        if j == 10 || j == 17 {
                            0
                        } else if j == 12 {
                            8
                        } else if j == 13 {
                            10
                        } else if j == 16 {
                            3
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
    let matrix = model.compile_reserves(&limits, &exits, &stores).unwrap();
    let vector = model.vector(&plan).unwrap();
    model
        .verify_cash_links(&plan, &VALUE, &[7, 14, 16, 17, 19, 20], &links)
        .unwrap();
    let mut forged = links.clone();
    forged[0].amount = (forged[0].amount.parse::<i128>().unwrap() + 1).to_string();
    assert!(
        model
            .verify_cash_links(&plan, &VALUE, &[7, 14, 16, 17, 19, 20], &forged)
            .is_err()
    );
    let verified = verify_linear(&matrix, &vector, None).unwrap();
    let housing_certificate = (name == "compound_delayed" && policy.reserve_days==0).then(|| {
        let mut bound_model = matrix.clone();
        bound_model.objective.fill(0);
        let index: std::collections::BTreeMap<_,_> = bound_model.rows.iter().enumerate()
            .map(|(i,r)|(r.name.clone(),i)).collect();
        let mut dual = RationalVector {numerators:vec![0; bound_model.rows.len()],denominator:1};
        for t in 392..422 {
            bound_model.objective[t*(3*R+J)+15] = 1;
            dual.numerators[index[&format!("balance:{t}:15")]] = 1;
            dual.numerators[index[&format!("balance:{}:3",t-1)]] = 10;
            dual.numerators[index[&format!("capacity:{}:1",t-2)]] = 100;
            dual.numerators[index[&format!("capacity:{}:16",t-2)]] = 100;
        }
        dual.numerators[index["storage:391:15"]] = 1;
        dual.numerators[index["storage:390:3"]] = 10;
        let proof = verify_linear(&bound_model,&vector,Some(&dual)).unwrap();
        assert_eq!(proof.objective_numerator,"0");
        let sparse: Vec<_> = bound_model.rows.iter().zip(&dual.numerators)
            .filter(|(_,w)|**w!=0).map(|(r,w)|json!({"row":r.name,"weight":w})).collect();
        // A deleted premise and an added energy buffer must both prevent this
        // exact zero-delivery certificate, even with unchanged feasible trace.
        let mut forged = dual.clone();
        forged.numerators[index["capacity:390:1"]] = 0;
        assert!(verify_linear(&bound_model,&vector,Some(&forged)).is_err());
        bound_model.rows[index["storage:390:3"]].rhs = 1;
        assert!(verify_linear(&bound_model,&vector,Some(&dual)).is_err());
        json!({"window":[392,421],"maximum_housing_delivery":"0","verification":proof,
            "dual_nonzero_rows":sparse,"deleted_premise_rejected":true,"changed_storage_premise_rejected":true})
    });
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
        assert!(p.closing[4] <= 2000 * SCALE + 1000 * p.closing[17]);
        if policy.durable_food_days > 0 {
            assert!(p.closing[11] <= 6000 * SCALE + 1000 * p.closing[19]);
        }
        if policy.preserved_life_days > 0 {
            assert!(p.closing[13] <= 6000 * SCALE + 1000 * p.closing[20]);
        }
        let recorded_loss: i64 = exits
            .iter()
            .filter(|e| e.period == day && e.resource == 4)
            .map(|e| e.quantity)
            .sum();
        let product = i128::from(prior[4]) * i128::from(policy.loss_ppm);
        assert_eq!(
            i128::from(recorded_loss),
            product / 1_000_000 + i128::from(product % 1_000_000 != 0)
        );
        prior = p.closing.clone();
    }
    eprintln!("Verified scenario {name}, trim={trim}");
    (
        json!({"scenario":name,"integrated_lifecycle":lifecycle.as_ref().map(|l|json!({"expired":l.expired,"transport_damage":l.damaged,"maintenance_starts":l.maintenance_starts})),"reserve_policy":policy,"reserve_target_minimum":reserve_target,"housing_outage_certificate":housing_certificate,"rerouting":reroute,"trim_evidence":trim_evidence,"input_status":"synthetic_uncalibrated","population_scale":100_000_000,"periods":DAYS,"resources":resources,"process_names":names,"processes":model.processes,"initial":model.initial,"storage_caps":model.storage_caps,"built_storage":stores,"limits":limits,"withdrawals":exits,
        "fixed_unit_measure":VALUE,"common_entitlement_per_sold_unit":margin,"owner_account":owner,
        "matrix":{"variables":matrix.objective.len(),"rows":matrix.rows.len(),"nonzero_terms":matrix.rows.iter().map(|r|r.terms.len()).sum::<usize>()},"minimum_outage_unmet_proven":infeasibility_bound,"outage_bound_scope":if infeasibility_bound.is_some(){Some("fixed_route_policy_no_reserve_meal_dispatch")}else{None},"cash_link_count":links.len(),"forged_payment_rejected":true,"physical_verification":verified,"food_objective_optimal":optimal,"first_unmet_day":first,"discarded":discarded.iter().map(ToString::to_string).collect::<Vec<_>>(),"unmet":missing.iter().map(ToString::to_string).collect::<Vec<_>>(),"activity_totals":starts.iter().map(ToString::to_string).collect::<Vec<_>>(),"first_activity":first_departure,"years":years,
        "requested_provision_measure":measures[0].to_string(),"delivered_provision_measure":measures[1].to_string(),"completed_capital_procurement_measure":measures[2].to_string(),"liquidity":bound,"money":money}),
        plan,
        events,
    )
}
fn main() {
    if env::args().nth(1).as_deref() == Some("--rolling-only") {
        let s = seasonal_study::study(&nutrition_study::study());
        let a = automation_study::study(&s);
        let u = upstream_study::study(&a);
        fs::write(
            env::args().nth(2).expect("output path"),
            serde_json::to_vec_pretty(&rolling_study::study(&u)).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--upstream-only") {
        let seasonal = seasonal_study::study(&nutrition_study::study());
        let automation = automation_study::study(&seasonal);
        fs::write(
            env::args().nth(2).expect("output path"),
            serde_json::to_vec_pretty(&upstream_study::study(&automation)).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--automation-only") {
        let path = env::args().nth(2).expect("output path");
        let seasonal = seasonal_study::study(&nutrition_study::study());
        fs::write(
            path,
            serde_json::to_vec_pretty(&automation_study::study(&seasonal)).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--seasonal-only") {
        let path = env::args().nth(2).expect("output path");
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&seasonal_study::study(&nutrition_study::study())).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--nutrition-only") {
        fs::write(
            env::args().nth(2).expect("output JSON path"),
            serde_json::to_vec_pretty(&nutrition_study::study()).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--daily-service-only") {
        fs::write(
            env::args().nth(2).expect("output JSON path"),
            serde_json::to_vec_pretty(&daily_service_study()).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--preserved-only") {
        fs::write(
            env::args().nth(2).expect("output JSON path"),
            serde_json::to_vec_pretty(&preserved_food_study()).unwrap(),
        )
        .unwrap();
        return;
    }
    if env::args().nth(1).as_deref() == Some("--durable-only") {
        let report = durable_food_study();
        fs::write(
            env::args().nth(2).expect("output JSON path"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        return;
    }
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
    let joint_planning = joint_reserve_search();
    let reserve_lifecycle = reserve_lifecycle_study();
    let nutrition = nutrition_study::study();
    let seasonal = seasonal_study::study(&nutrition);
    let automation = automation_study::study(&seasonal);
    let upstream = upstream_study::study(&automation);
    let rolling = rolling_study::study(&upstream);
    let report = json!({"version":15,"rolling_and_materials":rolling,"upstream_capital":upstream,"automation_and_food_chain":automation,"seasonal_food":seasonal,"nutrition_and_orders":nutrition,"daily_service":daily_service_study(),"durable_food_reserve":durable_food_study(),"preserved_food_reserve":preserved_food_study(),"integrated_lifecycle_graph":integrated_lifecycle_study(),"food_asset_lifecycle":lifecycle_study::study(),"reserve_lifecycle":reserve_lifecycle,"joint_planning":joint_planning,"kind":"synthetic_coupled_capability_study","model_calls":0,
        "assumptions":["Synthetic aggregate coefficients, not demographic/engineering norms","Water is an external boundary supply; deposits and initial assets/stocks are explicit","Training places represent consenting eligible capacity, not people owned or traded","Qualified capacity and equipment are shared use limits, not consumed ingredients","Public asset procurement is paid only for completed equipment/buildings","Input payments follow actual consumption; incomplete work ties up opening working measure","Natural resources and people have zero commodity price in this example","All monetary norms fixed; no interest, tax, loan or later issuance","Actual missing provision remains recorded; no claim that missing goods cancel human entitlements","Money is prefunded using exact schedule bounds, not optimized globally","Trimmed cases are retrospective fixed-delivery improvements, not causal forecasts or global optima","Recovery decisions use current observed capacities, stocks and committed output; no future outage duration is provided","Reserve routes take two periods and four shared equipment/skill units per batch; synthetic capacity is declared in every comparison","The original terminal stock is a lower bound in trimmed cases; all intermediate safety-stock policy remains open","Energy/fuel/material quantities are abstract recipe units, not measured kWh/tonnes; external water left unused is reported separately from energy discard","Legacy policies leave dated-food/service extensions disabled; integrated_lifecycle_graph, durable_food_reserve and preserved_food_reserve enable them; health demography, individual choice, teacher specializations and foreign trade remain open","Heuristic is not a global optimum except when a valid stated objective certificate exists"],"cases":cases,"improved_cases":improved,"comparisons":comparisons,"recovery":recovery,"common_funding":common_funding});
    let path = env::args()
        .nth(1)
        .unwrap_or("research/coupled-results.json".into());
    fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    println!("Verified coupled physical and monetary traces: {path}");
}

/// Finite policy search, not a continuous/integer global optimizer.
/// All eight policies are chosen before observing any outage, then use the same
/// feedback rule across the six declared paths. Full traces remain independently
/// checked by the physical matrix and the second stock/capacity replay above.
fn joint_reserve_search() -> serde_json::Value {
    let paths = [
        "baseline",
        "transport_outage",
        "long_transport_outage",
        "power_outage",
        "compound_outage",
        "compound_delayed",
    ];
    // End-of-horizon productive and food reserves, common to all candidates.
    // Natural deposits need nonnegativity, not a claim of indefinite renewal.
    let mut floors = [0i64; R];
    floors[7] = 300 * SCALE;
    floors[8] = 200 * SCALE;
    floors[14] = 1000 * SCALE;
    floors[10] = 6000 * SCALE;
    floors[11] = 4000 * SCALE;
    floors[12] = 4000 * SCALE;
    floors[13] = 3800 * SCALE;
    let mut candidates = vec![];
    let mut scores = vec![];
    let mut costs = vec![];
    let mut canonical_basis = None;
    for mask in 0..8 {
        let policy = ReservePolicy {
            food_buffers: mask & 1 != 0,
            equipment_target: mask & 2 != 0,
            early_training: mask & 4 != 0,
            ..ReservePolicy::default()
        };
        let mut results = vec![];
        let mut traces = vec![];
        let mut schedules = vec![];
        let mut feasible = true;
        let mut worst = [0i128; 2];
        let mut worst_capital = 0i128;
        for path in paths {
            let (result, trace, schedule) = study_policy(path, false, true, policy);
            let basis = json!([
                result["initial"],
                result["storage_caps"],
                result["processes"],
                result["limits"],
                result["withdrawals"],
                result["fixed_unit_measure"],
                result["owner_account"]
            ]);
            if let Some(ref expected) = canonical_basis {
                assert_eq!(expected, &basis);
            } else {
                canonical_basis = Some(basis);
            }
            let terminal = &trace.last().unwrap().closing;
            let terminal_ok = terminal.iter().zip(floors).all(|(&x, f)| x >= f);
            feasible &= terminal_ok;
            for (i, k) in [13, 15].into_iter().enumerate() {
                worst[i] = worst[i].max(result["unmet"][k].as_str().unwrap().parse().unwrap());
            }
            worst_capital = worst_capital.max(
                result["completed_capital_procurement_measure"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap(),
            );
            results.push(json!({"scenario":path,"unmet":result["unmet"],
                "first_unmet_day":result["first_unmet_day"],"housing_outage_certificate":result["housing_outage_certificate"],"terminal":terminal,
                "terminal_floor_satisfied":terminal_ok,"activity_totals":result["activity_totals"],
                "completed_capital_procurement_measure":result["completed_capital_procurement_measure"],
                "discarded":result["discarded"],"physical_verification":result["physical_verification"],
                "food_objective_optimal":result["food_objective_optimal"],
                "first_activity":result["first_activity"]}));
            traces.push(trace);
            schedules.push(schedule);
        }
        for (a, b, end) in [
            (0, 1, 365),
            (1, 2, 395),
            (0, 3, 600),
            (2, 4, 390),
            (4, 5, 405),
        ] {
            assert_eq!(
                serde_json::to_value(&traces[a][..end]).unwrap(),
                serde_json::to_value(&traces[b][..end]).unwrap()
            );
        }
        let named: Vec<_> = paths
            .iter()
            .zip(&schedules)
            .map(|(&p, s)| (p, s.as_slice()))
            .collect();
        let envelope = common_schedule_opening(J + 2, &named).unwrap();
        let funds: Vec<u64> = envelope
            .minimum_opening_by_account
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        for (result, schedule) in results.iter_mut().zip(&schedules) {
            let run = replay_schedule(&funds, schedule, &[]).unwrap();
            assert!(run.first_blocked_event.is_none());
            assert_eq!(run.closing[1], "0");
            result["money"] = serde_json::to_value(run).unwrap();
        }
        // Needs in both categories are the same daily scalar; max absolute
        // shortfall is thus the same ordering as max unmet fraction. Never add
        // food units to housing units or infer a social priority from money.
        let score = worst[0].max(worst[1]);
        scores.push(if feasible { Some(score) } else { None });
        costs.push([
            worst[0],
            worst[1],
            worst_capital,
            envelope.total_minimum_opening.parse::<i128>().unwrap(),
        ]);
        candidates.push(json!({"id":mask,"policy":policy,"terminal_feasible":feasible,
            "worst_meal_shortfall":worst[0].to_string(),"worst_housing_shortfall":worst[1].to_string(),
            "worst_unmet_numerator":score.to_string(),"worst_capital_measure":worst_capital.to_string(),
            "common_opening":envelope,"scenarios":results}));
    }
    let best = scores
        .iter()
        .flatten()
        .copied()
        .min()
        .expect("at least one admissible policy");
    let winners: Vec<_> = scores
        .iter()
        .enumerate()
        .filter_map(|(i, s)| (*s == Some(best)).then_some(i))
        .collect();
    let pareto: Vec<_> = (0..8)
        .filter(|&i| {
            scores[i].is_some()
                && !(0..8).any(|j| {
                    scores[j].is_some()
                        && costs[j].iter().zip(costs[i]).all(|(&a, b)| a <= b)
                        && costs[j].iter().zip(costs[i]).any(|(&a, b)| a < b)
                })
        })
        .collect();
    let total_need: i128 = (0..DAYS)
        .map(|d| i128::from((1000 + 50 * (d / 365) as i64) * SCALE))
        .sum();
    // Universal lower bound within this fixed physical model: during the
    // 30-day power outage, initial energy cannot bridge a second day (storage
    // cap zero), housing services cannot be stored, and have a one-day lead.
    // Outage starts 390. Last prior power completes390, housing completes391.
    // Days392..421 inclusive cannot receive housing output, regardless of policy.
    let unavoidable_housing: i128 = (392..422)
        .map(|d| i128::from((1000 + 50 * (d / 365) as i64) * SCALE))
        .sum();
    assert!(best >= unavoidable_housing);
    for c in &candidates {
        let n: i128 = c["scenarios"][5]["unmet"][15]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(n >= unavoidable_housing);
    }
    json!({"scope":"exhaustive_eight_precommitted_feedback_policies_six_fixed_disruption_paths",
        "input_status":"synthetic_uncalibrated","scenario_count":6,"candidate_count":8,
        "same_initial_resources_and_capacities":true,"same_observation_prefix_checks":40,
        "terminal_floors":floors,"objective":"minimize worst unmet fraction across the two services and six paths; no cash tie-break",
        "common_need_denominator":total_need.to_string(),"best_unmet_numerator":best.to_string(),
        "best_candidate_ids":winners,"pareto_candidate_ids":pareto,
        "pareto_axes":["worst_meal_shortfall","worst_housing_shortfall","worst_capital_measure","common_opening_measure"],"finite_policy_optimality":"all eight candidates evaluated exactly",
        "physical_lower_bound_numerator":unavoidable_housing.to_string(),
        "gap_to_physical_lower_bound":(best-unavoidable_housing).to_string(),
        "minimax_optimal_for_supplied_scenario_family":best==unavoidable_housing,
        "bound_scope":"housing during compound_30; zero energy and housing storage; one-day power/housing leads",
        "initial_resource_basis":canonical_basis,"candidates":candidates})
}

fn reserve_lifecycle_study() -> serde_json::Value {
    let paths = [
        "baseline",
        "compound_delayed",
        "supply_30",
        "supply_60",
        "supply_90",
        "supply_repeat",
        "early_supply",
    ];
    let mut designs = vec![];
    let mut basis = None;
    for (days, storage_aware) in [(0, true), (7, true), (30, true), (60, true), (60, false)] {
        let policy = ReservePolicy {
            food_buffers: true,
            equipment_target: true,
            early_training: false,
            reserve_days: days,
            loss_ppm: 1000,
            storage_aware,
            integrated_lifecycle: false,
            age_aware_food: false,
            durable_food_days: 0,
            preserved_food_days: 0,
            preserved_life_days: 0,
            food_recovery_allowance: false,
            daily_service: false,
        };
        let mut results = vec![];
        let mut traces = vec![];
        let mut schedules = vec![];
        for name in paths {
            let (r, trace, events) = study_policy(name, false, true, policy);
            let actual_basis = json!([
                r["initial"],
                r["storage_caps"],
                r["processes"],
                r["limits"],
                r["built_storage"],
                r["fixed_unit_measure"]
            ]);
            if let Some(ref expected) = basis {
                assert_eq!(expected, &actual_basis);
            } else {
                basis = Some(actual_basis);
            }
            let required = r["reserve_target_minimum"].as_i64().unwrap();
            let end = match name {
                "compound_delayed" | "supply_30" => 420,
                "supply_60" => 450,
                "supply_90" => 480,
                "supply_repeat" => 470,
                "early_supply" => 40,
                _ => 0,
            };
            let mut restored = None;
            let mut loss_sum = 0i128;
            let mut observations = vec![];
            let mut unmet_windows: Vec<serde_json::Value> = vec![];
            let mut start = None;
            for (day, p) in trace.iter().enumerate() {
                let prior = if day == 0 {
                    r["initial"][4].as_i64().unwrap()
                } else {
                    trace[day - 1].closing[4]
                };
                let loss = qazna::replanning::reserve_loss(prior, policy.loss_ppm).unwrap();
                loss_sum += i128::from(loss);
                let incoming = if day > 0 {
                    10 * trace[day - 1].activities[0]
                } else {
                    0
                };
                let ready = prior - loss + incoming;
                if day >= end && restored.is_none() && ready >= required {
                    restored = Some(day);
                }
                let need = (1000 + 50 * (day / 365) as i64) * SCALE;
                let missing = p.deliveries[13] < need || p.deliveries[15] < need;
                if missing && start.is_none() {
                    start = Some(day);
                }
                if !missing && let Some(first) = start.take() {
                    unmet_windows.push(json!([first, day - 1]));
                }
                if [
                    0,
                    10,
                    30,
                    60,
                    389,
                    390,
                    419,
                    420,
                    439,
                    440,
                    449,
                    450,
                    469,
                    470,
                    479,
                    480,
                    510,
                    600,
                    DAYS - 1,
                ]
                .contains(&day)
                {
                    observations.push(json!({"period":day,"fuel_opening":prior,"storage_loss":loss,
                        "fuel_ready":ready,"fuel_closing":p.closing[4],"storage_assets":p.closing[17],
                        "generators":p.closing[16],"backup_starts":p.activities[16],
                        "need_each_service":need,"meals_delivered":p.deliveries[13],"housing_delivered":p.deliveries[15]}));
                }
                if p.activities[16] > 0 {
                    assert!(ready - p.activities[1] - p.activities[16] >= 440 * SCALE);
                    assert!(day > 0 && trace[day - 1].closing[16] * 10 >= p.activities[16]);
                }
            }
            if let Some(first) = start {
                unmet_windows.push(json!([first, DAYS - 1]));
            }
            let last = &trace.last().unwrap().closing;
            let mut terminal_shortfalls = vec![];
            for (k, floor) in [
                (7, 300),
                (8, 200),
                (10, 6000),
                (11, 4000),
                (12, 4000),
                (13, 3800),
                (14, 1000),
            ] {
                if last[k] < floor * SCALE {
                    terminal_shortfalls
                        .push(json!({"resource":k,"required":floor*SCALE,"actual":last[k]}));
                }
            }
            if days > 0 && last[16] < 22 * SCALE {
                terminal_shortfalls
                    .push(json!({"resource":16,"required":22*SCALE,"actual":last[16]}));
            }
            results.push(json!({"scenario":name,"unmet":r["unmet"],"unmet_windows":unmet_windows,
                "first_unmet_day":r["first_unmet_day"],"first_fuel_target_after_recovery":restored,
                "fuel_storage_loss":loss_sum.to_string(),"discarded":r["discarded"],"activity_totals":r["activity_totals"],
                "completed_capital_procurement_measure":r["completed_capital_procurement_measure"],
                "requested_provision_measure":r["requested_provision_measure"],"delivered_provision_measure":r["delivered_provision_measure"],
                "physical_verification":r["physical_verification"],"cash_links":r["cash_link_count"],
                "terminal":last,"terminal_shortfalls":terminal_shortfalls,"terminal_feasible":terminal_shortfalls.is_empty(),"observations":observations,"first_activity":r["first_activity"]}));
            traces.push(trace);
            schedules.push(events);
        }
        for (a, b, end) in [
            (0, 2, 390),
            (2, 3, 420),
            (3, 4, 450),
            (2, 5, 440),
            (0, 6, 10),
        ] {
            assert_eq!(
                serde_json::to_value(&traces[a][..end]).unwrap(),
                serde_json::to_value(&traces[b][..end]).unwrap()
            );
        }
        let named: Vec<_> = paths
            .iter()
            .zip(&schedules)
            .map(|(&p, s)| (p, s.as_slice()))
            .collect();
        let common = common_schedule_opening(J + 2, &named).unwrap();
        let funds: Vec<u64> = common
            .minimum_opening_by_account
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        for (result, events) in results.iter_mut().zip(&schedules) {
            let run = replay_schedule(&funds, events, &[]).unwrap();
            assert!(run.first_blocked_event.is_none());
            assert_eq!(run.closing[1], "0");
            result["money"] = serde_json::to_value(run).unwrap();
        }
        let component_required =
            qazna::replanning::required_reserve(220 * SCALE, days as usize, 440 * SCALE, 1000)
                .unwrap();
        designs.push(json!({"policy":policy,"nominal_autonomy_periods":days,
            "minimum_component_opening_fuel":component_required,"restart_floor":440*SCALE,
            "common_opening":common,"all_terminal_floors_met":results.iter().all(|r|r["terminal_feasible"]==true),"scenarios":results}));
    }
    // Executable acceptance gates and a retained defective-policy counterexample.
    let covered = |d: usize, c: usize| {
        designs[d]["scenarios"][c]["unmet"][13] == "0"
            && designs[d]["scenarios"][c]["unmet"][15] == "0"
    };
    for (d, design) in designs.iter().enumerate().take(4) {
        assert!(covered(d, 0));
        assert_eq!(design["all_terminal_floors_met"], true);
        for c in design["scenarios"].as_array().unwrap() {
            assert_eq!(c["discarded"][4], "0");
        }
        assert!(!covered(d, 6)); // A reserve still under construction cannot erase early failures.
    }
    for c in [0, 1, 2, 5] {
        assert!(covered(2, c));
    }
    assert!(covered(3, 3));
    assert!(!covered(1, 2));
    assert!(!covered(2, 3));
    assert!(!covered(3, 4));
    assert_eq!(designs[4]["all_terminal_floors_met"], false);
    assert!(!covered(4, 0));
    assert_eq!(designs[4]["scenarios"][0]["terminal"][1], 0);
    let adequate: Vec<_> = (0..4)
        .filter(|&d| [0, 1, 2, 5].iter().all(|&c| covered(d, c)))
        .collect();
    json!({"scope":"constructed_fuel_reserves_backup_generation_and_deterministic_loss",
        "input_status":"synthetic_uncalibrated","candidate_count":5,"scenario_count":7,
        "same_initial_physical_basis":true,"same_observation_prefix_checks":25,
        "physical_basis":basis,"designs":designs,"adequate_for_prepared_30_day_family":adequate,
        "prepared_family_scenarios":["baseline","compound_delayed","supply_30","supply_repeat"],
        "smallest_tested_adequate_nominal_days":30,"counterexample_design_index":4,
        "scope_note":"prepared family excludes preconstruction failure and supply interruptions beyond 30 periods; all remain reported",
        "assumptions":["Autonomy sizing uses 220 fuel units per cohort-period plus two restart periods",
            "Daily fuel loss is 1000 ppm rounded up; not a measured fuel property",
            "Generator and storage construction each require 10 metal plus 10 energy and 30 periods",
            "One generator permits 10 backup batches per period; one storage unit holds 1000 additional fuel units",
            "Backup conversion has the same declared fuel/water recipe as primary generation; constructed redundancy is paid as capital",
            "Restart floor is protected from backup dispatch, not exempt from physical storage loss",
            "The experiment is not food-expiry modelling or a guarantee against unlimited starvation",
            "No new money is issued during any replay; every physical shortage remains visible"]})
}

/// Lifecycle constraints now affect starts, input consumption and all cash links
/// of the complete graph. A trace verifier is still not a lifecycle LP optimizer.
fn integrated_lifecycle_study() -> serde_json::Value {
    let policy = ReservePolicy {
        food_buffers: true,
        equipment_target: true,
        reserve_days: 30,
        loss_ppm: 1000,
        storage_aware: true,
        integrated_lifecycle: true,
        age_aware_food: true,
        ..ReservePolicy::default()
    };
    let (fixed_targets, _, _) = study_policy(
        "baseline",
        false,
        true,
        ReservePolicy {
            age_aware_food: false,
            ..policy
        },
    );
    let names = [
        "baseline",
        "supply_30",
        "supply_repeat",
        "transport_damage",
        "lifecycle_compound",
        "correlated_backup",
        "maintenance_disabled",
    ];
    let mut results = vec![];
    let mut schedules = vec![];
    let mut baseline = vec![];
    let mut prefixes = 0;
    for name in names {
        let (r, trace, events) = study_policy(name, false, true, policy);
        if name == "baseline" {
            baseline = trace.clone();
        } else if name != "maintenance_disabled" {
            assert_eq!(
                serde_json::to_value(&trace[..390]).unwrap(),
                serde_json::to_value(&baseline[..390]).unwrap()
            );
            prefixes += 1;
        }
        assert!(r["physical_verification"]["feasible"].as_bool().unwrap());
        assert!(r["money"]["first_blocked_event"].is_null());
        results.push(r);
        schedules.push(events);
    }
    let views: Vec<_> = names
        .iter()
        .zip(&schedules)
        .map(|(n, s)| (*n, s.as_slice()))
        .collect();
    let envelope = common_schedule_opening(J + 2, &views).unwrap();
    let opening: Vec<u64> = envelope
        .minimum_opening_by_account
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let mut monetary = vec![];
    for (name, events) in &views {
        let replay = replay_schedule(&opening, events, &[]).unwrap();
        assert!(replay.first_blocked_event.is_none());
        assert_eq!(replay.closing[1], "0");
        monetary.push(json!({"scenario":name,"replay":replay}));
    }
    assert!(
        results[0]["integrated_lifecycle"]["maintenance_starts"]
            .as_i64()
            .unwrap()
            > 0
    );
    assert_eq!(results[6]["integrated_lifecycle"]["maintenance_starts"], 0);
    assert!(
        results[3]["integrated_lifecycle"]["transport_damage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|q| q.as_u64().unwrap() > 0)
    );
    json!({"scope":"legacy_lifecycle_graph_with_two_inactive_storage_extensions",
        "periods":DAYS,"fixed_target_counterexample":fixed_targets,"results":results,"common_opening":envelope,"common_money_replay":monetary,
        "same_observation_prefix_checks":prefixes,
        "assumptions":["Grain expires 90 days after harvest; cooked meals expire five days after cooking completion; initial stock starts at age zero",
        "Transport preserves source expiry; cooking consumes valid grain and creates a newly dated cooked product; this is a synthetic process rule, not food-safety certification",
        "Expired stock and observed transit damage are explicit discarded quantities in the same physical matrix; damaged/lost goods do not become resident receipts",
        "Damage is ceil(10% per arriving transport cohort) during the declared shock; it is not silently substituted into recipe yields",
        "Backup cohorts are created only from completed generator production, usable next period; calendar maintenance due after 20 available-age days",
        "Service consumes one metal and one energy per generator unit, four shared qualified-hours and one equipment-use unit at start, with two-period turnaround",
        "Service labour is front-loaded at start in this graph; multi-day labour reservations in the isolated maintenance test are not yet carried into the graph",
        "Completed service is consumed internally at zero commodity measure, not counted as human entitlement; service inputs are funded by common treasury and checked as process inputs",
        "Age eligibility and service history are additional deterministic trace guards; the aggregate LP matrix alone does not prove their validity for a different plan",
        "Meals at kitchen and destination target three current-demand days including committed output; fixed six-thousand-unit targets are retained as a baseline counterexample; global target optimization remains open",
        "All quantities and norms synthetic; cash envelopes conditional on declared schedules, no later issuance; live runtime unchanged"]})
}

/// Store the strategic reserve upstream; fresh production supports short flow.
fn durable_food_study() -> serde_json::Value {
    let policy = ReservePolicy {
        equipment_target: true,
        reserve_days: 30,
        loss_ppm: 1000,
        storage_aware: true,
        integrated_lifecycle: true,
        durable_food_days: 30,
        ..ReservePolicy::default()
    };
    let names = [
        "baseline",
        "harvest_30",
        "harvest_60",
        "transport_damage",
        "lifecycle_compound",
        "cooking_outage",
        "grain_store_disabled",
        "early_harvest",
        "correlated_backup",
    ];
    let mut report = food_policy_comparison(policy, &names);
    report["assumptions"] = json!([
        "All coefficients synthetic; grain still expires 90 days after harvest and meals five days after cooking; no lifetime is extended to improve this comparison",
        "Existing stocks and assets identical to prior integrated runs; extra city-grain storage must be built from metal, energy and shared work/equipment, with 30-day lead",
        "Each completed dry-store asset adds 1000 grain units to initial 6000 capacity, capped at 60000; target is 30 demand-days plus one operating day",
        "Farm grain target seven demand-days; kitchen target two known future days, destination two; pending output and physical eligibility are checked",
        "Only published demand growth is anticipated; disruptions observed at onset, no knowledge of future duration",
        "Grain is an abstract staple equivalent, not a complete diet; protein, oils, micronutrients, packaging and food safety need separately calibrated recipes",
        "Storage construction and processing costs counted; storage operation/drying/quality loss not yet calibrated",
        "Raw grain cannot bypass unavailable cooking, energy or delivery; ready-to-eat durable emergency food is a separate extension",
        "Financial envelopes conditional on fixed schedules; live runtime authority unchanged"
    ]);
    report["scope"] = json!("historical_fresh_flow_with_durable_grain_reserve");
    report
}

fn food_policy_comparison(policy: ReservePolicy, names: &[&str]) -> serde_json::Value {
    let mut results = vec![];
    let mut schedules = vec![];
    let mut baseline = vec![];
    let mut prefixes = 0;
    for &name in names {
        let (r, trace, events) = study_policy(name, false, true, policy);
        if name == "baseline" {
            baseline = trace.clone();
        } else if !matches!(name, "grain_store_disabled" | "packaged_store_disabled") {
            let until = if name == "early_harvest" { 10 } else { 390 };
            assert_eq!(
                serde_json::to_value(&trace[..until]).unwrap(),
                serde_json::to_value(&baseline[..until]).unwrap()
            );
            prefixes += 1;
        }
        assert!(r["physical_verification"]["feasible"].as_bool().unwrap());
        assert!(r["money"]["first_blocked_event"].is_null());
        let checkpoints: Vec<_> = [0, 29, 30, 60, 389, 419, 449, 500, DAYS-1].into_iter().map(|day| json!({
            "day": day, "grain_farm":trace[day].closing[10], "grain_city":trace[day].closing[11],
            "product_at_processing":trace[day].closing[12], "product_at_collection":trace[day].closing[13],
            "grain_store_assets":trace[day].closing[19], "packaged_store_assets":trace[day].closing[20], "food_delivery":trace[day].deliveries[13],
            "farming_starts":trace[day].activities[5], "cooking_starts":trace[day].activities[7]
        })).collect();
        results.push(json!({"calculation":r,"checkpoints":checkpoints}));
        schedules.push(events);
    }
    let views: Vec<_> = names
        .iter()
        .zip(&schedules)
        .map(|(n, s)| (*n, s.as_slice()))
        .collect();
    let envelope = common_schedule_opening(J + 2, &views).unwrap();
    let opening: Vec<u64> = envelope
        .minimum_opening_by_account
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let mut replays = vec![];
    for (name, events) in &views {
        let replay = replay_schedule(&opening, events, &[]).unwrap();
        assert!(replay.first_blocked_event.is_none());
        assert_eq!(replay.closing[1], "0");
        replays.push(json!({"scenario":name,"replay":replay}));
    }
    json!({"periods":DAYS,"results":results,"common_opening":envelope,"common_money_replay":replays,"same_observation_prefix_checks":prefixes})
}
fn preserved_food_study() -> serde_json::Value {
    let policy = ReservePolicy {
        equipment_target: true,
        reserve_days: 30,
        loss_ppm: 1000,
        storage_aware: true,
        integrated_lifecycle: true,
        durable_food_days: 30,
        preserved_food_days: 30,
        preserved_life_days: 730,
        food_recovery_allowance: true,
        ..ReservePolicy::default()
    };
    let mut report = food_policy_comparison(
        policy,
        &[
            "baseline",
            "harvest_30",
            "harvest_60",
            "harvest_90",
            "transport_damage",
            "lifecycle_compound",
            "cooking_outage",
            "early_harvest",
            "correlated_backup",
            "correlated_60",
            "packaged_store_disabled",
        ],
    );
    let (nominal_only, _, _) = study_policy(
        "correlated_backup",
        false,
        true,
        ReservePolicy {
            food_recovery_allowance: false,
            ..policy
        },
    );
    for i in [0, 1, 2, 4, 5, 6, 8] {
        assert_eq!(report["results"][i]["calculation"]["unmet"][13], "0");
    }
    for i in [3, 7, 9, 10] {
        assert!(
            report["results"][i]["calculation"]["unmet"][13]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                > 0
        );
    }
    assert_eq!(
        report["results"][0]["calculation"]["integrated_lifecycle"]["expired"],
        json!([0, 0, 0, 0])
    );
    assert!(
        report["results"][0]["checkpoints"][4]["packaged_store_assets"]
            .as_i64()
            .unwrap()
            > 0
    );
    assert_eq!(
        report["results"][10]["checkpoints"][4]["packaged_store_assets"],
        0
    );
    assert!(
        nominal_only["unmet"][13]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
    report["nominal_reserve_counterexample"] = nominal_only;
    report["scope"] = json!("preserved_food_only_no_fresh_product_storage");
    report["storage_evidence"] = json!([
        {"source":"https://scholarsarchive.byu.edu/facpub/67/", "type":"2004 conference poster abstract", "product":"white rice, uncooked, reduced-oxygen sealed cans", "observation":"18 donated samples aged 6-30 years; both rice types above 88 percent emergency acceptance at 30 years", "boundary":"Sensory acceptance of tested samples, not universal safety or complete nutrient preservation for all foods"},
        {"source":"https://extension.usu.edu/preserve-the-harvest/research/storing-white-rice", "type":"university extension guidance", "observation":"25-30 years for properly sealed and stored polished white rice", "boundary":"Conditional on product, temperature, packaging and storage; does not calibrate generic preserved meals"},
        {"source":"https://ask.fsis.usda.gov/article/How-long-can-you-keep-canned-goods", "type":"USDA guidance", "observation":"Best-quality guidance distinguishes high-acid food, up to 18 months, and low-acid meat/vegetables, 2-5 years", "boundary":"Quality guidance is not an automatic safety cutoff or a decades-long guarantee"}
    ]);
    report["preservation_specification_gate"] = json!({
        "required_before_calibrated_use":["product and recipe identifier", "drying/thermal/other validated treatment", "water activity and moisture", "package and seal integrity", "oxygen/CO2 specification where applicable", "temperature/humidity envelope", "quality and nutrient retention curve", "safety acceptance and quarantine criteria", "energy/material/labour coefficients", "opening and rehydration requirements", "source, version and units"],
        "candidate_methods":["drying", "freeze drying", "validated thermal preservation", "vacuum packaging combined with appropriate treatment", "controlled atmosphere combined with appropriate treatment"],
        "simulation_only":true, "automatic_food_safety_authorization":false
    });
    report["assumptions"] = json!([
        "Target architecture has no fresh perishable product inventory: process 7 preserves and seals food; transport retains its expiry",
        "Preservation recipe per ten abstract staple units: ten grain, eight energy and one metal packaging unit; qualified/equipment capacity inherited, coefficients synthetic",
        "Packaged product life 730 days is a declared test parameter, not certification of vacuum/CO2/drying/canning interchangeability; grain remains 90 days",
        "Initial product quantities unchanged but represent pre-preserved sealed stock; its embodied processing/packaging is part of initial endowment, not free conversion of old fresh inventory",
        "Thirty demand-days plus one operating day and four restart/route-lead days at collection; extra packaged storage must be built in 30 days from metal, energy, shared work and equipment; no unbuilt space used",
        "FEFO rotates preserved goods through daily consumption; a long permitted life is not a requirement to leave every lot untouched for years",
        "Stock at collection can satisfy demand during disruption without new cooking or transport; this presumes local accessible collection points, not a proof of last-mile service",
        "Packed resource is an edible staple-equivalent, not a complete diet or dry food guaranteed edible without water; actual diets and rehydration needs require typed recipes",
        "No national calibration: storage climate, operating costs, package failure, food safety, recycling and nutrient retention remain explicit future inputs",
        "All historical fresh-food experiments retained only as comparators; no AI or live authority change"
    ]);
    report
}

/// Two separately capacity-constrained workdays; no automatic calendar repair.
fn daily_service_study() -> serde_json::Value {
    let policy = ReservePolicy {
        equipment_target: true,
        reserve_days: 30,
        loss_ppm: 1000,
        storage_aware: true,
        integrated_lifecycle: true,
        durable_food_days: 30,
        preserved_food_days: 30,
        preserved_life_days: 730,
        food_recovery_allowance: true,
        daily_service: true,
        ..ReservePolicy::default()
    };
    let names = [
        "baseline",
        "supply_30",
        "technician_30",
        "technician_60",
        "technician_repeat",
    ];
    let mut report = food_policy_comparison(policy, &names);
    let mut witnesses = vec![];
    for name in ["technician_30", "technician_60", "technician_repeat"] {
        let (_, trace, _) = study_policy(name, false, true, policy);
        let end = if name == "technician_60" { 450 } else { 420 };
        assert!(trace[390..end].iter().all(|p| p.activities[20] == 0));
        // Independent queue conservation, not the cohort implementation:
        // stage-two input must come from yesterday's starts or an older queue.
        let mut queue = 0;
        let mut completed = 0i128;
        for (day, p) in trace.iter().enumerate() {
            if day > 0 {
                queue += trace[day - 1].activities[17];
            }
            assert!(p.activities[20] <= queue);
            queue -= p.activities[20];
            assert_eq!(queue, p.closing[21]);
            assert_eq!(p.discarded[21], 0);
            let arrivals = if day > 0 {
                trace[day - 1].activities[20]
            } else {
                0
            };
            assert_eq!(p.deliveries[18], arrivals);
            completed += i128::from(arrivals);
        }
        assert_eq!(queue, 0);
        assert!(trace[390..end].iter().any(|p| p.closing[21] > 0));
        let repair_completions_during_absence: i64 =
            trace[391..end].iter().map(|p| p.deliveries[18]).sum();
        assert_eq!(repair_completions_during_absence, 0);
        // Matched counterfactual: old model auto-completes after two calendar
        // days even when the second day's technical team is unavailable.
        let (old, old_trace, _) = study_policy(
            name,
            false,
            true,
            ReservePolicy {
                daily_service: false,
                ..policy
            },
        );
        let old_completions: i64 = old_trace[391..end].iter().map(|p| p.deliveries[18]).sum();
        assert!(old_completions > 0);
        witnesses.push(
            json!({"scenario":name, "outage_start":390, "outage_end_exclusive":end,
            "pending_at_outage_end":trace[end-1].closing[21],
            "daily_model_completions_during_absence":repair_completions_during_absence,
            "front_loaded_completions_during_absence":old_completions,
            "completed_service_over_horizon":completed.to_string(),
            "old_model_unmet":old["unmet"],
            "old_model_conditional_opening":old["liquidity"],
            "first_restored_stage_two_day":(end..DAYS).find(|&t| trace[t].activities[20] > 0)}),
        );
    }
    report["scope"] = json!("full_graph_with_two_day_shared_labour_service_and_pauses");
    report["counterfactual_witnesses"] = json!(witnesses);
    report["assumptions"] = json!([
        "Synthetic two-day repair: phase one consumes one metal, one energy, two shared qualified-work units and one equipment-use unit; phase two consumes two shared qualified-work units on a later day",
        "Total four qualified-work units and material inputs unchanged; second phase needs no extra equipment or energy in this declared recipe, not an engineering calibration",
        "Phase-one completion is internal work-in-progress, never resident provision or new capital; phase two requires its actual available quantity and the team's observed daily capacity",
        "Pending service occupies the existing generator cohort; no new generator is created, no duplicate service starts, and delayed repair does not reset food deadlines",
        "Service completion arrives the day after phase two, usable one day later under the existing conservative operating convention; old front-loaded comparisons retained",
        "Technical-team absence blocks phase two for the stated window; other qualified staff remain available for phase one and unrelated work, so no forecast knowledge is used",
        "All five schedules replay with a single componentwise-minimal opening vector for that finite family; no later issuance or forced private saving",
        "Full-horizon service queues drain for these cases; indefinite outages and general production work spread over time remain outside this bounded experiment",
        "No real-food certification, demographics, live runtime authority or national feasibility claim"
    ]);
    report
}
