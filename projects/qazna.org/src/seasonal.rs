//! Offline seasonal extension: shared land, stage water and mandatory provision.
//! Not a crop-growth forecast, land allocation authority or monetary operation.
use crate::{
    horizon::{Horizon, PeriodPlan, StockCapacity},
    mathematics::{LinearModel, MathError, Row, Sense},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandPool {
    pub name: String,
    pub available_hectares: Vec<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CropUse {
    pub process: usize,
    pub land_pool: usize,
    pub hectares_per_activity: i64,
    /// Absolute periods, not an implicitly repeating calendar.
    pub sowing_periods: Vec<usize>,
    /// One entry per growth period, from sowing until just before harvest.
    pub irrigation_m3_per_activity: Vec<i64>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seasonal {
    pub physical: Horizon,
    pub land: Vec<LandPool>,
    pub crops: Vec<CropUse>,
    /// Withdrawal budget, not total evapotranspiration; unused water is not banked.
    pub irrigation_m3: Vec<i64>,
    pub closing_floors: Vec<Vec<i64>>,
}
#[derive(Debug, Serialize)]
pub struct SeasonalEvidence {
    pub land_hectares_by_period: Vec<Vec<i64>>,
    pub irrigation_m3_by_period: Vec<i64>,
    pub full_declared_provision: bool,
    pub horizon_closed: bool,
}
fn narrow(v: i128) -> Result<i64, MathError> {
    v.try_into().map_err(|_| MathError::Overflow)
}
fn add(v: &mut i128, a: i128) -> Result<(), MathError> {
    *v = v.checked_add(a).ok_or(MathError::Overflow)?;
    Ok(())
}
/// Explicit simplified stage withdrawal: ceil(max(ETc - effective rain, 0) *
/// 10 m3/ha/mm / irrigation efficiency). No soil-water carryover in this helper.
pub fn irrigation_per_hectare(
    etc_mm: u64,
    effective_rain_mm: u64,
    efficiency_bps: u16,
) -> Result<i64, MathError> {
    if efficiency_bps == 0 || efficiency_bps > 10_000 {
        return Err(MathError::Invalid("irrigation efficiency"));
    }
    let n = u128::from(etc_mm.saturating_sub(effective_rain_mm)) * 100_000;
    let d = u128::from(efficiency_bps);
    i64::try_from(n / d + u128::from(!n.is_multiple_of(d))).map_err(|_| MathError::Overflow)
}
impl Seasonal {
    fn validate_extension(&self) -> Result<(), MathError> {
        let h = &self.physical;
        let t = h.periods.len();
        let r = h.resources.len();
        let mut names = BTreeSet::new();
        if self.land.is_empty()
            || self.land.len() > 64
            || self.irrigation_m3.len() != t
            || self.irrigation_m3.iter().any(|&v| v < 0)
            || self.closing_floors.len() != t
            || self.closing_floors.iter().any(|v| {
                v.len() != r
                    || v.iter()
                        .zip(&h.storage_caps)
                        .any(|(&n, &cap)| n < 0 || n > cap)
            })
            || self.land.iter().any(|l| {
                l.name.trim().is_empty()
                    || !names.insert(&l.name)
                    || l.available_hectares.len() != t
                    || l.available_hectares.iter().any(|&v| v < 0)
            })
        {
            return Err(MathError::Invalid("seasonal dimensions or limits"));
        }
        let mut processes = BTreeSet::new();
        let mut scheduled_growth_periods = 0usize;
        for c in &self.crops {
            if c.process >= h.processes.len()
                || c.land_pool >= self.land.len()
                || c.hectares_per_activity <= 0
                || c.basis.trim().is_empty()
                || !processes.insert(c.process)
            {
                return Err(MathError::Invalid("crop identity or land use"));
            }
            let lag = h.processes[c.process].lead_periods;
            scheduled_growth_periods = scheduled_growth_periods
                .checked_add(
                    c.sowing_periods
                        .len()
                        .checked_mul(lag)
                        .ok_or(MathError::Overflow)?,
                )
                .ok_or(MathError::Overflow)?;
            if scheduled_growth_periods > 500_000 {
                return Err(MathError::Invalid("seasonal matrix growth limit"));
            }
            let mut periods = BTreeSet::new();
            if c.irrigation_m3_per_activity.len() != lag
                || c.irrigation_m3_per_activity.iter().any(|&v| v < 0)
                || c.sowing_periods.is_empty()
                || c.sowing_periods
                    .iter()
                    .any(|&s| s >= t || lag >= t - s || !periods.insert(s))
            {
                return Err(MathError::Invalid(
                    "crop calendar, growth stages or terminal harvest",
                ));
            }
        }
        Ok(())
    }
    /// Reuses physical balances, but requires ALL declared needs, not only the
    /// scalar objective resource. Zero objective: feasibility, not preference.
    pub fn compile(&self) -> Result<LinearModel, MathError> {
        let mut model = self.physical.compile()?;
        self.validate_extension()?;
        model.objective.fill(0);
        for row in &mut model.rows {
            if row.name.starts_with("need:") {
                row.sense = Sense::Equal;
            }
        }
        let h = &self.physical;
        let r = h.resources.len();
        let width = 3 * r + h.processes.len();
        for t in 0..h.periods.len() {
            let mut land = vec![Vec::new(); self.land.len()];
            let mut water = Vec::new();
            for c in &self.crops {
                let lag = h.processes[c.process].lead_periods;
                if !c.sowing_periods.contains(&t) {
                    model.rows.push(Row {
                        name: format!("sowing:{t}:{}", c.process),
                        terms: vec![(t * width + 3 * r + c.process, 1)],
                        sense: Sense::AtMost,
                        rhs: 0,
                    });
                }
                for &start in &c.sowing_periods {
                    if start > t || t - start >= lag {
                        continue;
                    }
                    let age = t - start;
                    let column = start * width + 3 * r + c.process;
                    land[c.land_pool].push((column, c.hectares_per_activity));
                    let w = c.irrigation_m3_per_activity[age];
                    if w > 0 {
                        water.push((column, w));
                    }
                }
            }
            for (p, terms) in land.into_iter().enumerate() {
                model.rows.push(Row {
                    name: format!("land:{t}:{p}"),
                    terms,
                    sense: Sense::AtMost,
                    rhs: self.land[p].available_hectares[t],
                });
            }
            model.rows.push(Row {
                name: format!("irrigation:{t}"),
                terms: water,
                sense: Sense::AtMost,
                rhs: self.irrigation_m3[t],
            });
            for k in 0..r {
                model.rows.push(Row {
                    name: format!("floor:{t}:{k}"),
                    terms: vec![(t * width + r + k, -1)],
                    sense: Sense::AtMost,
                    rhs: -self.closing_floors[t][k],
                });
            }
        }
        Ok(model)
    }
    /// Compose seasonal full-needs constraints with shared prior-period assets.
    /// No withdrawals are accepted here: seasonal balances must remain identical.
    pub fn compile_with_capabilities(
        &self,
        limits: &[StockCapacity],
    ) -> Result<LinearModel, MathError> {
        let mut model = self.compile()?;
        let shared = self.physical.compile_capabilities(limits, &[])?;
        model.rows.extend(
            shared
                .rows
                .into_iter()
                .filter(|r| r.name.starts_with("shared:")),
        );
        Ok(model)
    }
    /// Independently checks shared use; a completed machine or qualification is
    /// unavailable until the following period. Capacity is shared across routes.
    pub fn replay_with_capabilities(
        &self,
        plan: &[PeriodPlan],
        limits: &[StockCapacity],
    ) -> Result<SeasonalEvidence, MathError> {
        // Reuse schema validation, not compiled feasibility or row evaluation.
        self.physical.compile_capabilities(limits, &[])?;
        let evidence = self.replay(plan)?;
        for (t, p) in plan.iter().enumerate() {
            for limit in limits {
                let opening = if t == 0 {
                    self.physical.initial[limit.resource]
                } else {
                    plan[t - 1].closing[limit.resource]
                };
                let mut used = 0i128;
                for (&x, &coefficient) in p.activities.iter().zip(&limit.activity_use) {
                    add(&mut used, i128::from(x) * i128::from(coefficient))?;
                }
                if used > i128::from(opening) * i128::from(limit.units_per_stock) {
                    return Err(MathError::Infeasible(format!("shared:{}:{t}", limit.name)));
                }
            }
        }
        Ok(evidence)
    }
    /// Independent event replay. Does not evaluate or trust compiled matrix rows.
    /// Harvest arrives before current starts/deliveries; land is released at harvest.
    pub fn replay(&self, plan: &[PeriodPlan]) -> Result<SeasonalEvidence, MathError> {
        self.physical.vector(plan)?; // validates dimensions, never feasibility
        self.validate_extension()?;
        let h = &self.physical;
        let n = plan.len();
        let r = h.resources.len();
        let mut receipts = vec![vec![0i128; r]; n];
        let mut land = vec![vec![0i128; self.land.len()]; n];
        let mut water = vec![0i128; n];
        let mut stock: Vec<i128> = h.initial.iter().map(|&v| v.into()).collect();
        for (t, p) in plan.iter().enumerate() {
            if p.activities
                .iter()
                .chain(&p.deliveries)
                .chain(&p.closing)
                .chain(&p.discarded)
                .any(|&x| x < 0)
            {
                return Err(MathError::Invalid("negative seasonal plan"));
            }
            for (k, s) in stock.iter_mut().enumerate() {
                add(s, receipts[t][k])?;
                add(s, h.periods[t].supplies[k].into())?;
            }
            for (j, &x) in p.activities.iter().enumerate() {
                if x == 0 {
                    continue;
                }
                let process = &h.processes[j];
                if x > h.periods[t].capacities[j] || process.lead_periods >= n - t {
                    return Err(MathError::Infeasible(format!(
                        "capacity_or_terminal:{t}:{j}"
                    )));
                }
                for (k, s) in stock.iter_mut().enumerate() {
                    add(s, -i128::from(x) * i128::from(process.inputs[k]))?;
                    add(
                        &mut receipts[t + process.lead_periods][k],
                        i128::from(x) * i128::from(process.outputs[k]),
                    )?;
                }
                if let Some(c) = self.crops.iter().find(|c| c.process == j) {
                    if !c.sowing_periods.contains(&t) {
                        return Err(MathError::Infeasible(format!("sowing:{t}:{j}")));
                    }
                    for age in 0..process.lead_periods {
                        add(
                            &mut land[t + age][c.land_pool],
                            i128::from(x) * i128::from(c.hectares_per_activity),
                        )?;
                        add(
                            &mut water[t + age],
                            i128::from(x) * i128::from(c.irrigation_m3_per_activity[age]),
                        )?;
                    }
                }
            }
            for (k, s) in stock.iter_mut().enumerate() {
                add(s, -i128::from(p.deliveries[k]))?;
                add(s, -i128::from(p.discarded[k]))?;
                if p.deliveries[k] != h.periods[t].needs[k]
                    || *s != i128::from(p.closing[k])
                    || p.closing[k] < self.closing_floors[t][k]
                    || p.closing[k] > h.storage_caps[k]
                {
                    return Err(MathError::Infeasible(format!("stock_or_need:{t}:{k}")));
                }
            }
            if water[t] > i128::from(self.irrigation_m3[t])
                || land[t]
                    .iter()
                    .zip(&self.land)
                    .any(|(&used, pool)| used > i128::from(pool.available_hectares[t]))
            {
                return Err(MathError::Infeasible(format!("land_or_water:{t}")));
            }
        }
        Ok(SeasonalEvidence {
            land_hectares_by_period: land
                .into_iter()
                .map(|row| row.into_iter().map(narrow).collect())
                .collect::<Result<_, _>>()?,
            irrigation_m3_by_period: water.into_iter().map(narrow).collect::<Result<_, _>>()?,
            full_declared_provision: true,
            horizon_closed: true,
        })
    }
}
