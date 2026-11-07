//! Bounded lifecycle submodel, not a recalibration of the 18-resource economy.
use qazna::lifecycle::{LotBook, StandbyUnit};
use serde_json::{Value, json};

const DAYS: u32 = 1825;
const DEMAND: u64 = 1000;

pub fn study() -> Value {
    let designs = [
        ("fresh_staggered", 5, 10, 200),
        ("preserved_staggered", 30, 10, 200),
        ("fresh_aligned", 5, 0, 200),
        ("fresh_no_spares", 5, 10, 0),
    ];
    let paths = [
        "normal",
        "route_15",
        "late_arrival",
        "power_30",
        "transport_loss",
        "combined",
        "correlated_failure",
        "repeated_power",
    ];
    let mut runs = vec![];
    let mut prefix_checks = 0;
    for (name, shelf_life, second_age, initial_spares) in designs {
        let mut normal_trace = vec![];
        for path in paths {
            let mut lots = LotBook::default();
            let mut fleet = [
                StandbyUnit::new(0, 20, 2, 1200, 1, 2).unwrap(),
                StandbyUnit::new(second_age, 20, 2, 1200, 1, 2).unwrap(),
            ];
            let mut spares = initial_spares;
            let mut fuel = 5000u64;
            let mut fuel_used = 0;
            let mut labour_used = 0;
            let mut next_shipment = 0;
            let mut unmet = 0u64;
            let mut post_startup_unmet = 0u64;
            let mut expired_in_transit = 0u64;
            let mut trace = vec![];
            for day in 0..DAYS {
                let before = lots.balance();
                lots.begin_period(day).unwrap();
                let after_expiry = lots.balance();
                expired_in_transit += before.in_transit - after_expiry.in_transit;
                let disturbance = (390..420).contains(&day);
                let repeated = path == "repeated_power" && (440..470).contains(&day);
                let power_down = matches!(
                    path,
                    "power_30" | "combined" | "correlated_failure" | "repeated_power"
                ) && (disturbance || repeated);
                let correlated = path == "correlated_failure" && (390..400).contains(&day);
                let mut labour = 2u64; // Shared, finite maintenance capacity each day.
                let mut backup_capacity = 0;
                let mut maintenance = vec![];
                for unit in &mut fleet {
                    let result = unit
                        .step(day, unit.service_due(), spares, labour, correlated)
                        .unwrap();
                    spares -= result.spares_used;
                    labour -= result.labour_used;
                    labour_used += result.labour_used;
                    backup_capacity += result.available_capacity;
                    maintenance.push(result);
                }
                let hold_arrival = path == "late_arrival" && (390..400).contains(&day);
                let mut arrival = 0;
                let mut damaged = 0;
                if !hold_arrival {
                    for (id, quantity) in lots.due_shipments() {
                        let loss = if matches!(path, "transport_loss" | "combined") && disturbance {
                            quantity.div_ceil(10)
                        } else {
                            0
                        };
                        if loss > 0 {
                            lots.damage(id, loss).unwrap();
                            damaged += loss;
                        }
                        arrival += lots.receive(id).unwrap();
                    }
                }
                let consumed = lots.consume(DEMAND).unwrap();
                unmet += DEMAND - consumed;
                if day >= 10 {
                    post_startup_unmet += DEMAND - consumed;
                }
                // Boundary food supply is explicit and finite: no claim that it
                // has been produced by the coupled study's farming recipes.
                lots.supply(u64::from(day), 1200, day + shelf_life).unwrap();
                let b = lots.balance();
                let desired = 4000u64.saturating_sub(b.destination + b.in_transit);
                let route_down = path == "route_15" && (390..405).contains(&day);
                let capacity = if route_down {
                    0
                } else if power_down {
                    backup_capacity.min(fuel * 10).min(2000)
                } else {
                    2000
                };
                let mut budget = capacity.min(desired);
                let mut dispatched = 0;
                for (id, available, expiry) in lots.source_lots() {
                    if day + 2 >= expiry {
                        continue;
                    }
                    let quantity = available.min(budget);
                    if quantity == 0 {
                        continue;
                    }
                    lots.dispatch(next_shipment, id, quantity, day + 2).unwrap();
                    next_shipment += 1;
                    dispatched += quantity;
                    budget -= quantity;
                }
                let spent_fuel = if power_down {
                    dispatched.div_ceil(10)
                } else {
                    0
                };
                fuel -= spent_fuel;
                fuel_used += spent_fuel;
                let balance = lots.balance();
                assert_eq!(
                    balance.supplied,
                    balance.consumed
                        + balance.expired
                        + balance.transport_loss
                        + balance.source
                        + balance.destination
                        + balance.in_transit
                );
                assert_eq!(fuel + fuel_used, 5000);
                assert!(labour <= 2 && spares <= initial_spares);
                trace.push(
                    json!({"day":day,"consumed":consumed,"dispatched":dispatched,
                    "received":arrival,"damaged":damaged,"balance":balance,"fuel":fuel,
                    "spares":spares,"maintenance":maintenance}),
                );
            }
            if path == "normal" {
                normal_trace = trace.clone();
            } else {
                assert_eq!(&trace[..390], &normal_trace[..390]);
                prefix_checks += 1;
            }
            let balance = lots.balance();
            runs.push(
                json!({"design":name,"scenario":path,"shelf_life":shelf_life,
                "initial_asset_ages":[0,second_age],"initial_spares":initial_spares,
                "balance":balance,"unmet":unmet,"unmet_after_day_9":post_startup_unmet,
                "expired_in_transit":expired_in_transit,"spares_remaining":spares,
                "maintenance_labour":labour_used,"fuel_used":fuel_used,
                "receipt_measure_at_fixed_700":(u128::from(balance.consumed)*700).to_string(),
                "disturbance_trace":&trace[385..475],"all_days_conserved":true}),
            );
        }
    }
    let get = |name: &str, path: &str| {
        runs.iter()
            .find(|r| r["design"] == name && r["scenario"] == path)
            .unwrap()
    };
    assert_eq!(get("fresh_staggered", "normal")["unmet_after_day_9"], 0);
    assert!(
        get("fresh_staggered", "late_arrival")["expired_in_transit"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        get("fresh_staggered", "route_15")["unmet_after_day_9"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        get("fresh_no_spares", "power_30")["unmet_after_day_9"]
            .as_u64()
            .unwrap()
            > get("fresh_staggered", "power_30")["unmet_after_day_9"]
                .as_u64()
                .unwrap()
    );
    assert!(
        get("fresh_staggered", "correlated_failure")["unmet_after_day_9"]
            .as_u64()
            .unwrap()
            > 0
    );
    json!({"scope":"separate_food_transport_and_standby_lifecycle_submodel", "periods":DAYS,
        "runs":runs,"equal_observation_prefix_checks":prefix_checks,"verified_periods":DAYS*32,
        "assumptions":["All quantities synthetic; daily demand 1000, boundary food supply 1200, pipeline target 4000",
        "Expiry exclusive; original lot age survives dispatch and receipt; arrivals precede consumption; new dispatch follows consumption",
        "Two-period transit, maximum dispatch 2000; primary power supplies that capacity except during declared outages",
        "Two pre-existing standby units supply at most 1200 each, 20-day calendar service interval, two labour-days per service",
        "Shared maintenance labour 2 per day; each unit uses 2 per service day and one spare on start; no replenishment of spares",
        "Backup dispatch consumes ceil(quantity/10) fuel from opening 5000; no fuel refill; standby construction belongs to the earlier coupled experiment",
        "Failure flags become observable only on the current day; no restoration date is supplied to the policy",
        "Fresh versus preserved food and aligned versus staggered ages are different declared initial assumptions, not cost-free policy improvements",
        "Longer shelf life has no preservation cost in this isolated sensitivity study; it is not evidence that preservation is free",
        "Receipt valuation is an arithmetic link only, not a new ledger replay or common-opening proof",
        "No national scale/calibration/optimality claim; no runtime integration; quality, recalls and identity remain open"]})
}
