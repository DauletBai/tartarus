//! Offline geographic baseline and gross grain balance for a ten-billion-person scenario.
use qazna::mathematics::proportional_quota;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, error::Error, fs};

const INPUT: &str = include_str!("../fixtures/geography-baseline.json");
const TARGET: u64 = 10_000_000_000;

#[derive(Deserialize)]
struct Baseline {
    world: World,
    kazakhstan: Kazakhstan,
}

#[derive(Deserialize)]
struct World {
    july_world_population: BTreeMap<u16, u64>,
    areas: Vec<Area>,
}

#[derive(Deserialize)]
struct Area {
    un_location_id: u64,
    iso3: String,
    july_population: BTreeMap<u16, u64>,
    arable_ha_2023: Option<f64>,
    internal_renewable_water_billion_m3_per_year_2022_entry: Option<f64>,
    cereal_t_2024: Option<f64>,
    installed_renewables_w_per_person_2023: Option<f64>,
    long_run_precip_mm_2022_entry: Option<f64>,
    copper_reserves_t_mcs2026: Option<u64>,
}

#[derive(Deserialize)]
struct Kazakhstan {
    national: Region,
    regions: BTreeMap<String, Region>,
}

#[derive(Deserialize)]
struct Region {
    people_2026_10_01: u64,
    urban_people_2026_10_01: u64,
    rural_people_2026_10_01: u64,
    #[serde(default)]
    districts: BTreeMap<String, District>,
    annual_grain_pulses_t: BTreeMap<u16, Option<u64>>,
    grain_sown_ha_2024: Option<u64>,
    milk_t_2024: Option<u64>,
    carcass_meat_t_2024: Option<u64>,
    eggs_count_2024: Option<u64>,
    vegetables_t_2024: Option<u64>,
    potatoes_t_2024: Option<u64>,
    portland_cement_excl_white_t_2024: Option<u64>,
    pumps_produced_2024: Option<u64>,
    electricity_kwh_2024: Option<u64>,
    monthly_weather: BTreeMap<String, Option<Weather>>,
}

fn constant_output_per_person(
    region: &Region,
    future: u64,
) -> Result<BTreeMap<&'static str, Option<u64>>, Box<dyn Error>> {
    let base = u128::from(region.people_2026_10_01);
    if base == 0 {
        return Err("undefined per-resident production in an empty area".into());
    }
    [
        ("milk_t", region.milk_t_2024),
        ("carcass_meat_t", region.carcass_meat_t_2024),
        ("eggs_count", region.eggs_count_2024),
        ("vegetables_t", region.vegetables_t_2024),
        ("potatoes_t", region.potatoes_t_2024),
        (
            "portland_cement_excl_white_t",
            region.portland_cement_excl_white_t_2024,
        ),
        ("pumps_count", region.pumps_produced_2024),
        ("electricity_kwh", region.electricity_kwh_2024),
    ]
    .into_iter()
    .map(|(key, value)| {
        let projected = value.map(|v| (u128::from(v) * u128::from(future)).div_ceil(base));
        Ok((key, projected.map(u64::try_from).transpose()?))
    })
    .collect()
}

#[derive(Deserialize)]
struct District {
    people_2026_10_01: u64,
}

#[derive(Deserialize)]
struct Weather {
    precipitation_anomaly_mm: f64,
    temperature_anomaly_c: f64,
}

fn at_crossing(world: &World) -> Result<(u16, u16, Vec<u64>), Box<dyn Error>> {
    let (&hi, &upper) = world
        .july_world_population
        .iter()
        .find(|&(_, p)| *p >= TARGET)
        .ok_or("population threshold outside the supplied horizon")?;
    let lo = hi.checked_sub(1).ok_or("invalid year")?;
    let lower = *world
        .july_world_population
        .get(&lo)
        .ok_or("missing preceding year")?;
    if lower >= TARGET || upper <= lower {
        return Err("invalid crossing interval".into());
    }
    let mut weights = Vec::new();
    for area in &world.areas {
        let a = area
            .july_population
            .get(&lo)
            .ok_or("missing area in preceding year")?;
        let b = area
            .july_population
            .get(&hi)
            .ok_or("missing area in crossing year")?;
        let weight = u128::from(upper - TARGET) * u128::from(*a)
            + u128::from(TARGET - lower) * u128::from(*b);
        weights.push((area.un_location_id, u64::try_from(weight)?));
    }
    let people = proportional_quota(&weights, TARGET)?;
    if people.iter().sum::<u64>() != TARGET {
        return Err("incomplete population allocation".into());
    }
    Ok((lo, hi, people))
}

