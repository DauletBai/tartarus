//! Reconcile the national grain schedule with disjoint regional needs before replay.
use super::{FOODS, MONTHS, calendar, compare};
use qazna::{
    model_scenarios::ModelInput,
    population::{Demand, PopulationBinding},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

const CROPS: [&str; 2] = ["rice_component", "oats_component"];

fn grain_kg(demands: &[Demand], item: &str) -> i64 {
    let demand = demands
        .iter()
        .find(|row| row.item == item)
        .expect("grain norm");
    assert_eq!(demand.unit, "gross-equivalent kg");
    demand
        .central
        .as_ref()
        .expect("known grain need")
        .parse()
        .unwrap()
}

pub(super) fn study(input: &ModelInput, original: &[[i64; FOODS]], fixture: &Value) -> Value {
    let source: Value =
        serde_json::from_slice(&fs::read("research/service-results.json").unwrap()).unwrap();
    let binding: PopulationBinding =
        serde_json::from_value(source["runtime_fixture"]["input"]["population"].clone()).unwrap();
    let original_study: Value =
        serde_json::from_slice(&fs::read("fixtures/population-study.json").unwrap()).unwrap();
    assert_eq!(
        source["runtime_fixture"]["input"]["population"]["study"], original_study,
        "The profile and legacy schedules must use the same population assumptions"
    );
    let population = binding.study.project().unwrap();
    let profiles = binding.profiles.unwrap().project(&binding.study).unwrap();
    assert_eq!(population.initial, "100000000");
    assert_eq!(profiles.months.len(), MONTHS - 12);
    let mut revised = original.to_vec();
    let mut monthly = Vec::new();
    let mut extra = [0_i64; 2];
    let mut short_months = [0; 2];
    for (period, month) in profiles.months.iter().enumerate() {
        let t = period + 12;
        assert_eq!(month.calendar, calendar(t));
        let by_region: BTreeMap<_, _> = month
            .regions
            .iter()
            .map(|region| {
                (
                    region.region.as_str(),
                    CROPS.map(|item| grain_kg(&region.demands, item)),
                )
            })
            .collect();
        let mut delta = [0; 2];
        for (j, item) in CROPS.iter().enumerate() {
            // ProfilePlan aggregates disjoint person-days, applying each local norm
            // before adding regional quantities. Unknown norms must stay unknown.
            let quantity = grain_kg(&month.total, item);
            assert_eq!(quantity, by_region.values().map(|row| row[j]).sum::<i64>());
            revised[t][j] = quantity;
            delta[j] = quantity - original[t][j];
            extra[j] += delta[j];
            short_months[j] += usize::from(delta[j] > 0);
        }
        monthly.push(json!({"month":month.calendar,"opening_people":population.months[period].opening,
            "regional_requirements_kg":by_region, "legacy_target_kg":&original[t][..2], "profile_target_kg":&revised[t][..2], "difference_kg":delta}));
    }

    let cases = [85, 100, 120].map(|yield_percent| {
        let case = compare(input, &revised, yield_percent, fixture);
        let schedule = &case["renewed_schedule"];
        let money = &schedule["money"];
        let funded = &money["funded_continuation"];
        assert_eq!(funded["all_settled"], true);
        assert_eq!(money["opening_total"], funded["closing_total"]);
        json!({"yield_percent":yield_percent, "land_ha":case["fixed_field_area_ha"],
            "peak_irrigation_m3":case["required_peak_water_m3"], "physics_months":schedule["physical_months"],
            "opening_measure":money["opening_total"], "closing_measure":funded["closing_total"],
            "first_blocked_without_reallocation":money["first_blocked_month"],
            "treasury_reallocated":funded["allocation_total"], "settled_transfers":funded["settled"]})
    });
    json!({"sources":["fixtures/extended-scenario.json","fixtures/population-study.json",
           "research/service-results.json"], "from":"2027-01","through":"2031-12",
           "opening_people":population.initial,"closing_people":population.final_population,
           "crop_order":CROPS, "net_target_change_kg":extra, "months_with_short_target":short_months,
           "scope":"National grain delivery targets; feed and the 2026 predecessor retain their original schedules. Regions supply requirements; delivery to individual regions is not traced.",
           "monthly":monthly, "cases":cases})
}
