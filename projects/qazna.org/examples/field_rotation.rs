//! Replay six crop seasons serving a predecessor year and 60 population targets.
use qazna::{
    food_routes::{FoodRoutes, HarvestRoute},
    horizon::{Horizon, Period, PeriodPlan, Process},
    mathematics::verify_linear,
    model_scenarios::ModelInput,
    population::PopulationStudy,
    seasonal::{CropUse, LandPool, Seasonal},
};
use serde_json::{Value, json};
use std::{env, fs};

const YEARS: usize = 6;
const MONTHS: usize = YEARS * 12;
const BLOCKS: usize = 4;
const FOODS: usize = 3;
const FIELD_PROCESSES: usize = BLOCKS * FOODS;
const MANUAL: [usize; 6] = [15, 17, 19, 21, 23, 25];

fn calendar(t: usize) -> String {
    format!("{}-{:02}", 2026 + t / 12, t % 12 + 1)
}

fn area(total: i64, block: usize) -> i64 {
    total / 4 + i64::from((block as i64) < total % 4)
}

fn rest(year: usize) -> usize {
    year % BLOCKS
}

// On a given field the sequence is fallow, food grain, feed grain, flexible grain.
fn phase(year: usize, block: usize) -> usize {
    (year + BLOCKS - block) % BLOCKS
}

fn permitted(year: usize, block: usize, crop: usize) -> bool {
    match phase(year, block) {
        1 => crop < 2,
        2 => crop == 2,
        3 => true,
        _ => false,
    }
}

// Fill the dedicated slots first; any remaining food and feed share the flexible slot.
fn assignment(total: i64, year: usize, demand: [i64; FOODS]) -> Option<[[i64; FOODS]; BLOCKS]> {
    let mut remaining = demand;
    let mut fields = [[0; FOODS]; BLOCKS];
    for slot in 1..BLOCKS {
        let block = (rest(year) + BLOCKS - slot) % BLOCKS;
        let mut free = area(total, block);
        for (crop, q) in remaining.iter_mut().enumerate() {
            if permitted(year, block, crop) {
                let plant = free.min(*q);
                fields[block][crop] = plant;
                *q -= plant;
                free -= plant;
            }
        }
    }
    (remaining == [0; FOODS]).then_some(fields)
}

