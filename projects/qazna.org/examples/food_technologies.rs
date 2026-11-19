//! Screen mature production routes against the same dated orders and finite site inputs.
use serde::Deserialize;
use serde_json::{Value, json};
use std::{env, fs};

#[derive(Deserialize)]
struct Site {
    first_year: u64,
    months: usize,
    monthly_orders_kg: Orders,
    lettuce: Lettuce,
    fish: Fish,
    coast: Coast,
    well: Well,
    district: District,
}
#[derive(Deserialize)]
struct Orders {
    lettuce: u64,
    fish_fillet: u64,
    shellfish_meat: u64,
}
#[derive(Deserialize)]
struct Lettuce {
    hydroponic_l_per_kg: u64,
    field_l_per_kg: u64,
    greenhouse_yield_tenths_kg_per_m2_year: u64,
    field_yield_tenths_kg_per_m2_year: u64,
    electric_wh_per_kg: u64,
    heat_kwh_per_kg_by_month: [u64; 12],
}
#[derive(Deserialize)]
struct Fish {
    edible_percent: u64,
    feed_g_per_live_kg: u64,
    ras_makeup_l_per_live_kg: u64,
    river_gross_l_per_live_kg: u64,
    ras_electric_wh_per_live_kg: u64,
    river_electric_wh_per_live_kg: u64,
    critical_ras_load_kw: u64,
    usable_purge_m3_per_month: u64,
    polishing_yield_percent: u64,
    polishing_wh_per_m3: u64,
}
#[derive(Deserialize)]
struct Coast {
    edible_percent: u64,
    monthly_mature_shell_on_kg: u64,
    cold_store_edible_kg: u64,
    opening_frozen_edible_kg: u64,
    partner_distance_km: u64,
    cold_chain_wh_per_edible_kg: u64,
    store_wh_per_opening_kg_month: u64,
}
#[derive(Deserialize)]
struct Well {
    opening_m3: u64,
    capacity_m3: u64,
    protected_floor_m3: u64,
    net_inflow_m3_by_month: [u64; 12],
    monthly_permitted_m3: u64,
    monthly_pump_capacity_m3: u64,
    treated_yield_percent: u64,
    total_dynamic_head_m: u64,
    pump_efficiency_percent: u64,
    treatment_wh_per_raw_m3: u64,
}
#[derive(Deserialize)]
struct District {
    monthly_power_kwh: u64,
    usable_backup_kwh: u64,
    backup_rating_kw: u64,
    monthly_heat_kwh: u64,
    shared_basin_gross_allocation_m3: u64,
    monthly_contracted_feed_kg: u64,
    river_intake_m3_after_environmental_reserve_by_month: [u64; 12],
    water_haul_one_way_km: u64,
    monthly_haul_water_m3: u64,
    monthly_cold_freight_kg_km: u64,
}
#[derive(Clone, Copy, PartialEq)]
enum Route {
    WellRas,
    HauledRas,
    IntegratedRas,
    River,
}
#[derive(Clone, Copy, Default)]
struct Shock {
    dry: bool,
    deep: bool,
    closures: bool,
    prepared_depot: bool,
    outage_hours: u64,
    power_loss: bool,
}

