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

#[path = "support/field_renewal.rs"]
mod field_renewal;

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

// Reuse the dated non-food starts from the reference schedule. Track the best
// possible electricity and wrapping balance for these specific grain deadlines.
// Manual routes minimize both inputs; powered routes cannot improve this bound.
fn upstream_bound(input: &ModelInput, plan: &[PeriodPlan]) -> Value {
    let h = &input.model.seasonal.physical;
    let mut resources = Vec::new();
    for r in [9, 10, 27] {
        let mut available = h.initial[r];
        let mut minimum = available;
        let mut first = None;
        let mut grain_use = 0;
        for (t, food) in plan.iter().enumerate() {
            let reference = &input.plan[source_period(t)];
            available += h.periods[source_period(t)].supplies[r] - reference.deliveries[r];
            for (j, process) in h.processes.iter().enumerate().skip(27) {
                available -= reference.activities[j] * process.inputs[r];
                if let Some(start) = t.checked_sub(process.lead_periods) {
                    available +=
                        input.plan[source_period(start)].activities[j] * process.outputs[r];
                }
            }
            for (lane, &manual) in MANUAL.iter().enumerate() {
                let coefficient = h.processes[manual].inputs[r];
                assert!(h.processes[manual + 1].inputs[r] >= coefficient);
                let use_now = food.activities[FIELD_PROCESSES + lane] * coefficient;
                grain_use += use_now;
                available -= use_now;
            }
            if available < 0 && first.is_none() {
                first = Some(calendar(t));
            }
            minimum = minimum.min(available);
        }
        resources.push(
            json!({"resource":h.resources[r],"total_grain_inputs":grain_use,
            "first_uncovered_month":first,"minimum_additional_arrivals":(-minimum).max(0),
            "terminal_balance_before_remedy":available}),
        );
    }
    json!(resources)
}

// Curtail only ore extraction, residue recovery and wrapper fabrication when
// their dated inputs run out. No opening balances restart at the year boundary.
fn capped_packaging(input: &ModelInput, plan: &[PeriodPlan]) -> (Value, Vec<Vec<i64>>) {
    let mut schedules = Vec::new();
    let h = &input.model.seasonal.physical;
    let tracked = [9, 10, 27, 28, 29];
    let mut stock = h.initial.clone();
    let mut arrivals = vec![vec![0; stock.len()]; MONTHS];
    let mut minima = stock.clone();
    let mut curtailed = [0; 3];
    let mut work_gaps = Vec::new();
    let mut equipment_gaps = Vec::new();
    let mut storage_gaps = Vec::new();
    let mut capital = input.model.clone();
    capital.seasonal.physical.periods = (0..MONTHS)
        .map(|t| h.periods[source_period(t)].clone())
        .collect();
    let original_starts: Vec<_> = (0..MONTHS)
        .map(|t| input.plan[source_period(t)].clone())
        .collect();
    // Changed packaging starts have no capital outputs, so the original commissioning and retirement dates apply.
    for life in &capital.lives {
        assert!(
            (0..27)
                .chain([33, 42, 43])
                .all(|j| h.processes[j].outputs[life.resource] == 0)
        );
    }
    let retirements = capital.retirements(&original_starts).unwrap();
    let mut capabilities = h.initial.clone();
    let mut completions = vec![vec![0; stock.len()]; MONTHS];

    let labour = &input.model.limits[0];
    assert_eq!(labour.name, "shared_human_work_ticks");
    assert!(
        h.processes
            .iter()
            .all(|p| p.inputs[labour.resource] == 0 && p.outputs[labour.resource] == 0)
    );
    for (t, grain) in plan.iter().enumerate() {
        let period = &h.periods[source_period(t)];
        let reference = &input.plan[source_period(t)];
        let mut starts = reference.activities.clone();
        starts[..27].fill(0);
        for (lane, &j) in MANUAL.iter().enumerate() {
            starts[j] = grain.activities[FIELD_PROCESSES + lane];
        }
        assert!(labour.activity_use[..15].iter().all(|v| *v == 0));
        assert!(tracked.iter().all(|&r| {
            h.processes[..15]
                .iter()
                .all(|p| p.inputs[r] == 0 && p.outputs[r] == 0)
        }));
        for &r in &tracked {
            stock[r] += arrivals[t][r] + period.supplies[r] - reference.deliveries[r];
        }
        for (index, (j, r)) in [(42, 27), (43, 29), (33, 28)].into_iter().enumerate() {
            let allowed = starts[j].min(stock[r] / h.processes[j].inputs[r]);
            curtailed[index] += starts[j] - allowed;
            starts[j] = allowed;
        }
        for (j, (&q, p)) in starts.iter().zip(&h.processes).enumerate() {
            assert!(q >= 0 && q <= period.capacities[j]);
            for &r in &tracked {
                stock[r] -= q * p.inputs[r];
                if let Some(arrival) = arrivals.get_mut(t + p.lead_periods) {
                    arrival[r] += q * p.outputs[r];
                }
            }
        }
        for &r in &tracked {
            assert!(stock[r] <= h.storage_caps[r]);
            minima[r] = minima[r].min(stock[r]);
        }
        assert_eq!(
            (
                period.supplies[labour.resource],
                reference.deliveries[labour.resource],
                reference.discarded[labour.resource]
            ),
            (0, 0, 0)
        );
        for life in &capital.lives {
            capabilities[life.resource] -= retirements[t][life.resource];
        }
        for limit in capital.limits.iter().skip(1) {
            assert!(limit.activity_use[..15].iter().all(|&v| v == 0));
            let used: i64 = starts
                .iter()
                .zip(&limit.activity_use)
                .map(|(q, k)| q * k)
                .sum();
            let capacity = capabilities[limit.resource] * limit.units_per_stock;
            if used > capacity {
                equipment_gaps.push(
                    json!({"month":calendar(t),"limit":limit.name,"missing_units":used-capacity}),
                );
            }
        }
        for pool in &capital.stores {
            let kg: i64 = pool
                .stored_resources
                .iter()
                .map(|&r| grain.closing[r])
                .sum();
            let capacity = capabilities[pool.asset_resource] * pool.units_per_asset;
            if kg > capacity {
                storage_gaps
                    .push(json!({"month":calendar(t),"pool":pool.name,"missing_kg":kg-capacity}));
            }
        }
        for limit in &capital.limits {
            // All reference capability stocks are distinct and sustained by completions.
            let r = limit.resource;
            assert!(h.processes.iter().all(|p| p.inputs[r] == 0));
            assert_eq!((period.supplies[r], reference.deliveries[r]), (0, 0));
        }
        for (j, p) in h.processes.iter().enumerate() {
            if let Some(row) = completions.get_mut(t + p.lead_periods) {
                for (r, quantity) in p.outputs.iter().enumerate() {
                    row[r] += starts[j] * quantity;
                }
            }
        }
        for (capacity, completed) in capabilities.iter_mut().zip(&completions[t]) {
            *capacity += completed;
        }
        let work: i64 = starts
            .iter()
            .zip(&labour.activity_use)
            .map(|(q, k)| q * k)
            .sum();
        let available = h.initial[labour.resource] * labour.units_per_stock;
        schedules.push(starts);
        if work > available {
            work_gaps.push(json!({"month":calendar(t),"missing_manual_work_ticks":work-available}));
        }
    }
    let balances: Vec<_> = tracked
        .iter()
        .map(|&r| json!({"resource":h.resources[r],"minimum":minima[r],"closing":stock[r]}))
        .collect();
    (
        json!({"balances":balances,"curtailed_extraction_recovery_fabrication_starts":curtailed,
        "manual_work_shortfalls":work_gaps,"equipment_shortfalls":equipment_gaps,"grain_storage_shortfalls":storage_gaps}),
        schedules,
    )
}

