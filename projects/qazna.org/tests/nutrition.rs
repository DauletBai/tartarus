use qazna::{
    mathematics::{RationalVector, verify_linear},
    nutrition::*,
};
use std::collections::BTreeMap;
fn catalogue() -> Composition {
    Composition {
        quanta_per_unit: QUANTA,
        foods: vec![Food {
            id: "grain".into(),
            sector: "grain".into(),
            cells: vec![Cell {
                nutrient: "protein".into(),
                unit: "g".into(),
                quanta_per_100g: Some(20 * QUANTA),
            }],
        }],
    }
}
fn profile() -> Profile {
    Profile {
        id: "adult".into(),
        scope: "Synthetic test only".into(),
        requirements: vec![Requirement {
            nutrient: "protein".into(),
            unit: "g".into(),
            minimum_quanta: 10 * QUANTA,
            maximum_quanta: Some(30 * QUANTA),
            source: "synthetic".into(),
        }],
    }
}
fn ingredient(grams: u64) -> Ingredient {
    Ingredient {
        food: "grain".into(),
        grams_per_person_day: grams,
        retention: BTreeMap::from([(
            "protein".into(),
            Retention {
                minimum_bps: 10_000,
                maximum_bps: 10_000,
            },
        )]),
    }
}
fn order(id: &str, persons: u64, mode: Preparation, delivery: bool) -> Order {
    Order {
        id: id.into(),
        profile: "adult".into(),
        persons,
        households: persons.div_ceil(2),
        preparation: mode,
        delivery,
        ingredients: vec![ingredient(100)],
    }
}
fn route() -> SupplyRoute {
    SupplyRoute {
        food: "grain".into(),
        edible_yield_bps: 8000,
        usable_after_loss_bps: 9500,
        lead_days: 150,
        inputs: vec![ResourceCoefficient {
            resource: "water".into(),
            unit: "ml".into(),
            units_per_kg_gross: 3000,
        }],
    }
}
fn plan(orders: &[Order]) -> OrderPlan {
    plan_orders(
        &catalogue(),
        &[profile()],
        &[Cohort {
            profile: "adult".into(),
            persons: 100,
        }],
        orders,
        &[route()],
    )
    .unwrap()
}
#[test]
fn independent_exhaustive_integer_oracle_matches_direct_and_matrix_checks() {
    let c = catalogue();
    let p = profile();
    for lo in [0, 2500, 8000, 10_000] {
        for hi in [lo, 10_000] {
            for grams in 1..=200 {
                let mut i = ingredient(grams);
                i.retention.insert(
                    "protein".into(),
                    Retention {
                        minimum_bps: lo,
                        maximum_bps: hi,
                    },
                );
                // Independently scaled: 20 g protein / 100 g ingredient.
                let good = 20 * grams * u64::from(lo) >= 10 * 100 * 10_000
                    && 20 * grams * u64::from(hi) <= 30 * 100 * 10_000;
                let check = assess(&c, &p, &[i.clone()]).unwrap();
                assert_eq!(check.all_supplied_constraints_verified, good);
                let m = constraint_matrix(&c, &p, &[i]).unwrap();
                assert_eq!(
                    verify_linear(
                        &m,
                        &RationalVector {
                            numerators: vec![grams as i64],
                            denominator: 1
                        },
                        None
                    )
                    .is_ok(),
                    good
                );
            }
        }
    }
}
#[test]
fn sub_quantum_shortfall_is_not_rounded_into_success() {
    let mut c = catalogue();
    c.foods[0].cells[0].quanta_per_100g = Some(99);
    let mut p = profile();
    p.requirements[0].minimum_quanta = 1;
    p.requirements[0].maximum_quanta = None;
    let r = assess(&c, &p, &[ingredient(1)]).unwrap();
    assert_eq!(r.nutrients[0].possible_upper_quanta, Some(1));
    assert_eq!(r.nutrients[0].status, "definite_shortfall");
}
#[test]
fn unknown_composition_differs_from_measured_zero() {
    let mut c = catalogue();
    c.foods[0].cells[0].quanta_per_100g = None;
    let r = assess(&c, &profile(), &[ingredient(100)]).unwrap();
    assert_eq!(r.nutrients[0].status, "undetermined");
    assert_eq!(r.nutrients[0].possible_upper_quanta, None);
    assert!(constraint_matrix(&c, &profile(), &[ingredient(100)]).is_err());
    c.foods[0].cells[0].quanta_per_100g = Some(0);
    assert_eq!(
        assess(&c, &profile(), &[ingredient(100)])
            .unwrap()
            .nutrients[0]
            .status,
        "definite_shortfall"
    );
}
#[test]
fn unknown_retention_blocks_consumed_adequacy_and_matrix() {
    let mut i = ingredient(100);
    i.retention.clear();
    let r = assess(&catalogue(), &profile(), &[i.clone()]).unwrap();
    assert!(!r.all_supplied_constraints_verified);
    assert_eq!(r.nutrients[0].known_lower_quanta, 0);
    assert_eq!(r.nutrients[0].possible_upper_quanta, Some(20 * QUANTA));
    assert!(constraint_matrix(&catalogue(), &profile(), &[i]).is_err());
}
#[test]
fn missing_zero_threshold_is_not_a_complete_certificate() {
    let mut p = profile();
    p.requirements[0].minimum_quanta = 0;
    p.requirements[0].maximum_quanta = None;
    let mut c = catalogue();
    c.foods[0].cells[0].quanta_per_100g = None;
    let check = assess(&c, &p, &[ingredient(100)]).unwrap();
    assert_eq!(
        check.nutrients[0].status,
        "bounds_met_with_missing_evidence"
    );
    assert!(!check.all_supplied_constraints_verified);
}
#[test]
fn vitamins_with_different_definitions_are_not_interchangeable() {
    let mut c = catalogue();
    c.foods[0].cells[0].nutrient = "vitamin_a_RAE".into();
    c.foods[0].cells[0].unit = "µg".into();
    let mut p = profile();
    p.requirements[0].nutrient = "vitamin_a_RE".into();
    p.requirements[0].unit = "µg".into();
    let mut i = ingredient(100);
    i.retention.clear();
    let r = assess(&c, &p, &[i]).unwrap();
    assert_eq!(r.nutrients[0].possible_upper_quanta, None);
}
#[test]
fn unit_mismatch_and_duplicate_coefficients_fail_closed() {
    let mut c = catalogue();
    c.foods[0].cells[0].unit = "mg".into();
    assert!(assess(&c, &profile(), &[ingredient(100)]).is_err());
    c = catalogue();
    let duplicate = c.foods[0].cells[0].clone();
    c.foods[0].cells.push(duplicate);
    assert!(c.validate().is_err());
}
#[test]
fn duplicate_ingredients_and_invalid_retention_are_rejected() {
    assert!(assess(&catalogue(), &profile(), &[ingredient(50), ingredient(50)]).is_err());
    for (lo, hi) in [(9000, 8000), (10_001, 10_001)] {
        let mut i = ingredient(100);
        i.retention.insert(
            "protein".into(),
            Retention {
                minimum_bps: lo,
                maximum_bps: hi,
            },
        );
        assert!(assess(&catalogue(), &profile(), &[i]).is_err());
    }
}
#[test]
fn order_mode_does_not_duplicate_agriculture_or_force_delivery() {
    let home = plan(&[order("a", 100, Preparation::Home, false)]);
    let mixed = plan(&[
        order("a", 70, Preparation::Home, false),
        order("b", 30, Preparation::Prepared, true),
    ]);
    assert_eq!(home.food[0].gross_grams, mixed.food[0].gross_grams);
    assert_eq!(home.upstream_inputs, mixed.upstream_inputs);
    assert_eq!(
        (
            mixed.home_person_days,
            mixed.prepared_person_days,
            mixed.delivery_consignments
        ),
        (70, 30, 15)
    );
    assert_eq!(
        (home.prepared_person_days, home.delivery_consignments),
        (0, 0)
    );
}
#[test]
fn household_grocery_delivery_is_independent_of_preparation() {
    let r = plan(&[order("home", 100, Preparation::Home, true)]);
    assert_eq!(r.delivery_consignments, 50);
    assert_eq!(r.prepared_person_days, 0);
}
#[test]
fn absent_orders_remain_unassigned_without_automatic_meals() {
    let r = plan(&[]);
    assert_eq!(r.unassigned_persons, 100);
    assert_eq!(r.population_needs[0].minimum_quanta, 1000 * QUANTA);
    assert_eq!(r.population_needs[0].profiled_persons, 100);
    assert_eq!(r.prepared_person_days, 0);
    assert_eq!(r.delivery_consignments, 0);
    assert!(r.food.is_empty());
    assert!(!r.all_ordered_baskets_verified);
}
#[test]
fn population_or_duplicate_order_cannot_be_overbooked() {
    for orders in [
        vec![order("a", 101, Preparation::Home, false)],
        vec![
            order("a", 50, Preparation::Home, false),
            order("a", 50, Preparation::Home, false),
        ],
    ] {
        assert!(
            plan_orders(
                &catalogue(),
                &[profile()],
                &[Cohort {
                    profile: "adult".into(),
                    persons: 100
                }],
                &orders,
                &[route()]
            )
            .is_err()
        );
    }
}
#[test]
fn one_groups_surplus_cannot_mask_another_groups_shortfall() {
    let mut a = order("low", 50, Preparation::Home, false);
    a.ingredients[0].grams_per_person_day = 10;
    let mut b = order("high", 50, Preparation::Prepared, false);
    b.ingredients[0].grams_per_person_day = 150;
    // Population average 16 g exceeds the 10 g minimum, but the first group gets 2 g.
    let r = plan(&[a, b]);
    assert!(!r.all_ordered_baskets_verified);
    assert_eq!(
        r.assessments[0].check.nutrients[0].status,
        "definite_shortfall"
    );
    assert!(r.assessments[1].check.all_supplied_constraints_verified);
}
#[test]
fn gross_requirement_is_minimal_and_rounding_independent_of_order_partition() {
    for persons in 1..=100 {
        let r = plan(&[order("a", persons, Preparation::Home, false)]);
        let gross = u128::from(r.food[0].gross_grams);
        let need = u128::from(persons) * 100 * 100_000_000;
        assert!(gross * 8000 * 9500 >= need);
        assert!((gross - 1) * 8000 * 9500 < need);
        if persons > 1 {
            let split = plan(&[
                order("a", 1, Preparation::Home, false),
                order("b", persons - 1, Preparation::Prepared, false),
            ]);
            assert_eq!(r.upstream_inputs, split.upstream_inputs);
            assert_eq!(r.food[0].gross_grams, split.food[0].gross_grams);
        }
    }
}
#[test]
fn overflow_does_not_wrap_into_a_feasible_diet_or_supply_plan() {
    let mut c = catalogue();
    c.foods[0].cells[0].quanta_per_100g = Some(u64::MAX);
    assert_eq!(
        assess(&c, &profile(), &[ingredient(u64::MAX)]).unwrap_err(),
        NutritionError::Overflow
    );
    let mut o = order("huge", u64::MAX, Preparation::Home, false);
    o.ingredients[0].grams_per_person_day = u64::MAX;
    assert!(
        plan_orders(
            &catalogue(),
            &[profile()],
            &[Cohort {
                profile: "adult".into(),
                persons: u64::MAX
            }],
            &[o],
            &[route()]
        )
        .is_err()
    );
}
#[test]
fn supply_routes_require_units_and_nonzero_yields() {
    let c = catalogue();
    let p = profile();
    let cohort = Cohort {
        profile: p.id.clone(),
        persons: 100,
    };
    for yield_bps in [0, 10_001] {
        let mut r = route();
        r.edible_yield_bps = yield_bps;
        assert!(
            plan_orders(
                &c,
                std::slice::from_ref(&p),
                std::slice::from_ref(&cohort),
                &[order("a", 100, Preparation::Home, false)],
                &[r]
            )
            .is_err()
        );
    }
}
#[test]
fn imported_reference_preserves_exact_micrograms_and_absent_oil_macros() {
    let c: Composition =
        serde_json::from_str(include_str!("../research/food-composition.json")).unwrap();
    c.validate().unwrap();
    let oil = c.foods.iter().find(|f| f.id == "olive_oil").unwrap();
    assert_eq!(
        oil.cells
            .iter()
            .find(|c| c.nutrient == "protein")
            .unwrap()
            .quanta_per_100g,
        None
    );
    let milk = c.foods.iter().find(|f| f.id == "milk").unwrap();
    let b12 = milk
        .cells
        .iter()
        .find(|c| c.nutrient == "vitamin_b12")
        .unwrap();
    assert_eq!((&*b12.unit, b12.quanta_per_100g), ("µg", Some(540_000)));
}