// Minimum total for these annual sowings and fixed, nearly equal-sized blocks.
fn land_threshold(annual: &[[i64; FOODS]]) -> i64 {
    let mut low = 0;
    let mut high = 4 * annual.iter().map(|a| a.iter().sum::<i64>()).max().unwrap();
    while low < high {
        let mid = low + (high - low) / 2;
        if annual
            .iter()
            .enumerate()
            .all(|(year, demand)| assignment(mid, year, *demand).is_some())
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}

fn source_period(t: usize) -> usize {
    // The predecessor repeats the first twelve baseline needs and capacities.
    if t < 12 { t } else { t - 12 }
}

fn targets(input: &ModelInput, study: &PopulationStudy) -> Vec<[i64; FOODS]> {
    let population = study.project().unwrap();
    assert_eq!(population.months.len(), MONTHS - 12);
    (0..MONTHS)
        .map(|t| {
            let h = &input.model.seasonal.physical;
            let mut needs: [i64; FOODS] =
                h.periods[source_period(t)].needs[6..9].try_into().unwrap();
            if t >= 12 {
                let month = &population.months[t - 12];
                assert_eq!(month.calendar, calendar(t));
                for (crop, item) in ["rice_component", "oats_component"].into_iter().enumerate() {
                    let d = month.demands.iter().find(|d| d.item == item).unwrap();
                    assert_eq!(d.unit, "gross-equivalent kg");
                    needs[crop] = d.central.as_ref().unwrap().parse().unwrap();
                }
            }
            needs
        })
        .collect()
}

fn grain_recipe(recipe: &Process) -> Process {
    Process {
        inputs: recipe.inputs[..9].to_vec(),
        outputs: recipe.outputs[..9].to_vec(),
        lead_periods: recipe.lead_periods,
    }
}

fn build(input: &ModelInput, needs: &[[i64; FOODS]], yield_percent: i64) -> Seasonal {
    let source = &input.model.seasonal;
    let h = &source.physical;
    let mut crops = Vec::new();
    let mut processes = Vec::new();
    for block in 0..BLOCKS {
        for crop in 0..FOODS {
            let rule = &source.crops[crop];
            assert_eq!((rule.process, rule.hectares_per_activity), (crop, 1));
            let mut recipe = grain_recipe(&h.processes[crop]);
            assert_eq!(
                (rule.sowing_periods.as_slice(), recipe.lead_periods),
                (&[3][..], 5)
            );
            // The fixture uses annual processes for the same crop. Guard reuse of year one.
            for y in 1..5 {
                let later = &source.crops[FOODS * y + crop];
                let upstream = grain_recipe(&h.processes[later.process]);
                assert_eq!(later.sowing_periods, [12 * y + 3]);
                assert_eq!(
                    later.irrigation_m3_per_activity,
                    rule.irrigation_m3_per_activity
                );
                assert_eq!(
                    (upstream.inputs, upstream.outputs, upstream.lead_periods),
                    (
                        recipe.inputs.clone(),
                        recipe.outputs.clone(),
                        recipe.lead_periods
                    )
                );
            }
            recipe.outputs[crop] = recipe.outputs[crop] * yield_percent / 100;
            crops.push(CropUse{process:processes.len(),land_pool:block,hectares_per_activity:1,
                sowing_periods:(0..YEARS).filter(|&year|permitted(year, block, crop)).map(|y|y*12+3).collect(),
                irrigation_m3_per_activity:rule.irrigation_m3_per_activity.clone(),
                basis:format!("Field block {block}; one non-producing fallow season in four; retained gross yield {yield_percent}% of reference.")});
            processes.push(recipe);
        }
    }
    for j in MANUAL {
        let recipe = grain_recipe(&h.processes[j]);
        assert_eq!(recipe.lead_periods, 1);
        processes.push(recipe);
    }
    let periods = needs
        .iter()
        .enumerate()
        .map(|(t, food)| {
            let mut row = Period {
                supplies: vec![0; 9],
                needs: vec![0; 9],
                capacities: vec![0; processes.len()],
            };
            row.needs[6..].copy_from_slice(food);
            for (j, old) in MANUAL.iter().enumerate() {
                row.capacities[FIELD_PROCESSES + j] = h.periods[source_period(t)].capacities[*old];
            }
            row
        })
        .collect();
    let mut floors = vec![vec![0; 9]; MONTHS];
    floors[MONTHS - 1].copy_from_slice(&source.closing_floors.last().unwrap()[..9]);
    Seasonal {
        physical: Horizon {
            resources: h.resources[..9].to_vec(),
            initial: h.initial[..9].to_vec(),
            storage_caps: h.storage_caps[..9].to_vec(),
            processes,
            periods,
            objective_resource: 6,
        },
        crops,
        land: (0..BLOCKS)
            .map(|b| LandPool {
                name: format!("fixed field group {b}"),
                available_hectares: vec![0; MONTHS],
            })
            .collect(),
        irrigation_m3: (0..MONTHS)
            .map(|t| source.irrigation_m3[source_period(t)])
            .collect(),
        closing_floors: floors,
    }
}

fn candidate(model: &mut Seasonal) -> (Vec<PeriodPlan>, Vec<[i64; FOODS]>, i64) {
    let h = &mut model.physical;
    let last = MONTHS - 1;
    let mut plan: Vec<_> = h
        .periods
        .iter()
        .map(|p| PeriodPlan {
            activities: vec![0; h.processes.len()],
            deliveries: p.needs.clone(),
            closing: vec![0; 9],
            discarded: vec![0; 9],
        })
        .collect();
    let mut annual = vec![[0; FOODS]; YEARS];
    for (crop, recipe) in h.processes[..FOODS].iter().enumerate() {
        let pack = FIELD_PROCESSES + 2 * crop;
        let ship = pack + 1;
        // Opening finished stock covers January; opening preserved stock supplies
        // the first dispatch. Rebuild each of these stores at the terminal date.
        assert_eq!(h.initial[crop + 6], plan[0].deliveries[crop + 6]);
        for t in 0..last {
            plan[t].activities[ship] = plan[t + 1].deliveries[crop + 6];
        }
        plan[last - 1].activities[ship] += model.closing_floors[last][crop + 6];
        assert_eq!(h.initial[crop + 3], plan[0].activities[ship]);
        for t in 0..last {
            plan[t].activities[pack] = plan[t + 1].activities[ship];
        }
        plan[last - 1].activities[pack] += model.closing_floors[last][crop + 3];
        // Each harvest supplies preservation until the next harvest and the
        // seed for the next sowing. At the last harvest retain the raw floor.
        let mut carry = model.closing_floors[last][crop];
        for year in (0..YEARS).rev() {
            let harvest = 12 * year + 8;
            let end = (harvest + 12).min(MONTHS);
            let gross_required = carry
                + plan[harvest..end]
                    .iter()
                    .map(|p| p.activities[pack])
                    .sum::<i64>();
            let gross = recipe.outputs[crop];
            annual[year][crop] = gross_required / gross + i64::from(gross_required % gross != 0);
            carry = annual[year][crop] * recipe.inputs[crop];
        }
    }
    let total = land_threshold(&annual);
    for (b, pool) in model.land.iter_mut().enumerate() {
        pool.available_hectares.fill(area(total, b));
        for p in &mut h.periods {
            p.capacities[b * FOODS..(b + 1) * FOODS].fill(area(total, b));
        }
    }
    for (year, required) in annual.iter().enumerate() {
        let fields = assignment(total, year, *required).unwrap();
        for (block, planted) in fields.iter().enumerate() {
            let first = block * FOODS;
            plan[12 * year + 3].activities[first..first + FOODS].copy_from_slice(planted);
        }
    }
    let mut stock = h.initial.clone();
    let mut arrivals = vec![[0; 9]; MONTHS];
    for (t, row) in plan.iter_mut().enumerate() {
        for (r, q) in stock.iter_mut().enumerate() {
            *q += arrivals[t][r] - row.deliveries[r];
        }
        for (j, &x) in row.activities.iter().enumerate().filter(|(_, x)| **x > 0) {
            let recipe = &h.processes[j];
            for (r, q) in stock.iter_mut().enumerate() {
                *q -= x * recipe.inputs[r];
                arrivals[t + recipe.lead_periods][r] += x * recipe.outputs[r];
            }
        }
        row.closing.clone_from(&stock);
    }
    (plan, annual, total)
}

fn compare(input: &ModelInput, needs: &[[i64; FOODS]], yield_percent: i64) -> Value {
    let mut model = build(input, needs, yield_percent);
    let (plan, annual, land) = candidate(&mut model);
    let source = &input.model.seasonal;
    let mut minimum_sown = 0;
    for crop in 0..FOODS {
        let delivery: i64 = needs.iter().map(|row| row[crop]).sum();
        for r in [crop, crop + 3, crop + 6] {
            assert_eq!(
                model.closing_floors[MONTHS - 1][r],
                model.physical.initial[r]
            );
        }
        let recipe = &model.physical.processes[crop];
        let net = recipe.outputs[crop] - recipe.inputs[crop];
        minimum_sown += delivery / net + i64::from(delivery % net != 0);
    }
    let mut reference_active_area = 0;
    let mut reference_water_upper_area = 0;
    let mut base_water_gaps = Vec::new();
    let mut land_gaps = Vec::new();
    let mut assignments = Vec::new();
    let mut water = vec![0; MONTHS];
    for (year, hectares) in annual.iter().enumerate() {
        let sow = year * 12 + 3;
        let original = source.land[0].available_hectares[source_period(sow)];
        let active = original - area(original, rest(year));
        reference_active_area += active;
        // All modeled grains draw water in the same five stages. Even if the crop mix
        // changed, each hectare would need at least the smallest crop coefficient.
        let water_upper = (0..5)
            .filter_map(|stage| {
                let minimum = model
                    .crops
                    .iter()
                    .map(|c| c.irrigation_m3_per_activity[stage])
                    .min()
                    .unwrap();
                (minimum > 0).then(|| model.irrigation_m3[sow + stage] / minimum)
            })
            .min()
            .unwrap();
        reference_water_upper_area += water_upper;
        for (crop, &planted) in hectares.iter().enumerate() {
            let src = source_period(sow);
            let j = source.crops[src / 12 * FOODS + crop].process;
            assert!(planted <= source.physical.periods[src].capacities[j]);
        }
        let need = hectares.iter().sum::<i64>();
        land_gaps.push((need - active).max(0));
        for c in &model.crops {
            for (stage, per_ha) in c.irrigation_m3_per_activity.iter().enumerate() {
                water[sow + stage] += per_ha * plan[sow].activities[c.process];
            }
        }
        assignments.push(json!({"sowing":calendar(sow),"resting_group":rest(year),"hectares_by_crop":hectares,
            "hectares_by_field_and_crop":plan[sow].activities[..FIELD_PROCESSES].chunks(FOODS).collect::<Vec<_>>() }));
    }
    for (t, &needed) in water.iter().enumerate() {
        let available = model.irrigation_m3[t];
        if needed > available {
            base_water_gaps.push(json!({"month":calendar(t),"missing_m3":needed-available}));
        }
        // The candidate requests extra water only in months where it is necessary.
        model.irrigation_m3[t] = available.max(needed);
    }
    let verified = model.replay(&plan).unwrap();
    assert_eq!(water, verified.irrigation_m3_by_period);
    verify_linear(
        &model.compile().unwrap(),
        &model.physical.vector(&plan).unwrap(),
        None,
    )
    .unwrap();
    let inherited = input.food_routes.as_ref().unwrap();
    assert!(
        inherited
            .harvests
            .iter()
            .all(|c| c.shelf_periods == inherited.harvests[0].shelf_periods)
    );
    let food = FoodRoutes {
        resources: (0..9).collect(),
        initial_remaining_periods: inherited.initial_remaining_periods.clone(),
        harvests: model
            .crops
            .iter()
            .map(|c| HarvestRoute {
                process: c.process,
                shelf_periods: inherited.harvests[0].shelf_periods,
            })
            .collect(),
        transfers: (FIELD_PROCESSES..FIELD_PROCESSES + 6).collect(),
        unit: inherited.unit.clone(),
        basis: inherited.basis.clone(),
    };
    let dated = food.audit_seasonal(&model, &plan).unwrap();
    assert_eq!(dated.final_in_transit, "0");
    let peak = *water.iter().max().unwrap();
    let peak_month = water.iter().position(|v| *v == peak).unwrap();
    let mut dry = model.clone();
    dry.irrigation_m3[peak_month] = peak * 80 / 100;
    let linear = dry.compile().unwrap();
    assert!(verify_linear(&linear, &dry.physical.vector(&plan).unwrap(), None).is_err());
    assert!(dry.replay(&plan).is_err());
    // Move one actual sowing onto the resting field without changing the
    // total seed, production, irrigation, delivery or reported stock.
    let mut invalid = plan.clone();
    let row = &mut invalid[3].activities;
    let from = (FOODS..FIELD_PROCESSES).find(|&j| row[j] > 0).unwrap();
    row[from] -= 1;
    row[from % FOODS] += 1;
    assert!(model.replay(&invalid).is_err());
    assert!(
        verify_linear(
            &model.compile().unwrap(),
            &model.physical.vector(&invalid).unwrap(),
            None
        )
        .is_err()
    );
    json!({"gross_yield_percent":yield_percent,"fixed_field_area_ha":land,"required_peak_water_m3":peak,
        "peak_month":calendar(peak_month),"restored_terminal_stocks_kg":dated.final_stock,"expired_kg":dated.expired,
        "seed_consumed_kg":dated.seed_withdrawn,
        "six_season_bound": {
            "minimum_planted_hectare_seasons": minimum_sown,
            "original_rotation_upper_hectare_seasons": reference_active_area,
            "original_water_upper_hectare_seasons": reference_water_upper_area,
            "original_rotation_shortfall_at_least": (minimum_sown-reference_active_area).max(0),
            "original_water_shortfall_at_least": (minimum_sown-reference_water_upper_area).max(0)
        },
        "monthly_required_water_m3":water,"seasons":assignments,"reference_rotated_area_shortfalls_ha":land_gaps,
        "reference_water_shortfalls":base_water_gaps,"peak_month_water_shortfall_if_20_percent_lost_m3":peak-dry.irrigation_m3[peak_month]})
}

fn main() {
    let v: Value =
        serde_json::from_slice(&fs::read("fixtures/extended-scenario.json").unwrap()).unwrap();
    let input: ModelInput = serde_json::from_value(v["input"].clone()).unwrap();
    let study: PopulationStudy =
        serde_json::from_slice(&fs::read("fixtures/population-study.json").unwrap()).unwrap();
    let needs = targets(&input, &study);
    let result = json!({"period_zero":"2026-01","target_months":60,"monthly_targets_kg":needs[12..],"crop_order":["rice_component","oats_component","feed_grain_equivalent"],
        "field_history":"Each fixed field group cycles through fallow, rice or oats, feed grain, then flexible grain. Group zero rests in 2026, group one in 2027, and so on. The same phases are assumed before 2026. No yield, irrigation, or nutrient credit is assigned to dry fallow.",
        "scope":"Grain, seed, dated shelf life, finite grain storage, one-month preservation and one-month dispatch, per-route throughput, sowing windows, land and stage water. Shared energy, materials, labour, equipment and treasury for these larger flows are outside this crop balance.",
        "cases":[compare(&input,&needs,100),compare(&input,&needs,120),compare(&input,&needs,85)]});
    fs::write(
        env::args().nth(1).expect("output path"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_sizes_and_crop_specific_slots_match_an_independent_area_bound() {
        // The flexible slot can absorb the sum of both dedicated-slot overflows.
        for total in 0..=20 {
            for year in 0..4 {
                let food_block = (year + 3) % 4;
                let feed_block = (year + 2) % 4;
                let flexible = (year + 1) % 4;
                for rice in 0..=5 {
                    for oats in 0..=4 {
                        for feed in 0..=8 {
                            let demand = [rice, oats, feed];
                            let overflow = (rice + oats - area(total, food_block)).max(0)
                                + (feed - area(total, feed_block)).max(0);
                            let actual = assignment(total, year, demand);
                            assert_eq!(actual.is_some(), overflow <= area(total, flexible));
                            if let Some(fields) = actual {
                                assert_eq!(fields[year], [0; 3]);
                                assert_eq!(fields[food_block][2], 0);
                                assert_eq!(fields[feed_block][..2], [0, 0]);
                                for (b, group) in fields.iter().enumerate() {
                                    assert!(group.iter().sum::<i64>() <= area(total, b));
                                }
                                for (c, &d) in demand.iter().enumerate() {
                                    assert_eq!(fields.iter().map(|group| group[c]).sum::<i64>(), d);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
