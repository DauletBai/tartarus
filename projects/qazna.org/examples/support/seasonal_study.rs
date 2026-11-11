//! Five-year cereal/feed subchain derived from the maintained order snapshot.
use qazna::{
    horizon::{Horizon, Period, PeriodPlan, Process},
    mathematics::verify_linear,
    seasonal::*,
};
use serde_json::{Value, json};
const DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const YIELD: [i64; 3] = [4000, 3500, 6000];
const SEED: [i64; 3] = [180, 120, 150];
fn ceil(a: i64, b: i64) -> i64 {
    a / b + i64::from(a % b != 0)
}
fn inputs(nutrition: &Value) -> [i64; 3] {
    let plan = &nutrition["cases"][0]["plan"];
    let food = plan["food"].as_array().unwrap();
    let gross = |key: &str| {
        food.iter().find(|f| f["food"] == key).unwrap()["gross_grams"]
            .as_i64()
            .unwrap()
    };
    assert_eq!(plan["input_units"]["feed_grain_equivalent"], "g");
    [
        gross("rice"),
        gross("oats"),
        plan["upstream_inputs"]["feed_grain_equivalent"]
            .as_i64()
            .unwrap(),
    ]
}
fn model(
    daily_g: [i64; 3],
    reserve_days: i64,
    shock: bool,
    recovery: bool,
) -> (Seasonal, Vec<Vec<i64>>) {
    let demand: Vec<Vec<i64>> = (0..60)
        .map(|t| {
            daily_g
                .iter()
                .map(|&g| ceil(g * DAYS[t % 12], 1000))
                .collect()
        })
        .collect();
    let annual: Vec<i64> = (0..3)
        .map(|k| demand[..12].iter().map(|d| d[k]).sum())
        .collect();
    let area: Vec<i64> = (0..3)
        .map(|k| ceil(annual[k], YIELD[k] - SEED[k]))
        .collect();
    let mut processes = Vec::new();
    let mut crops = Vec::new();
    let mut starts = vec![vec![0; 15]; 60];
    for year in 0..5 {
        for k in 0..3 {
            let j = year * 3 + k;
            let mut feed = vec![0; 3];
            feed[k] = SEED[k];
            let mut output = vec![0; 3];
            output[k] = if shock && year == 2 {
                YIELD[k] * 80 / 100
            } else {
                YIELD[k]
            };
            let lost = area[k] * (YIELD[k] - YIELD[k] * 80 / 100);
            let extra = if recovery && year >= 3 {
                ceil(lost, 2 * (YIELD[k] - SEED[k]))
            } else {
                0
            };
            starts[year * 12 + 3][j] = area[k] + extra;
            processes.push(Process {
                inputs: feed,
                outputs: output,
                lead_periods: 5,
            });
            crops.push(CropUse{process:j,land_pool:0,hectares_per_activity:1,sowing_periods:vec![year*12+3],
            irrigation_m3_per_activity:[40,70,100,90,50].into_iter().map(|etc|irrigation_per_hectare(etc,20,8000).unwrap()).collect(),
            basis:"Synthetic hectare activity, stage ETc/rain/efficiency; not a Kazakhstan crop calibration. Grain is a model stock equivalent, not interchangeable retail foods.".into()});
        }
    }
    let initial: Vec<i64> = (0..3)
        .map(|k| {
            demand[..8].iter().map(|d| d[k]).sum::<i64>()
                + area[k] * SEED[k]
                + ceil(daily_g[k] * reserve_days, 1000)
        })
        .collect();
    let total_area: i64 = area.iter().sum();
    let land_limit = ceil(total_area * 112, 100);
    let water_limit = land_limit * 1000;
    let mut floors = vec![vec![0; 3]; 60];
    floors[59] = initial.clone();
    let h = Horizon {
        resources: vec![
            "rice_gross_equivalent_kg".into(),
            "oats_gross_equivalent_kg".into(),
            "feed_grain_equivalent_kg".into(),
        ],
        initial,
        storage_caps: annual.iter().map(|n| n * 3).collect(),
        processes,
        periods: (0..60)
            .map(|t| Period {
                supplies: vec![0; 3],
                needs: demand[t].clone(),
                capacities: vec![land_limit; 15],
            })
            .collect(),
        objective_resource: 0,
    };
    (
        Seasonal {
            physical: h,
            land: vec![LandPool {
                name: "shared_arable_land".into(),
                available_hectares: vec![land_limit; 60],
            }],
            crops,
            irrigation_m3: vec![water_limit; 60],
            closing_floors: floors,
        },
        starts,
    )
}
fn candidate(m: &Seasonal, starts: &[Vec<i64>]) -> Vec<PeriodPlan> {
    let h = &m.physical;
    let mut stock = h.initial.clone();
    (0..60)
        .map(|t| {
            for (k, value) in stock.iter_mut().enumerate() {
                for (j, p) in h.processes.iter().enumerate() {
                    if t >= p.lead_periods {
                        *value += starts[t - p.lead_periods][j] * p.outputs[k];
                    }
                    *value -= starts[t][j] * p.inputs[k];
                }
                *value -= h.periods[t].needs[k];
            }
            PeriodPlan {
                activities: starts[t].clone(),
                deliveries: h.periods[t].needs.clone(),
                closing: stock.clone(),
                discarded: vec![0; 3],
            }
        })
        .collect()
}
fn case(name: &str, m: Seasonal, starts: Vec<Vec<i64>>) -> Value {
    let p = candidate(&m, &starts);
    let matrix = m.compile().unwrap();
    let linear = verify_linear(&matrix, &m.physical.vector(&p).unwrap(), None);
    let replay = m.replay(&p);
    assert_eq!(linear.is_ok(), replay.is_ok());
    let minimum: Vec<i64> = (0..3)
        .map(|k| p.iter().map(|row| row.closing[k]).min().unwrap())
        .collect();
    let first_shortage = p.iter().position(|row| row.closing.iter().any(|&v| v < 0));
    let terminal_gap: Vec<i64> = (0..3)
        .map(|k| (m.closing_floors[59][k] - p[59].closing[k]).max(0))
        .collect();
    let area_by_year: Vec<Vec<i64>> = (0..5)
        .map(|y| (0..3).map(|k| starts[y * 12 + 3][y * 3 + k]).collect())
        .collect();
    let water_by_month: Vec<i64> = (0..60)
        .map(|t| {
            m.crops
                .iter()
                .map(|c| {
                    (0..5.min(t + 1))
                        .map(|age| starts[t - age][c.process] * c.irrigation_m3_per_activity[age])
                        .sum::<i64>()
                })
                .sum()
        })
        .collect();
    let water_gap = water_by_month
        .iter()
        .zip(&m.irrigation_m3)
        .map(|(&a, &b)| (a - b).max(0))
        .max()
        .unwrap();
    json!({"name":name,"feasible":linear.is_ok(),"matrix_rows":matrix.rows.len(),"matrix_columns":matrix.objective.len(),
        "exact_matrix_result":linear.map_err(|e|e.to_string()),"independent_replay":replay.map_err(|e|e.to_string()),
        "minimum_stock_kg":minimum,"first_negative_stock_month_index":first_shortage,"terminal_stock_gap_kg":terminal_gap,
        "land_hectares_by_year_and_crop":area_by_year,"irrigation_m3_by_month":water_by_month,"maximum_monthly_irrigation_gap_m3":water_gap,
        "model":m,"candidate":p,
        "warning":"Infeasible candidates are diagnostic equations, not executable allocations. No full diet or monetary authority is implied."})
}
pub fn study(nutrition: &Value) -> Value {
    let daily = inputs(nutrition);
    let mut cases = Vec::new();
    for (name, buffer, shock, recovery) in [
        ("baseline_90_day_reserve", 90, false, false),
        ("20_percent_year3_harvest_loss_no_recovery", 90, true, false),
        (
            "20_percent_year3_loss_replenish_in_years4_5",
            90,
            true,
            true,
        ),
        ("no_buffer_year3_loss", 0, true, false),
    ] {
        let (m, s) = model(daily, buffer, shock, recovery);
        cases.push(case(name, m, s));
    }
    let (mut dry, s) = model(daily, 90, false, false);
    dry.irrigation_m3[29] /= 2;
    cases.push(case("year3_peak_irrigation_budget_halved", dry, s));
    let (mut no_feed, s) = model(daily, 90, false, false);
    for p in &mut no_feed.physical.periods {
        p.capacities
            .iter_mut()
            .enumerate()
            .filter(|(j, _)| j % 3 == 2)
            .for_each(|(_, v)| *v = 0);
    }
    cases.push(case("feed_crops_unavailable", no_feed, s));
    json!({"kind":"conditional_five_year_seasonal_cereal_and_feed_subchain","population_from_orders":100_000_000,"model_calls":0,
        "input_daily_grams":[{"resource":"rice_gross_equivalent","g":daily[0]},{"resource":"oats_gross_equivalent","g":daily[1]},{"resource":"feed_grain_equivalent","g":daily[2]}],
        "calendar":"60 months; five repeated 365-day years, leap days excluded explicitly; April sowing, September harvest; synthetic single shared land pool",
        "yield_kg_per_ha":YIELD,"seed_kg_per_ha":SEED,"cases":cases,"preparation_retention":retention_screen(),
        "method_sources":["https://www.fao.org/4/s2022e/s2022e02.htm","https://cropcalendar.apps.fao.org/#/home"],
        "scope":["Only rice/oats gross-equivalent and declared animal feed-grain demand are closed through seasonal crop/seed/land/water balances",
            "The feed demand is derived once from animal-product supply routes; it is not added again as human food",
            "Demand quantities use the unchanged baseline order basket, not the alternative macro-screen candidates",
            "Crop yields, seed rates, land availability, water budgets and calendar are synthetic assumptions; FAO provides methodology, not these coefficients",
            "Irrigation here is an alternative stage-water submodel, NOT an amount to add to the earlier generic water coefficient",
            "Initial stocks and land are declared starting endowments, not goods created by monetary issuance; origins and startup buildup remain unverified",
            "No crop-growth/yield-water feedback, pests, soil balance, crop rotation, labour, fertilizer, fisheries, animal breeding, milling or distribution scheduling is certified",
            "Stock quantities assume suitable preserved inventory without deterioration in this submodel; FEFO/loss/retention integration remains required",
            "Other basket foods and micronutrient adequacy remain outside this subchain; full human freedom of menu/preparation remains in the order model",
            "Terminal floors preserve the starting next-cycle bridge plus reserve; a finite buffer is not a permanent substitute for production",
            "No LP optimum claimed: a supplied candidate is checked by an exact matrix and a separate chronological replay; no runtime or monetary operations"]})
}