fn run(s: &Site, route: Route, shock: Shock) -> Value {
    let (w, d, c, f, o) = (
        &s.well,
        &s.district,
        &s.coast,
        &s.fish,
        &s.monthly_orders_kg,
    );
    let live = (o.fish_fillet * 100).div_ceil(f.edible_percent);
    let feed = (live * f.feed_g_per_live_kg).div_ceil(1000);
    let greenhouse = (o.lettuce * s.lettuce.hydroponic_l_per_kg).div_ceil(1000);
    let ras = route != Route::River;
    let makeup = if ras {
        (live * f.ras_makeup_l_per_live_kg).div_ceil(1000)
    } else {
        0
    };
    // Only a separately collected and acceptable purge stream can displace
    // greenhouse make-up. No internal circulation or nutrient credit is counted.
    let polished_input = if route == Route::IntegratedRas {
        f.usable_purge_m3_per_month
            .min(makeup)
            .min((greenhouse * 100).div_ceil(f.polishing_yield_percent))
    } else {
        0
    };
    let reused = greenhouse.min(polished_input * f.polishing_yield_percent / 100);
    let clean = greenhouse + makeup - reused;
    let river_need = if ras {
        0
    } else {
        (live * f.river_gross_l_per_live_kg).div_ceil(1000)
    };
    let (mut stock, mut frozen) = (w.opening_m3, c.opening_frozen_edible_kg);
    let (mut extraction, mut pumping, mut treatment, mut hauled, mut unmet_shellfish) =
        (0, 0, 0, 0, 0);
    let mut min_stock = u64::MAX;
    // A seasonal depot option expands the physical store and orders from open lots in advance.
    let closure_orders = 3 * o.shellfish_meat;
    let cold_capacity = if shock.prepared_depot {
        c.cold_store_edible_kg.max(closure_orders)
    } else {
        c.cold_store_edible_kg
    };
    let mut total_received = 0;
    let mut total_electricity = 0;
    let mut total_heat = 0;
    let mut failures = vec![];
    for t in 0..s.months {
        let m = t % 12;
        let before = stock;
        let inflow = w.net_inflow_m3_by_month[m] / if shock.dry { 2 } else { 1 };
        let spill = (stock + inflow).saturating_sub(w.capacity_m3);
        stock = stock + inflow - spill;
        let mut gaps = serde_json::Map::new();
        let mut check = |label: &str, need: u64, available: u64| {
            if need > available {
                gaps.insert(label.to_owned(), json!(need - available));
            }
        };
        let mut well_withdrawal = 0;
        let mut water_electric = (polished_input * f.polishing_wh_per_m3).div_ceil(1000);
        if route == Route::HauledRas {
            let supplied = clean.min(d.monthly_haul_water_m3);
            hauled += supplied * d.water_haul_one_way_km;
            check("delivered_clean_water_m3", clean, supplied);
        } else {
            let wanted = (clean * 100).div_ceil(w.treated_yield_percent);
            let raw = wanted
                .min(w.monthly_permitted_m3)
                .min(d.shared_basin_gross_allocation_m3)
                .min(w.monthly_pump_capacity_m3)
                .min(stock.saturating_sub(w.protected_floor_m3));
            well_withdrawal = raw;
            stock -= raw;
            extraction += raw;
            assert_eq!(before + inflow, stock + raw + spill);
            let head = w.total_dynamic_head_m * if shock.deep { 4 } else { 1 };
            let pump = (raw * 981 * head).div_ceil(3600 * w.pump_efficiency_percent);
            let treat = (raw * w.treatment_wh_per_raw_m3).div_ceil(1000);
            pumping += pump;
            treatment += treat;
            water_electric += pump + treat;
            check(
                "treated_makeup_m3",
                clean,
                raw * w.treated_yield_percent / 100,
            );
        }
        min_stock = min_stock.min(stock);
        check(
            "river_gross_intake_m3",
            river_need,
            d.river_intake_m3_after_environmental_reserve_by_month[m]
                .min(d.shared_basin_gross_allocation_m3 - well_withdrawal),
        );
        check("compound_feed_kg", feed, d.monthly_contracted_feed_kg);

        // Frozen product is held inland. Only a released coastal lot can be shipped.
        // Consume oldest first; order enough to restore the specified working reserve.
        let closed = shock.closures && (6..=8).contains(&m);
        let coast_cap = if closed {
            0
        } else {
            c.monthly_mature_shell_on_kg * c.edible_percent / 100
        };
        let opening_frozen = frozen;
        let target_stock = if shock.prepared_depot {
            closure_orders
        } else {
            c.opening_frozen_edible_kg
        };
        let freed_space = cold_capacity - frozen + o.shellfish_meat;
        let received = (o.shellfish_meat + target_stock)
            .saturating_sub(frozen)
            .min(freed_space)
            .min(coast_cap)
            .min(d.monthly_cold_freight_kg_km / c.partner_distance_km);
        total_received += received;
        let served = o.shellfish_meat.min(frozen + received);
        frozen = frozen + received - served;
        unmet_shellfish += o.shellfish_meat - served;
        check("shellfish_order_kg", o.shellfish_meat, served);
        let heat = o.lettuce * s.lettuce.heat_kwh_per_kg_by_month[m];
        check("usable_heat_kwh", heat, d.monthly_heat_kwh);
        let fish_wh = if ras {
            f.ras_electric_wh_per_live_kg
        } else {
            f.river_electric_wh_per_live_kg
        };
        let electricity = (o.lettuce * s.lettuce.electric_wh_per_kg
            + live * fish_wh
            + received * c.cold_chain_wh_per_edible_kg
            + opening_frozen * c.store_wh_per_opening_kg_month)
            .div_ceil(1000)
            + water_electric;
        let power = if shock.power_loss && t == 0 {
            d.monthly_power_kwh / 2
        } else {
            d.monthly_power_kwh
        };
        total_electricity += electricity;
        total_heat += heat;
        check("electricity_kwh", electricity, power);
        if ras && shock.outage_hours > 0 && t == 0 {
            check(
                "backup_rating_kw",
                f.critical_ras_load_kw,
                d.backup_rating_kw,
            );
            check(
                "backup_energy_kwh",
                shock.outage_hours * f.critical_ras_load_kw,
                d.usable_backup_kwh,
            );
        }
        if !gaps.is_empty() {
            failures.push(
                json!({"month":format!("{}-{:02}",s.first_year+t as u64/12,m+1),"missing":gaps}),
            );
        }
    }
    json!({"months_screened":s.months,"months_with_shortfall":failures.len(),"shortfalls":failures,
        "well_raw_m3":extraction,"well_pumping_kwh":pumping,"well_treatment_kwh":treatment,
        "aquifer_closing_m3":stock,"aquifer_minimum_closing_m3":min_stock,"road_water_tonne_km":hauled,
        "monthly_accepted_purge_to_greenhouse_m3":reused,"monthly_makeup_or_delivered_water_m3":clean,"monthly_river_gross_m3":river_need,
        "fish_live_kg_per_month":live,"compound_feed_kg_per_month":feed,
        "total_site_electricity_required_kwh":total_electricity,"total_heat_required_kwh":total_heat,
        "cold_store_capacity_kg":cold_capacity,"received_shellfish_meat_kg":total_received,
        "aquifer_opening_restoration_gap_m3":w.opening_m3.saturating_sub(stock),
        "cold_reserve_restoration_gap_kg":c.opening_frozen_edible_kg.saturating_sub(frozen),
        "unserved_shellfish_kg":unmet_shellfish,"frozen_closing_kg":frozen})
}

