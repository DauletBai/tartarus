//! Offline nutrition constraints and order-driven food requirements.
//! No medical prescription, stock mutation, issuance or automatic rationing.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const QUANTA: u64 = 1_000_000;
const BPS: u128 = 10_000;
const NUTRIENT_DENOMINATOR: u128 = 100 * BPS;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NutritionError {
    #[error("invalid nutrition input: {0}")]
    Invalid(String),
    #[error("nutrition arithmetic overflow")]
    Overflow,
}
type Result<T> = std::result::Result<T, NutritionError>;
fn invalid(s: &str) -> NutritionError {
    NutritionError::Invalid(s.into())
}
fn add(a: u128, b: u128) -> Result<u128> {
    a.checked_add(b).ok_or(NutritionError::Overflow)
}
fn mul(a: u128, b: u128) -> Result<u128> {
    a.checked_mul(b).ok_or(NutritionError::Overflow)
}
fn narrow(a: u128) -> Result<u64> {
    a.try_into().map_err(|_| NutritionError::Overflow)
}
fn ceil(a: u128, b: u128) -> Result<u64> {
    if b == 0 {
        return Err(invalid("zero denominator"));
    }
    narrow(a / b + u128::from(!a.is_multiple_of(b)))
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cell {
    pub nutrient: String,
    pub unit: String,
    pub quanta_per_100g: Option<u64>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Food {
    pub id: String,
    pub sector: String,
    pub cells: Vec<Cell>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Composition {
    pub quanta_per_unit: u64,
    pub foods: Vec<Food>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Requirement {
    pub nutrient: String,
    pub unit: String,
    pub minimum_quanta: u64,
    pub maximum_quanta: Option<u64>,
    pub source: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    pub id: String,
    /// Explicitly versioned age, life-stage, body mass and activity scope.
    pub scope: String,
    pub requirements: Vec<Requirement>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Retention {
    pub minimum_bps: u16,
    pub maximum_bps: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ingredient {
    pub food: String,
    pub grams_per_person_day: u64,
    /// Missing means unmeasured processing/storage retention, not 100% retention.
    pub retention: BTreeMap<String, Retention>,
}
#[derive(Debug, Serialize)]
pub struct NutrientCheck {
    pub nutrient: String,
    pub unit: String,
    pub known_lower_quanta: u64,
    pub possible_upper_quanta: Option<u64>,
    pub minimum_quanta: u64,
    pub maximum_quanta: Option<u64>,
    pub missing_evidence: Vec<String>,
    pub status: String,
}
#[derive(Debug, Serialize)]
pub struct BasketCheck {
    pub profile: String,
    /// Scoped only to supplied requirements, never a certification of a complete diet.
    pub all_supplied_constraints_verified: bool,
    pub nutrients: Vec<NutrientCheck>,
}
impl Composition {
    pub fn validate(&self) -> Result<()> {
        if self.quanta_per_unit != QUANTA || self.foods.is_empty() {
            return Err(invalid("composition scale or empty catalogue"));
        }
        let mut ids = BTreeSet::new();
        let mut units = BTreeMap::new();
        for f in &self.foods {
            if f.id.is_empty() || f.sector.is_empty() || !ids.insert(&f.id) {
                return Err(invalid("duplicate or empty food"));
            }
            let mut keys = BTreeSet::new();
            for c in &f.cells {
                if c.nutrient.is_empty() || c.unit.is_empty() || !keys.insert(&c.nutrient) {
                    return Err(invalid("duplicate or empty nutrient"));
                }
                if units
                    .insert(&c.nutrient, &c.unit)
                    .is_some_and(|u| u != &c.unit)
                {
                    return Err(invalid("inconsistent nutrient unit"));
                }
            }
        }
        Ok(())
    }
    fn food(&self, id: &str) -> Result<&Food> {
        self.foods
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| invalid("unknown food"))
    }
}
fn validate_basket<'a>(catalogue: &'a Composition, basket: &[Ingredient]) -> Result<Vec<&'a Food>> {
    let mut ids = BTreeSet::new();
    basket
        .iter()
        .map(|line| {
            if line.grams_per_person_day == 0 || !ids.insert(&line.food) {
                return Err(invalid("zero or duplicate ingredient"));
            }
            let f = catalogue.food(&line.food)?;
            for (key, r) in &line.retention {
                if !f.cells.iter().any(|c| &c.nutrient == key)
                    || r.minimum_bps > r.maximum_bps
                    || r.maximum_bps > 10_000
                {
                    return Err(invalid("invalid retention"));
                }
            }
            Ok(f)
        })
        .collect()
}
/// Assessment is per person/profile BEFORE aggregation. Excess in one group cannot
/// compensate for a shortfall in another. Unknowns never silently become zero.
pub fn assess(
    catalogue: &Composition,
    profile: &Profile,
    basket: &[Ingredient],
) -> Result<BasketCheck> {
    catalogue.validate()?;
    if profile.id.is_empty() || profile.scope.is_empty() || profile.requirements.is_empty() {
        return Err(invalid("empty profile"));
    }
    let foods = validate_basket(catalogue, basket)?;
    let mut keys = BTreeSet::new();
    let mut result = Vec::new();
    for r in &profile.requirements {
        if r.source.is_empty()
            || r.unit.is_empty()
            || !keys.insert(&r.nutrient)
            || r.maximum_quanta.is_some_and(|m| m < r.minimum_quanta)
        {
            return Err(invalid("invalid requirement"));
        }
        let (mut low, mut high) = (0u128, Some(0u128));
        let mut missing = Vec::new();
        for (line, f) in basket.iter().zip(&foods) {
            let c = f.cells.iter().find(|c| c.nutrient == r.nutrient);
            if c.is_some_and(|c| c.unit != r.unit) {
                return Err(invalid("requirement/composition unit mismatch"));
            }
            let Some(q) = c.and_then(|c| c.quanta_per_100g) else {
                missing.push(format!("{}:composition", f.id));
                high = None;
                continue;
            };
            let retention = line.retention.get(&r.nutrient);
            if retention.is_none() {
                missing.push(format!("{}:retention", f.id));
            }
            let lo = retention.map_or(0, |t| t.minimum_bps);
            let hi = retention.map_or(10_000, |t| t.maximum_bps);
            let base = mul(u128::from(q), u128::from(line.grams_per_person_day))?;
            low = add(low, mul(base, u128::from(lo))?)?;
            if let Some(h) = high {
                high = Some(add(h, mul(base, u128::from(hi))?)?);
            }
        }
        // Compare unrounded rational quantities; round outward only for display.
        let min = mul(u128::from(r.minimum_quanta), NUTRIENT_DENOMINATOR)?;
        let max = r
            .maximum_quanta
            .map(|v| mul(u128::from(v), NUTRIENT_DENOMINATOR))
            .transpose()?;
        let status = if max.is_some_and(|m| low > m) {
            "definite_excess"
        } else if high.is_some_and(|h| h < min) {
            "definite_shortfall"
        } else if low >= min && max.is_none_or(|m| high.is_some_and(|h| h <= m)) {
            if missing.is_empty() {
                "verified_for_supplied_constraint"
            } else {
                "bounds_met_with_missing_evidence"
            }
        } else {
            "undetermined"
        };
        result.push(NutrientCheck {
            nutrient: r.nutrient.clone(),
            unit: r.unit.clone(),
            known_lower_quanta: narrow(low / NUTRIENT_DENOMINATOR)?,
            possible_upper_quanta: high.map(|h| ceil(h, NUTRIENT_DENOMINATOR)).transpose()?,
            minimum_quanta: r.minimum_quanta,
            maximum_quanta: r.maximum_quanta,
            missing_evidence: missing,
            status: status.into(),
        });
    }
    Ok(BasketCheck {
        profile: profile.id.clone(),
        all_supplied_constraints_verified: result
            .iter()
            .all(|r| r.status == "verified_for_supplied_constraint"),
        nutrients: result,
    })
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Preparation {
    Home,
    Prepared,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Order {
    pub id: String,
    pub profile: String,
    pub persons: u64,
    pub households: u64,
    pub preparation: Preparation,
    pub delivery: bool,
    pub ingredients: Vec<Ingredient>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Cohort {
    pub profile: String,
    pub persons: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResourceCoefficient {
    pub resource: String,
    pub unit: String,
    pub units_per_kg_gross: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SupplyRoute {
    pub food: String,
    pub edible_yield_bps: u16,
    pub usable_after_loss_bps: u16,
    pub lead_days: u32,
    pub inputs: Vec<ResourceCoefficient>,
}
#[derive(Debug, Serialize)]
pub struct FoodDemand {
    pub food: String,
    pub sector: String,
    pub edible_grams: u64,
    pub gross_grams: u64,
    pub lead_days: u32,
}
#[derive(Debug, Serialize)]
pub struct OrderAssessment {
    pub order: String,
    pub persons: u64,
    pub check: BasketCheck,
}
#[derive(Debug, Serialize)]
pub struct PopulationNeed {
    pub nutrient: String,
    pub unit: String,
    pub profiled_persons: u64,
    /// Aggregate planning floor, never proof of equitable individual provision.
    pub minimum_quanta: u64,
}
#[derive(Debug, Serialize)]
pub struct OrderPlan {
    pub total_persons: u64,
    pub population_needs: Vec<PopulationNeed>,
    pub unassigned_persons: u64,
    pub home_person_days: u64,
    pub prepared_person_days: u64,
    pub delivery_consignments: u64,
    pub food: Vec<FoodDemand>,
    pub upstream_inputs: BTreeMap<String, u64>,
    pub input_units: BTreeMap<String, String>,
    pub assessments: Vec<OrderAssessment>,
    pub all_ordered_baskets_verified: bool,
}
/// One explicit daily snapshot. No order implies no kitchen/delivery allocation.
/// Cohorts must be disjoint; privacy-preserving population counts are supplied by
/// the caller. This pure calculation does not certify identity or actual receipt.
pub fn plan_orders(
    catalogue: &Composition,
    profiles: &[Profile],
    cohorts: &[Cohort],
    orders: &[Order],
    routes: &[SupplyRoute],
) -> Result<OrderPlan> {
    catalogue.validate()?;
    let mut profile_ids = BTreeSet::new();
    for p in profiles {
        if !profile_ids.insert(&p.id) {
            return Err(invalid("duplicate profile"));
        }
        assess(catalogue, p, &[])?;
    }
    let mut remaining = BTreeMap::new();
    let mut total = 0u128;
    for c in cohorts {
        if !profile_ids.contains(&c.profile)
            || c.persons == 0
            || remaining.insert(c.profile.clone(), c.persons).is_some()
        {
            return Err(invalid("invalid cohort"));
        }
        total = add(total, u128::from(c.persons))?;
    }
    // Needs exist independently of orders. Missing orders never erase the
    // agricultural planning target; they leave the food mix unresolved.
    let mut needs = BTreeMap::<String, (String, u128, u128)>::new();
    for c in cohorts {
        let p = profiles
            .iter()
            .find(|p| p.id == c.profile)
            .ok_or_else(|| invalid("unknown cohort profile"))?;
        for r in &p.requirements {
            let e = needs
                .entry(r.nutrient.clone())
                .or_insert((r.unit.clone(), 0, 0));
            if e.0 != r.unit {
                return Err(invalid("inconsistent population nutrient unit"));
            }
            e.1 = add(e.1, u128::from(c.persons))?;
            e.2 = add(
                e.2,
                mul(u128::from(c.persons), u128::from(r.minimum_quanta))?,
            )?;
        }
    }
    let population_needs = needs
        .into_iter()
        .map(|(nutrient, (unit, persons, q))| {
            Ok(PopulationNeed {
                nutrient,
                unit,
                profiled_persons: narrow(persons)?,
                minimum_quanta: narrow(q)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut route_ids = BTreeSet::new();
    let mut units = BTreeMap::new();
    for r in routes {
        catalogue.food(&r.food)?;
        if !route_ids.insert(&r.food)
            || r.edible_yield_bps == 0
            || r.edible_yield_bps > 10_000
            || r.usable_after_loss_bps == 0
            || r.usable_after_loss_bps > 10_000
        {
            return Err(invalid("invalid supply yield"));
        }
        let mut resources = BTreeSet::new();
        for i in &r.inputs {
            if i.resource.is_empty()
                || i.unit.is_empty()
                || !resources.insert(&i.resource)
                || units
                    .insert(i.resource.clone(), i.unit.clone())
                    .is_some_and(|u| u != i.unit)
            {
                return Err(invalid("invalid supply input unit"));
            }
        }
    }
    let (mut home, mut prepared, mut deliveries) = (0u128, 0u128, 0u128);
    let mut ids = BTreeSet::new();
    let mut edible = BTreeMap::<String, u128>::new();
    let mut assessments = Vec::new();
    for o in orders {
        if o.id.is_empty()
            || !ids.insert(&o.id)
            || o.persons == 0
            || o.households == 0
            || o.households > o.persons
            || o.ingredients.is_empty()
        {
            return Err(invalid("invalid order"));
        }
        let rem = remaining
            .get_mut(&o.profile)
            .ok_or_else(|| invalid("unknown order cohort"))?;
        *rem = rem
            .checked_sub(o.persons)
            .ok_or_else(|| invalid("overbooked cohort"))?;
        let p = profiles
            .iter()
            .find(|p| p.id == o.profile)
            .ok_or_else(|| invalid("unknown order profile"))?;
        assessments.push(OrderAssessment {
            order: o.id.clone(),
            persons: o.persons,
            check: assess(catalogue, p, &o.ingredients)?,
        });
        match o.preparation {
            Preparation::Home => home = add(home, u128::from(o.persons))?,
            Preparation::Prepared => prepared = add(prepared, u128::from(o.persons))?,
        }
        if o.delivery {
            deliveries = add(deliveries, u128::from(o.households))?;
        }
        for i in &o.ingredients {
            let v = edible.entry(i.food.clone()).or_default();
            *v = add(
                *v,
                mul(u128::from(i.grams_per_person_day), u128::from(o.persons))?,
            )?;
        }
    }
    let mut food = Vec::new();
    let mut inputs = BTreeMap::<String, u128>::new();
    for (id, g) in edible {
        let r = routes
            .iter()
            .find(|r| r.food == id)
            .ok_or_else(|| invalid("missing production route"))?;
        let gross = ceil(
            mul(g, BPS * BPS)?,
            u128::from(r.edible_yield_bps) * u128::from(r.usable_after_loss_bps),
        )?;
        for i in &r.inputs {
            let v = inputs.entry(i.resource.clone()).or_default();
            // Aggregate before rounding so splitting an order cannot multiply input use.
            *v = add(
                *v,
                mul(u128::from(gross), u128::from(i.units_per_kg_gross))?,
            )?;
        }
        food.push(FoodDemand {
            food: id.clone(),
            sector: catalogue.food(&id)?.sector.clone(),
            edible_grams: narrow(g)?,
            gross_grams: gross,
            lead_days: r.lead_days,
        });
    }
    let all_ordered_baskets_verified = !assessments.is_empty()
        && assessments
            .iter()
            .all(|a| a.check.all_supplied_constraints_verified);
    Ok(OrderPlan {
        total_persons: narrow(total)?,
        population_needs,
        unassigned_persons: narrow(remaining.values().map(|v| u128::from(*v)).sum())?,
        home_person_days: narrow(home)?,
        prepared_person_days: narrow(prepared)?,
        delivery_consignments: narrow(deliveries)?,
        food,
        upstream_inputs: inputs
            .into_iter()
            .map(|(k, v)| Ok((k, ceil(v, 1000)?)))
            .collect::<Result<_>>()?,
        input_units: units,
        assessments,
        all_ordered_baskets_verified,
    })
}

/// Compile robust nutrient inequalities in grams into the existing exact LP
/// verifier. Missing coefficients/retention block compilation, never become zero.
/// The zero objective is feasibility only, not a least-cost or optimal diet.
pub fn constraint_matrix(
    catalogue: &Composition,
    profile: &Profile,
    options: &[Ingredient],
) -> Result<crate::mathematics::LinearModel> {
    use crate::mathematics::{LinearModel, Row, Sense};
    assess(catalogue, profile, options)?;
    let mut rows = Vec::new();
    let to_i64 = |v: u128| -> Result<i64> { v.try_into().map_err(|_| NutritionError::Overflow) };
    for req in &profile.requirements {
        let mut lower = Vec::new();
        let mut upper = Vec::new();
        for (j, line) in options.iter().enumerate() {
            let cell = catalogue
                .food(&line.food)?
                .cells
                .iter()
                .find(|c| c.nutrient == req.nutrient)
                .ok_or_else(|| invalid("incomplete nutrient matrix"))?;
            let q = cell
                .quanta_per_100g
                .ok_or_else(|| invalid("incomplete nutrient matrix"))?;
            let r = line
                .retention
                .get(&req.nutrient)
                .ok_or_else(|| invalid("unknown processing retention"))?;
            let a = to_i64(mul(u128::from(q), u128::from(r.minimum_bps))?)?;
            let b = to_i64(mul(u128::from(q), u128::from(r.maximum_bps))?)?;
            if a != 0 {
                lower.push((j, -a));
            }
            if b != 0 {
                upper.push((j, b));
            }
        }
        rows.push(Row {
            name: format!("{}_minimum", req.nutrient),
            terms: lower,
            sense: Sense::AtMost,
            rhs: -to_i64(mul(u128::from(req.minimum_quanta), NUTRIENT_DENOMINATOR)?)?,
        });
        if let Some(max) = req.maximum_quanta {
            rows.push(Row {
                name: format!("{}_maximum", req.nutrient),
                terms: upper,
                sense: Sense::AtMost,
                rhs: to_i64(mul(u128::from(max), NUTRIENT_DENOMINATOR)?)?,
            });
        }
    }
    Ok(LinearModel {
        objective: vec![0; options.len()],
        rows,
    })
}