fn retention_screen() -> Value {
    use qazna::nutrition::{Composition, Retention, sequential_retention};
    let source: Value =
        serde_json::from_str(include_str!("../../research/food-retention.json")).unwrap();
    let composition: Value =
        serde_json::from_str(include_str!("../../research/food-composition.json")).unwrap();
    let catalogue: Composition = serde_json::from_value(composition.clone()).unwrap();
    let rice = catalogue.foods.iter().find(|f| f.id == "rice").unwrap();
    let thiamin = rice.cells.iter().find(|c| c.nutrient == "thiamin").unwrap();
    assert_eq!(thiamin.unit, "mg");
    let original = thiamin.quanta_per_100g.unwrap();
    let factor = |code: &str| {
        let row = source["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["Retn_Code"] == code && r["Nutr_No"] == "404")
            .unwrap();
        assert_eq!(row["NutrDesc"], "Thiamin");
        let bps = row["Retn_Factor"].as_str().unwrap().parse::<u16>().unwrap() * 100;
        Some(Retention {
            minimum_bps: bps,
            maximum_bps: bps,
        })
    };
    let estimates=[("cooked_drained",vec![factor("431")]),("cooked_water_used",vec![factor("432")]),
        ("cooked_drained_then_reheated",vec![factor("431"),factor("434")]),
        ("unmeasured_storage_then_cooked",vec![None,factor("431")])]
        .into_iter().map(|(name,stages)| {
            let nominal=sequential_retention(&stages).unwrap();
            // 100 g RAW ingredient basis, not 100 g cooked rice. No moisture gain multiplier.
            let retained=nominal.map(|r|json!({"numerator":u128::from(original)*u128::from(r.minimum_bps),"denominator":10_000}));
            json!({"name":name,"source_factors_as_point_assumptions":stages,"composed_nominal_bps":nominal,"retained_thiamin_quanta_per_100g_raw_rational":retained})
        }).collect::<Vec<_>>();
    json!({"source":source["catalogue"],"source_sha256":source["source_sha256"],"composition_source_sha256":composition["source_sha256"],
        "source_nutrient_no":"404","composition_nutrient":"thiamin","unit":"mg","quanta_per_unit":1_000_000,
        "original_quanta_per_100g_raw":original,"estimates":estimates,
        "scope":"USDA preparation point estimates, not measured batch bounds or a complete diet. Exact arithmetic does not remove source uncertainty. Sequential reheating is a conditional model use of both factors; no long-term storage coefficient has been inferred. Cooked mass/concentration requires a separate yield. Folate and vitamin A definitions are not implicitly mapped."})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preparation_estimates_preserve_raw_mass_basis_and_unknown_storage() {
        let r = retention_screen();
        let e = r["estimates"].as_array().unwrap();
        assert_eq!(e[0]["composed_nominal_bps"]["minimum_bps"], 7500);
        assert_eq!(e[1]["composed_nominal_bps"]["minimum_bps"], 8000);
        assert_eq!(e[2]["composed_nominal_bps"]["minimum_bps"], 7125);
        assert!(e[3]["retained_thiamin_quanta_per_100g_raw_rational"].is_null());
        let raw = r["original_quanta_per_100g_raw"].as_u64().unwrap();
        assert_eq!(
            e[0]["retained_thiamin_quanta_per_100g_raw_rational"]["numerator"],
            raw * 7500
        );
        assert_eq!(
            e[0]["retained_thiamin_quanta_per_100g_raw_rational"]["denominator"],
            10000
        );
    }
    #[test]
    fn harvest_loss_can_spare_current_meals_but_exhaust_replenishment() {
        let n = super::super::nutrition_study::study();
        let r = study(&n);
        let c = r["cases"].as_array().unwrap();
        assert_eq!(c[0]["feasible"], true);
        assert_eq!(c[1]["feasible"], false);
        assert!(c[1]["first_negative_stock_month_index"].is_null());
        assert_eq!(c[2]["feasible"], true);
        assert_eq!(c[3]["feasible"], false);
        assert!(c[3]["first_negative_stock_month_index"].is_number());
        assert_eq!(c[4]["feasible"], false);
        assert_eq!(c[5]["feasible"], false);
        for v in c[2]["terminal_stock_gap_kg"].as_array().unwrap() {
            assert_eq!(v, 0);
        }
    }
}