fn gap_kg(harvest_t: Option<u64>, people: u64, kg_per_person: u32) -> Option<u64> {
    harvest_t.map(|t| (people * u64::from(kg_per_person)).saturating_sub(t * 1000))
}

// Table 4.54 is printed in tenths of a thousand tonnes; a dash means no output.
// Half a reporting unit on either side includes both possible rounding ties.
fn harvest_bounds_kg(tonnes: u64) -> (u128, u128) {
    if tonnes == 0 {
        return (0, 0);
    }
    let kg = u128::from(tonnes) * 1000;
    (kg.saturating_sub(50_000), kg + 50_000)
}

fn territorial_grain_balance(
    regions: impl Iterator<Item = (u64, Option<u64>)>,
    national_t: Option<u64>,
    kg_per_person: u32,
) -> Result<Value, Box<dyn Error>> {
    let mut reported_t = 0_u128;
    let (mut known_low, mut known_high) = (0_u128, 0_u128);
    let (mut gap_low, mut gap_high) = (0_u128, 0_u128);
    let (mut missing_regions, mut missing_need) = (0_u32, 0_u128);
    let mut total_need = 0_u128;
    for (population, harvest) in regions {
        let need = u128::from(population) * u128::from(kg_per_person);
        total_need += need;
        if let Some(t) = harvest {
            let (low, high) = harvest_bounds_kg(t);
            reported_t += u128::from(t);
            known_low += low;
            known_high += high;
            gap_low += need.saturating_sub(high);
            gap_high += need.saturating_sub(low);
        } else {
            missing_regions += 1;
            missing_need += need;
        }
    }
    gap_high += missing_need;
    let mut unreported_ceiling = None;
    if let Some(t) = national_t {
        let (low, high) = harvest_bounds_kg(t);
        if high < known_low || (missing_regions == 0 && low > known_high) {
            return Err(
                "regional harvest is incompatible with the national reporting interval".into(),
            );
        }
        let available = high - known_low;
        unreported_ceiling = Some(if missing_regions == 0 { 0 } else { available });
        gap_low += missing_need.saturating_sub(available);
        gap_low = gap_low.max(total_need.saturating_sub(high));
    }
    let difference = national_t.map(|t| i128::from(t) - reported_t as i128);
    Ok(json!({
        "national_minus_reported_regions_t":difference,
        "unreported_regions":missing_regions,
        "unreported_harvest_upper_bound_kg":unreported_ceiling,
        "sum_of_local_shortfalls_lower_bound_kg":gap_low,
        "sum_of_local_shortfalls_upper_bound_kg":gap_high
    }))
}

// Score a forecast with values that precede the target harvest. Missing history
// makes the forecast unavailable even when the outcome is known.
fn harvest_backtest(region: &Region, year: u16) -> Option<Value> {
    let history = &region.annual_grain_pulses_t;
    let mut past = [
        history.get(&(year - 3)).copied().flatten()?,
        history.get(&(year - 2)).copied().flatten()?,
        history.get(&(year - 1)).copied().flatten()?,
    ];
    past.sort_unstable();
    let actual = history.get(&year).copied().flatten()?;
    Some(
        json!({"median_of_previous_three_years_t":past[1], "observed_t":actual,
        "absolute_error_t":past[1].abs_diff(actual)}),
    )
}

fn growing_season(r: &Region, year: u16) -> Option<Value> {
    let mut rain = 0.;
    let mut heat = 0.;
    for month in 4..=9 {
        let weather = r
            .monthly_weather
            .get(&format!("{year}-{month:02}"))?
            .as_ref()?;
        rain += weather.precipitation_anomaly_mm;
        heat += weather.temperature_anomaly_c;
    }
    Some(json!({"precipitation_anomaly_mm":(rain*10.).round()/10.,
                "mean_monthly_temperature_anomaly_c":(heat/6.*100.).round()/100.}))
}

