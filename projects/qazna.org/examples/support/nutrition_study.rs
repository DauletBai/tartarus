//! Research annex: nutrient constraints and explicitly requested preparation.
use qazna::nutrition::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
const EFSA: &str = "https://www.efsa.europa.eu/sites/default/files/2017_09_DRVs_summary_report.pdf";
fn requirement(
    key: &str,
    unit: &str,
    minimum: u64,
    maximum: Option<u64>,
    source: &str,
) -> Requirement {
    Requirement {
        nutrient: key.into(),
        unit: unit.into(),
        minimum_quanta: minimum,
        maximum_quanta: maximum,
        source: source.into(),
    }
}
fn profile(female: bool) -> Profile {
    let energy = if female { 2000 } else { 2300 };
    let mass = if female { 60 } else { 70 };
    let mut requirements = vec![
        requirement(
            "energy_atwater_general",
            "kcal",
            (energy - 100) * QUANTA,
            Some((energy + 100) * QUANTA),
            "Synthetic activity/energy envelope, not an individual prescription",
        ),
        requirement(
            "protein",
            "g",
            mass * 830_000,
            None,
            &format!("{EFSA} section 4: adult PRI 0.83 g/kg/day; illustrative body mass"),
        ),
        requirement(
            "fat",
            "g",
            energy * 20 * QUANTA / 900,
            Some(energy * 35 * QUANTA / 900),
            &format!("{EFSA} table 2: 20-35% of assumed energy, rounded to quanta"),
        ),
        // By-difference includes fibre: do NOT relabel it as available carbohydrate.
        requirement(
            "carbohydrate_by_difference",
            "g",
            if female { 220 } else { 250 } * QUANTA,
            Some(if female { 320 } else { 370 } * QUANTA),
            "Synthetic basket envelope; not EFSA available-carbohydrate RI",
        ),
        requirement(
            "fibre_total",
            "g",
            25 * QUANTA,
            None,
            &format!("{EFSA} table 2: adult AI"),
        ),
    ];
    for (key, unit, value, kind) in [
        ("calcium", "mg", 950 * QUANTA, "PRI age >=25"),
        (
            "iron",
            "mg",
            if female { 16 } else { 11 } * QUANTA,
            "PRI; premenopausal female profile",
        ),
        (
            "magnesium",
            "mg",
            if female { 300 } else { 350 } * QUANTA,
            "AI",
        ),
        ("phosphorus", "mg", 550 * QUANTA, "AI"),
        ("potassium", "mg", 3500 * QUANTA, "AI"),
        (
            "copper",
            "mg",
            if female { 1_300_000 } else { 1_600_000 },
            "AI",
        ),
        ("selenium", "µg", 70 * QUANTA, "AI"),
        ("iodine", "µg", 150 * QUANTA, "AI"),
        (
            "vitamin_a_RE",
            "µg RE",
            if female { 650 } else { 750 } * QUANTA,
            "PRI; RE, not RAE",
        ),
        (
            "vitamin_c",
            "mg",
            if female { 95 } else { 110 } * QUANTA,
            "PRI",
        ),
        (
            "vitamin_d_D2_D3",
            "µg",
            15 * QUANTA,
            "AI under minimal skin synthesis",
        ),
        (
            "vitamin_e_alpha_tocopherol",
            "mg",
            if female { 11 } else { 13 } * QUANTA,
            "AI",
        ),
        ("vitamin_k_phylloquinone", "µg", 70 * QUANTA, "AI"),
        ("riboflavin", "mg", 1_600_000, "PRI"),
        (
            "vitamin_b6",
            "mg",
            if female { 1_600_000 } else { 1_700_000 },
            "PRI",
        ),
        ("vitamin_b12", "µg", 4 * QUANTA, "AI"),
        (
            "folate_DFE",
            "µg DFE",
            330 * QUANTA,
            "PRI; not total folate mass",
        ),
        ("choline", "mg", 400 * QUANTA, "AI"),
        ("biotin", "µg", 40 * QUANTA, "AI"),
        ("pantothenic_acid", "mg", 5 * QUANTA, "AI"),
    ] {
        requirements.push(requirement(
            key,
            unit,
            value,
            None,
            &format!("{EFSA} tables 5/7/9/11: {kind}"),
        ));
    }
    // Energy-dependent references: 1 kcal = 0.004184 MJ exactly.
    requirements.push(requirement(
        "thiamin",
        "mg",
        energy * 4184 / 10,
        None,
        &format!("{EFSA}: PRI 0.1 mg/MJ, assumed energy"),
    ));
    requirements.push(requirement(
        "niacin_NE",
        "mg NE",
        energy * 4184 * 16 / 10,
        None,
        &format!("{EFSA}: PRI 1.6 mg NE/MJ, not niacin alone"),
    ));
    requirements.push(requirement("sodium","mg",0,Some(2000*QUANTA),"https://www.who.int/en/news-room/fact-sheets/detail/healthy-diet ; adult upper target, not minimum"));
    Profile {
        id: if female {
            "female_30_60kg"
        } else {
            "male_30_70kg"
        }
        .into(),
        scope: format!(
            "Synthetic healthy 30-year-old {}, {mass} kg; not pregnant/lactating; {energy} kcal/day assumed; EFSA 2017 reference snapshot, not comprehensive current clinical norms",
            if female { "female" } else { "male" }
        ),
        requirements,
    }
}
fn basket(c: &Composition, female: bool, raw_inventory: bool) -> Vec<Ingredient> {
    // These are declared choices, NOT an optimized or nutritionally certified menu.
    [
        ("rice", if female { 100 } else { 130 }),
        ("oats", 80),
        ("lentils", 80),
        ("chicken", 70),
        ("beef", 30),
        ("salmon", 70),
        ("carrot", 150),
        ("spinach", 100),
        ("apple", 200),
        ("orange", 200),
        ("milk", 400),
        ("olive_oil", 25),
    ]
    .into_iter()
    .map(|(id, grams)| Ingredient {
        food: id.into(),
        grams_per_person_day: grams,
        retention: if raw_inventory {
            c.foods
                .iter()
                .find(|f| f.id == id)
                .unwrap()
                .cells
                .iter()
                .map(|n| {
                    (
                        n.nutrient.clone(),
                        Retention {
                            minimum_bps: 10_000,
                            maximum_bps: 10_000,
                        },
                    )
                })
                .collect()
        } else {
            BTreeMap::new()
        },
    })
    .collect()
}
fn routes(c: &Composition) -> Vec<SupplyRoute> {
    c.foods
        .iter()
        .map(|f| {
            // Engineering stress coefficients only; not USDA agricultural coefficients.
            let (yield_bps, water_ml, energy_wh, feed_g, lead) = match f.sector.as_str() {
                "grain" => (8000, 1_200_000, 500, 0, 150),
                "pulses" => (9000, 1_500_000, 600, 0, 130),
                "meat" => (6000, 3_000_000, 2000, 3000, 240),
                "fish" => (6500, 500_000, 3000, 1500, 360),
                "dairy" => (9800, 1_000_000, 500, 500, 30),
                "vegetables" => (8500, 300_000, 300, 0, 90),
                "fruit" => (8000, 500_000, 400, 0, 180),
                "oil_crops" => (2000, 1_500_000, 1500, 0, 180),
                "nuts" => (5000, 1_500_000, 800, 0, 300),
                _ => unreachable!(),
            };
            SupplyRoute {
                food: f.id.clone(),
                edible_yield_bps: yield_bps,
                usable_after_loss_bps: 9500,
                lead_days: lead,
                inputs: vec![
                    ResourceCoefficient {
                        resource: "water".into(),
                        unit: "ml".into(),
                        units_per_kg_gross: water_ml,
                    },
                    ResourceCoefficient {
                        resource: "agri_processing_energy".into(),
                        unit: "Wh".into(),
                        units_per_kg_gross: energy_wh,
                    },
                    ResourceCoefficient {
                        resource: "feed_grain_equivalent".into(),
                        unit: "g".into(),
                        units_per_kg_gross: feed_g,
                    },
                ],
            }
        })
        .collect()
}
fn orders(c: &Composition, prepared_percent: u64, delivery: bool) -> Vec<Order> {
    let mut out = Vec::new();
    for female in [false, true] {
        let p = profile(female);
        for mode in [Preparation::Home, Preparation::Prepared] {
            let count = 50_000_000
                * if mode == Preparation::Prepared {
                    prepared_percent
                } else {
                    100 - prepared_percent
                }
                / 100;
            if count == 0 {
                continue;
            }
            out.push(Order {
                id: format!("{}_{mode:?}", p.id),
                profile: p.id.clone(),
                persons: count,
                households: count / 2,
                preparation: mode,
                delivery,
                ingredients: basket(c, female, false),
            });
        }
    }
    out
}
fn service(plan: &OrderPlan) -> Value {
    let home = plan.home_person_days;
    let prepared = plan.prepared_person_days;
    let delivery = plan.delivery_consignments;
    json!({"coefficient_status":"synthetic stress coefficients; household cooking is not free",
        "home_energy_Wh":home*800,"home_water_ml":home*3000,
        "kitchen_energy_Wh":prepared*600,"kitchen_water_ml":prepared*2500,
        "kitchen_person_minutes":prepared*5,"ordered_daily_meal_sets":prepared,
        "delivery_consignments":delivery,"delivery_energy_Wh":delivery*200,
        "capacity_gaps_against_declared_daily_limits":{
            "kitchen_person_days":prepared.saturating_sub(35_000_000),
            "delivery_consignments":delivery.saturating_sub(40_000_000)}})
}
pub fn study() -> Value {
    let source: Value =
        serde_json::from_str(include_str!("../../research/food-composition.json")).unwrap();
    let c: Composition = serde_json::from_value(source.clone()).unwrap();
    let profiles = vec![profile(false), profile(true)];
    let cohorts = profiles
        .iter()
        .map(|p| Cohort {
            profile: p.id.clone(),
            persons: 50_000_000,
        })
        .collect::<Vec<_>>();
    let routes = routes(&c);
    let mut cases = Vec::new();
    for (name, percentage, delivery) in [
        ("all_home_pickup", 0, false),
        ("70_home_30_prepared_pickup", 30, false),
        ("70_home_30_prepared_delivery", 30, true),
        ("all_prepared_delivery", 100, true),
    ] {
        let requested = orders(&c, percentage, delivery);
        let plan = plan_orders(&c, &profiles, &cohorts, &requested, &routes).unwrap();
        assert_eq!(
            plan.home_person_days + plan.prepared_person_days,
            100_000_000
        );
        assert_eq!(plan.unassigned_persons, 0);
        let annual=plan.food.iter().map(|f|json!({"food":f.food,"sector":f.sector,
            "annual_edible_g":f.edible_grams*365,"annual_gross_g":f.gross_grams*365,
            "five_year_gross_g":f.gross_grams*1825,"production_release_days_before_supply":f.lead_days})).collect::<Vec<_>>();
        cases.push(json!({"name":name,"orders":requested,"service":service(&plan),"annual_and_five_year":annual,"plan":plan}));
    }
    for case in &cases[1..] {
        assert_eq!(cases[0]["plan"]["food"], case["plan"]["food"]);
    }
    let no_orders = plan_orders(&c, &profiles, &cohorts, &[], &routes).unwrap();
    let raw_checks = profiles
        .iter()
        .enumerate()
        .map(|(i, p)| assess(&c, p, &basket(&c, i == 1, true)).unwrap())
        .collect::<Vec<_>>();
    json!({"kind":"offline_nutrient_and_order_requirements_annex","source":{
            "composition_url":source["source_url"],"composition_sha256":source["source_sha256"],"norms":EFSA},
        "population":100_000_000,"population_status":"two synthetic adult cohorts; not actual national demography",
        "periods":1825,"model_calls":0,"profiles":profiles,"cohorts":cohorts,"supply_routes":routes,
        "raw_inventory_nutrient_screen":raw_checks,"bounded_menu_search":menu_search(&c),"cases":cases,"no_orders":no_orders,
        "scope":["All displayed gram amounts refer to described ingredients, not served cooked weights",
            "Raw inventory screen assumes no processing solely to identify missing composition and potential nutrient gaps; it is not a diet",
            "Delivery-stage retention is unknown: no diet adequacy certificate is issued",
            "PRI and AI lower screens do not establish safety; comprehensive tolerable upper levels, bioavailability, allergies, essential fatty/amino acids and clinical restrictions remain open",
            "Vitamin A RE and niacin NE are not silently inferred from RAE or niacin; missing is not zero",
            "Selected food choices are not an optimum or a requirement to eat all listed products",
            "Prepared daily meal sets and delivery arise only from explicit orders; home groceries can also be delivered",
            "Annual and five-year figures repeat a constant daily snapshot; no demographic, seasonality or yield forecast is claimed",
            "Agricultural water/energy/feed/yield/lead coefficients are synthetic, not values provided by USDA or EFSA",
            "Production requirements do not prove capacity or availability: feed crop production, seed, land, nutrients, breeding, fisheries limits and initial pipeline are not closed",
            "This annex does not replace or certify the previous abstract-meal monetary graph; no automatic ledger or stock actions"]})
}