fn stage_water(model: &Seasonal, plan: &[PeriodPlan]) -> Vec<i64> {
    let mut required = vec![0; plan.len()];
    for crop in &model.crops {
        for &sow in &crop.sowing_periods {
            for (stage, rate) in crop.irrigation_m3_per_activity.iter().enumerate() {
                required[sow + stage] += plan[sow].activities[crop.process] * rate;
            }
        }
    }
    required
}

fn compare(
    input: &ModelInput,
    needs: &[[i64; FOODS]],
    yield_percent: i64,
    fixture: &Value,
) -> Value {
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
    let water = stage_water(&model, &plan);
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
    let (baseline, starts) = capped_packaging(input, &plan);
    let renewed = field_renewal::study(input, &model, &plan, starts, fixture);
    json!({"renewed_schedule":renewed,"gross_yield_percent":yield_percent,"fixed_field_area_ha":land,"required_peak_water_m3":peak,
        "peak_month":calendar(peak_month),"restored_terminal_stocks_kg":dated.final_stock,"expired_kg":dated.expired,
        "seed_consumed_kg":dated.seed_withdrawn,"fixed_upstream_bound":upstream_bound(input,&plan),"finite_packaging_schedule":baseline,
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
    let mut cases = [100, 120, 85].map(|yield_percent| compare(&input, &needs, yield_percent, &v));
    // Each yield is replayed separately. Publish the common downstream balance once.
    let mut packaging = cases.iter_mut().map(|case| {
        case["renewed_schedule"]
            .as_object_mut()
            .unwrap()
            .remove("packaging_2032")
            .unwrap()
    });
    let continuation = packaging.next().unwrap();
    assert!(packaging.all(|other| other == continuation));
    let result = json!({"period_zero":"2026-01","target_months":60,"monthly_targets_kg":needs[12..],"crop_order":["rice_component","oats_component","feed_grain_equivalent"],
        "field_history":"Each fixed field group cycles through fallow, rice or oats, feed grain, then flexible grain. Group zero rests in 2026, group one in 2027, and so on. The same phases are assumed before 2026. No yield, irrigation, or nutrient credit is assigned to dry fallow.",
        "scope":"Grain, seed, dated shelf life, finite grain storage, one-month preservation and one-month dispatch, per-route throughput, sowing windows, land and stage water. A separate prefix bound screens shared energy, wrapping and its finite ore against inherited non-food starts. The renewed schedule replays all physical resources and the original monetary policy on the same calendar.",
        "packaging_after_2031":continuation,"cases":cases});
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