fn site() -> Site {
    serde_json::from_str(include_str!("../fixtures/food-technologies.json")).unwrap()
}
fn main() {
    let s = site();
    let l = &s.lettuce;
    let area = |yield_tenths| (s.monthly_orders_kg.lettuce * 12 * 10).div_ceil(yield_tenths);
    let mut cases = serde_json::Map::new();
    for (name, route, shock) in [
        (
            "planned_coastal_closure_with_depot",
            Route::WellRas,
            Shock {
                closures: true,
                prepared_depot: true,
                ..Shock::default()
            },
        ),
        (
            "eight_hour_outage",
            Route::WellRas,
            Shock {
                outage_hours: 8,
                ..Shock::default()
            },
        ),
        (
            "treated_fish_water_to_greenhouse",
            Route::IntegratedRas,
            Shock::default(),
        ),
        ("local_well_ras", Route::WellRas, Shock::default()),
        ("delivered_water_ras", Route::HauledRas, Shock::default()),
        ("river_flow_through", Route::River, Shock::default()),
        (
            "half_net_recharge",
            Route::WellRas,
            Shock {
                dry: true,
                ..Shock::default()
            },
        ),
        (
            "fourfold_pump_head",
            Route::WellRas,
            Shock {
                deep: true,
                ..Shock::default()
            },
        ),
        (
            "three_month_coastal_closure",
            Route::WellRas,
            Shock {
                closures: true,
                ..Shock::default()
            },
        ),
        (
            "winter_power_shortfall",
            Route::WellRas,
            Shock {
                power_loss: true,
                ..Shock::default()
            },
        ),
    ] {
        cases.insert(name.to_owned(), run(&s, route, shock));
    }
    let report = json!({"lettuce_reference":{"greenhouse_growing_area_m2":area(l.greenhouse_yield_tenths_kg_per_m2_year),
        "open_field_growing_area_m2":area(l.field_yield_tenths_kg_per_m2_year),"field_annual_water_m3":(s.monthly_orders_kg.lettuce*12*l.field_l_per_kg).div_ceil(1000)},
        "cases":cases});
    fs::write(
        env::args().nth(1).expect("output path"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drought_never_pumps_the_protected_stock_and_shortfalls_accumulate() {
        let mut s = site();
        s.well.opening_m3 = s.well.protected_floor_m3 + 245;
        s.well.net_inflow_m3_by_month = [0; 12];
        let r = run(&s, Route::WellRas, Shock::default());
        assert_eq!(r["well_raw_m3"], 245);
        assert_eq!(r["aquifer_closing_m3"], 500);
        assert_eq!(r["months_with_shortfall"], 59);
        assert_eq!(r["shortfalls"][0]["missing"]["treated_makeup_m3"], 220);
    }
    #[test]
    fn closures_consume_only_available_frozen_product_and_resume_under_freight_cap() {
        let s = site();
        let r = run(
            &s,
            Route::WellRas,
            Shock {
                closures: true,
                ..Shock::default()
            },
        );
        assert_eq!(r["unserved_shellfish_kg"], 3000);
        assert_eq!(r["frozen_closing_kg"], 300);
        assert_eq!(r["shortfalls"][0]["month"], "2027-08");
    }
    #[test]
    fn groundwater_and_river_cannot_both_claim_one_basin_allocation() {
        let mut s = site();
        s.district.shared_basin_gross_allocation_m3 = 5000;
        let r = run(&s, Route::River, Shock::default());
        assert_eq!(r["shortfalls"][0]["missing"]["river_gross_intake_m3"], 23);
    }

    #[test]
    fn reuse_is_limited_by_quality_accepted_purge_and_plant_demand() {
        let mut s = site();
        let r = run(&s, Route::IntegratedRas, Shock::default());
        assert_eq!(r["well_raw_m3"], 13380);
        assert_eq!(r["monthly_accepted_purge_to_greenhouse_m3"], 20);
        s.fish.usable_purge_m3_per_month = 4;
        let r = run(&s, Route::IntegratedRas, Shock::default());
        assert_eq!(r["monthly_accepted_purge_to_greenhouse_m3"], 3);
        s.fish.usable_purge_m3_per_month = 0;
        assert_eq!(
            run(&s, Route::IntegratedRas, Shock::default()),
            run(&s, Route::WellRas, Shock::default())
        );
    }
    #[test]
    fn advance_orders_bridge_the_closure_without_extra_opening_stock() {
        let s = site();
        let r = run(
            &s,
            Route::WellRas,
            Shock {
                closures: true,
                prepared_depot: true,
                ..Shock::default()
            },
        );
        assert_eq!(r["months_with_shortfall"], 0);
        assert_eq!(r["received_shellfish_meat_kg"], 18000);
        assert_eq!(r["frozen_closing_kg"], 300);
        assert_eq!(r["cold_store_capacity_kg"], 900);
    }
    #[test]
    fn continuous_aeration_has_an_intra_month_power_constraint() {
        let s = site();
        let r = run(
            &s,
            Route::WellRas,
            Shock {
                outage_hours: 8,
                ..Shock::default()
            },
        );
        assert_eq!(
            r["shortfalls"][0]["missing"],
            json!({"backup_energy_kwh":8})
        );
    }
    #[test]
    fn water_and_power_routes_change_only_their_own_resource_claims() {
        let s = site();
        let local = run(&s, Route::WellRas, Shock::default());
        let hauled = run(&s, Route::HauledRas, Shock::default());
        assert_eq!(local["months_with_shortfall"], 0);
        assert_eq!(hauled["months_with_shortfall"], 0);
        assert_eq!(local["well_pumping_kwh"], 2340);
        assert_eq!(hauled["road_water_tonne_km"], 462000);
        assert_eq!(hauled["well_raw_m3"], 0);
        let river = run(&s, Route::River, Shock::default());
        assert_eq!(
            river["shortfalls"][0]["missing"]["river_gross_intake_m3"],
            500
        );
    }
}