fn menu_search(c: &Composition) -> Value {
    use qazna::mathematics::{RationalVector, verify_linear};
    let mut studies = Vec::new();
    for female in [false, true] {
        let full_profile = profile(female);
        let mut macro_profile = full_profile.clone();
        macro_profile.id.push_str("_four_constraint_submodel");
        macro_profile.requirements.truncate(4);
        let mut accepted = Vec::new();
        let mut candidate_requirements = Vec::new();
        let mut tested = 0;
        // Ordered candidate menus are synthetic declared choices, not imposed substitutions.
        for nuts in [20, 30, 40] {
            for oats in [80, 90, 100, 110] {
                tested += 1;
                let mut candidate = basket(c, female, true);
                candidate.retain(|i| i.food != "olive_oil");
                candidate
                    .iter_mut()
                    .find(|i| i.food == "oats")
                    .unwrap()
                    .grams_per_person_day = oats;
                let food = c.foods.iter().find(|f| f.id == "walnuts").unwrap();
                candidate.push(Ingredient {
                    food: food.id.clone(),
                    grams_per_person_day: nuts,
                    retention: food
                        .cells
                        .iter()
                        .map(|n| {
                            (
                                n.nutrient.clone(),
                                Retention {
                                    minimum_bps: 10_000,
                                    maximum_bps: 10_000,
                                },
                            )
                        })
                        .collect(),
                });
                let check = assess(c, &macro_profile, &candidate).unwrap();
                let matrix = constraint_matrix(c, &macro_profile, &candidate).unwrap();
                let primal = RationalVector {
                    numerators: candidate
                        .iter()
                        .map(|i| i.grams_per_person_day as i64)
                        .collect(),
                    denominator: 1,
                };
                let verified = verify_linear(&matrix, &primal, None);
                assert_eq!(verified.is_ok(), check.all_supplied_constraints_verified);
                let requirement_plan = plan_orders(
                    c,
                    std::slice::from_ref(&full_profile),
                    &[Cohort {
                        profile: full_profile.id.clone(),
                        persons: 50_000_000,
                    }],
                    &[Order {
                        id: format!("candidate_{tested}"),
                        profile: full_profile.id.clone(),
                        persons: 50_000_000,
                        households: 25_000_000,
                        preparation: Preparation::Home,
                        delivery: false,
                        ingredients: candidate.clone(),
                    }],
                    &routes(c),
                )
                .unwrap();
                candidate_requirements.push(json!({"candidate_rank":tested,"persons":50_000_000,
                    "four_constraints_feasible":check.all_supplied_constraints_verified,"food":requirement_plan.food,
                    "upstream_inputs":requirement_plan.upstream_inputs,"input_units":requirement_plan.input_units}));
                if check.all_supplied_constraints_verified {
                    accepted.push(json!({"candidate_rank":tested,"ingredients":candidate,"four_constraints":check,"matrix":matrix,
                        "exact_matrix_verification":verified.unwrap(),"full_profile_screen":assess(c,&full_profile,&candidate).unwrap()}));
                }
            }
        }
        studies.push(json!({"profile":full_profile.id,"candidate_count":tested,"candidate_production_requirements":candidate_requirements,"feasible_count_for_four_constraints":accepted.len(),"first_in_declared_preference_order":accepted.first(),"feasible_candidate_ranks":accepted.iter().map(|v|v["candidate_rank"].clone()).collect::<Vec<_>>()}));
    }
    json!({"scope":"24 raw-ingredient candidate menus; exact energy/protein/fat/by-difference carbohydrate constraints only; no cooked adequacy, nutritional safety, allergy clearance or global diet optimum claimed",
        "change":"optional walnuts replace olive oil in this declared candidate family; milk/meat/fish/other ingredients retained; preferences are synthetic",
        "profiles":studies})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidate_search_has_exact_feasible_subset_but_no_full_diet_certificate() {
        let r = study();
        assert_eq!(
            r["bounded_menu_search"]["profiles"][0]["feasible_count_for_four_constraints"],
            4
        );
        assert_eq!(
            r["bounded_menu_search"]["profiles"][1]["feasible_count_for_four_constraints"],
            8
        );
        for p in r["bounded_menu_search"]["profiles"].as_array().unwrap() {
            assert_eq!(
                p["candidate_production_requirements"]
                    .as_array()
                    .unwrap()
                    .len(),
                12
            );
            assert_eq!(
                p["first_in_declared_preference_order"]["full_profile_screen"]["all_supplied_constraints_verified"],
                false
            );
        }
    }
    #[test]
    fn changing_orders_moves_capacity_without_recreating_food_or_concealing_gaps() {
        let r = study();
        let cases = r["cases"].as_array().unwrap();
        for c in cases {
            assert_eq!(c["plan"]["food"], cases[0]["plan"]["food"]);
            assert_eq!(c["plan"]["all_ordered_baskets_verified"], false);
            assert_eq!(c["plan"]["total_persons"], 100_000_000);
        }
        assert_eq!(cases[0]["service"]["home_energy_Wh"], 80_000_000_000u64);
        assert_eq!(cases[1]["service"]["ordered_daily_meal_sets"], 30_000_000);
        assert_eq!(
            cases[3]["service"]["capacity_gaps_against_declared_daily_limits"]["kitchen_person_days"],
            65_000_000
        );
        assert_eq!(
            cases[2]["service"]["capacity_gaps_against_declared_daily_limits"]["delivery_consignments"],
            10_000_000
        );
        assert_eq!(r["no_orders"]["unassigned_persons"], 100_000_000);
    }
}