// Callers pass rows in stable key order. A common multiplier allows growth above the base.
fn fixed_shares(
    counts: impl Iterator<Item = u64>,
    target: u64,
) -> Result<Vec<u64>, Box<dyn Error>> {
    let counts: Vec<_> = counts.collect();
    let total = counts.iter().sum::<u64>();
    if total == 0 {
        return Err("empty geographic population".into());
    }
    let scale = target.div_ceil(total).max(1);
    let weights: Vec<_> = counts
        .into_iter()
        .enumerate()
        .map(|(i, n)| (i as u64, n * scale))
        .collect();
    Ok(proportional_quota(&weights, target)?)
}

fn report(b: &Baseline, kg_per_person: u32) -> Result<Value, Box<dyn Error>> {
    let (lo, hi, people) = at_crossing(&b.world)?;
    let mut coverage: BTreeMap<&str, (usize, u64)> = BTreeMap::new();
    let mut by_area = BTreeMap::new();
    let mut cereal_per_person = BTreeMap::new();
    for (a, p) in b.world.areas.iter().zip(people) {
        if by_area.insert(a.iso3.as_str(), p).is_some() {
            return Err("duplicate area code".into());
        }
        let kg = a
            .cereal_t_2024
            .filter(|_| p > 0)
            .map(|t| (t * 1000. / p as f64).round() as u64);
        cereal_per_person.insert(a.iso3.as_str(), kg);
        for (field, present) in [
            ("arable_land", a.arable_ha_2023.is_some()),
            (
                "internal_renewable_water",
                a.internal_renewable_water_billion_m3_per_year_2022_entry
                    .is_some(),
            ),
            ("cereal", a.cereal_t_2024.is_some()),
            (
                "renewables",
                a.installed_renewables_w_per_person_2023.is_some(),
            ),
            ("long_run_rain", a.long_run_precip_mm_2022_entry.is_some()),
            ("copper_reserves", a.copper_reserves_t_mcs2026.is_some()),
        ] {
            let tally = coverage.entry(field).or_default();
            if present {
                tally.0 += 1;
                tally.1 += p;
            }
        }
    }
    let k = &b.kazakhstan;
    let present = k.national.people_2026_10_01;
    let future = by_area["KAZ"];
    let sum: u64 = k.regions.values().map(|r| r.people_2026_10_01).sum();
    if sum == 0 || sum != present {
        return Err("regional population differs from national total".into());
    }
    let distributed = fixed_shares(k.regions.values().map(|r| r.people_2026_10_01), future)?;
    let mut regions = BTreeMap::new();
    let mut known_gap = 0;
    let mut unknown_population = 0;
    for ((name, r), future_people) in k.regions.iter().zip(distributed) {
        if r.urban_people_2026_10_01 + r.rural_people_2026_10_01 != r.people_2026_10_01 {
            return Err("settlement counts do not reconcile".into());
        }
        let harvest = r.annual_grain_pulses_t[&2024];
        let gap = gap_kg(harvest, r.people_2026_10_01, kg_per_person);
        match gap {
            Some(kg) => known_gap += kg,
            None => unknown_population += r.people_2026_10_01,
        }
        let historical: BTreeMap<_, _> = r
            .annual_grain_pulses_t
            .iter()
            .map(|(year, t)| (year, gap_kg(*t, future_people, kg_per_person)))
            .collect();
        let seasons: BTreeMap<_, _> = (2023..=2025)
            .map(|year| {
                (
                    year,
                    json!({
                        "april_september":growing_season(r,year),
                        "gross_harvest_t":r.annual_grain_pulses_t.get(&year).copied().flatten()
                    }),
                )
            })
            .collect();
        let district_total: u64 = r.districts.values().map(|d| d.people_2026_10_01).sum();
        if district_total != r.people_2026_10_01 {
            return Err("districts do not reconcile to parent".into());
        }
        let district_people = fixed_shares(
            r.districts.values().map(|d| d.people_2026_10_01),
            future_people,
        )?;
        let district_plan: BTreeMap<_,_> = r.districts.iter().zip(district_people).map(|((key,d),p)| {
            (key, json!({"base_residents":d.people_2026_10_01, "base_gross_requirement_kg":d.people_2026_10_01 * u64::from(kg_per_person), "scenario_residents":p, "scenario_gross_requirement_kg":p * u64::from(kg_per_person)}))
        }).collect();
        regions.insert(name,json!({
            "residents_2026":r.people_2026_10_01, "scenario_residents_fixed_share":future_people,
            "observed_seasons":seasons,"districts":district_plan,
            "scenario_output_at_fixed_gross_output_per_resident":constant_output_per_person(r,future_people)?,
            "harvest_backtest":{"2023":harvest_backtest(r,2023),"2024":harvest_backtest(r,2024)},
            "harvest_2024_kg_per_sown_ha":harvest.zip(r.grain_sown_ha_2024).filter(|(_,ha)|*ha>0).map(|(t,ha)|t*1000/ha),
            "gross_gap_kg_2024_harvest_current_residents":gap,
            "gross_gap_kg_historical_harvest_scenario_residents":historical
        }));
    }
    let grain_balances: BTreeMap<_, _> = k
        .national
        .annual_grain_pulses_t
        .iter()
        .map(|(&year, &national)| {
            let rows = k.regions.values().map(|r| {
                (
                    r.people_2026_10_01,
                    r.annual_grain_pulses_t.get(&year).copied().flatten(),
                )
            });
            territorial_grain_balance(rows, national, kg_per_person).map(|bounds| (year, bounds))
        })
        .collect::<Result<_, _>>()?;
    Ok(json!({
        "gross_grain_pulses_scenario_kg_per_person_year":kg_per_person,
        "world":{"target_people":TARGET,"july_crossing_interval":[lo,hi],
            "scenario_people_by_iso3":by_area,"cereal_kg_per_scenario_person_at_2024_output":cereal_per_person,"coverage_areas_and_scenario_people":coverage},
        "kazakhstan":{"base_population":present,"scenario_population":future,
            "territorial_grain_balance_by_harvest_year_at_base_population":grain_balances,
            "scenario_output_at_fixed_gross_output_per_resident":constant_output_per_person(&k.national,future)?,
            "national_harvest_backtest":{"2023":harvest_backtest(&k.national,2023),"2024":harvest_backtest(&k.national,2024)},
            "base_gross_requirement_kg":present*u64::from(kg_per_person),
            "scenario_gross_requirement_kg":future*u64::from(kg_per_person),
            "known_local_gross_shortfalls_kg_2024":known_gap,
            "residents_with_unreported_local_2024_harvest":unknown_population,"regions":regions}
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: geography_study OUTPUT [gross kg per person per year]")?;
    let kg: u32 = args.next().as_deref().unwrap_or("100").parse()?;
    if kg == 0 || args.next().is_some() {
        return Err("expected a positive annual gross grain quantity".into());
    }
    let value = report(&serde_json::from_str(INPUT)?, kg)?;
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&value)?))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn district_needs_close_and_withheld_harvest_is_scored_out_of_sample() {
        let mut b: Baseline = serde_json::from_str(INPUT).unwrap();
        let study = report(&b, 120).unwrap();
        let k = &study["kazakhstan"];
        let mut places = 0;
        let mut kg = 0;
        for region in k["regions"].as_object().unwrap().values() {
            let children = region["districts"].as_object().unwrap();
            places += children.len();
            let total: u64 = children
                .values()
                .map(|d| d["scenario_residents"].as_u64().unwrap())
                .sum();
            assert_eq!(total, region["scenario_residents_fixed_share"]);
            kg += children
                .values()
                .map(|d| d["scenario_gross_requirement_kg"].as_u64().unwrap())
                .sum::<u64>();
        }
        assert_eq!(places, 228);
        assert_eq!(kg, k["scenario_gross_requirement_kg"]);
        assert_eq!(
            k["scenario_output_at_fixed_gross_output_per_resident"]["milk_t"],
            5_022_058
        );
        assert_eq!(
            k["regions"]["kostanay"]["districts"]["Рудный г.а."]["base_residents"],
            122_453
        );
        b.kazakhstan
            .national
            .annual_grain_pulses_t
            .insert(2024, Some(0));
        let score = harvest_backtest(&b.kazakhstan.national, 2024).unwrap();
        assert_eq!(score["median_of_previous_three_years_t"], 17_096_600);
        assert_eq!(score["absolute_error_t"], 17_096_600);
        b.kazakhstan
            .regions
            .get_mut("kostanay")
            .unwrap()
            .districts
            .clear();
        assert!(report(&b, 120).is_err());
    }

    #[test]
    fn population_closes_at_both_levels_under_reordering() {
        let mut b: Baseline = serde_json::from_str(INPUT).unwrap();
        let (_, _, p) = at_crossing(&b.world).unwrap();
        assert_eq!(p.iter().sum::<u64>(), TARGET);
        let first = report(&b, 100).unwrap();
        b.world.areas.reverse();
        assert_eq!(first, report(&b, 100).unwrap());
        let k = &first["kazakhstan"];
        assert_eq!(
            k["regions"]
                .as_object()
                .unwrap()
                .values()
                .map(|r| r["scenario_residents_fixed_share"].as_u64().unwrap())
                .sum::<u64>(),
            k["scenario_population"]
        );
        b.world.areas[0].july_population.remove(&2060);
        assert!(at_crossing(&b.world).is_err());
    }

    #[test]
    fn national_rounding_constrains_unreported_harvest_without_assigning_it() {
        let rows = [(100_000, Some(100)), (200_000, None), (30_000, Some(0))];
        let balance = territorial_grain_balance(rows.into_iter(), Some(100), 1).unwrap();
        assert_eq!(balance["unreported_harvest_upper_bound_kg"], 100_000);
        for actual_known in (50_000_u64..=150_000).step_by(1000) {
            for total in (actual_known..=150_000).step_by(1000) {
                let actual_gap = 100_000_u64.saturating_sub(actual_known) + 200_000
                    - (total - actual_known)
                    + 30_000;
                assert!(
                    actual_gap
                        >= balance["sum_of_local_shortfalls_lower_bound_kg"]
                            .as_u64()
                            .unwrap()
                );
                assert!(
                    actual_gap
                        <= balance["sum_of_local_shortfalls_upper_bound_kg"]
                            .as_u64()
                            .unwrap()
                );
            }
        }
        let unconstrained = territorial_grain_balance(rows.into_iter(), None, 1).unwrap();
        assert!(unconstrained["unreported_harvest_upper_bound_kg"].is_null());
        assert_eq!(
            unconstrained["sum_of_local_shortfalls_lower_bound_kg"],
            30_000
        );
        assert!(territorial_grain_balance(rows.into_iter(), Some(0), 1).is_err());
        assert!(territorial_grain_balance([(10, Some(0))].into_iter(), Some(200), 1).is_err());
    }

    #[test]
    fn missing_harvest_and_zero_harvest_have_different_consequences() {
        let b: Baseline = serde_json::from_str(INPUT).unwrap();
        let k = &report(&b, 100).unwrap()["kazakhstan"];
        let interval = &k["territorial_grain_balance_by_harvest_year_at_base_population"]["2024"];
        assert_eq!(interval["national_minus_reported_regions_t"], -100);
        assert_eq!(
            interval["sum_of_local_shortfalls_lower_bound_kg"],
            680_567_900
        );
        assert_eq!(
            interval["sum_of_local_shortfalls_upper_bound_kg"],
            681_717_900
        );
        assert_eq!(gap_kg(Some(0), 500, 100), Some(50_000));
        assert_eq!(gap_kg(Some(49), 500, 100), Some(1000));
        assert!(k["regions"]["mangystau"]["gross_gap_kg_2024_harvest_current_residents"].is_null());
        assert_eq!(k["residents_with_unreported_local_2024_harvest"], 827102);
        assert_eq!(
            k["regions"]["astana-city"]["gross_gap_kg_2024_harvest_current_residents"],
            169815000
        );
        assert!(
            k["regions"]["astana-city"]["observed_seasons"]["2025"]["april_september"]["precipitation_anomaly_mm"].is_null()
        );
        assert_eq!(
            k["regions"]["akmola"]["observed_seasons"]["2025"]["april_september"]["precipitation_anomaly_mm"],
            34.2
        );
    }
}
