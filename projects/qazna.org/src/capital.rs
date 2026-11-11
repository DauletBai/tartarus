//! Offline finite-life capital and constructed storage on a seasonal graph.
use crate::{
    horizon::{PeriodPlan, StockCapacity},
    mathematics::{LinearModel, MathError, Row, Sense},
    seasonal::{Seasonal, SeasonalEvidence},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetLife {
    pub resource: usize,
    /// New completions can operate next period, for this many periods.
    pub service_periods: usize,
    /// Initial stock is available at period zero, expires at this boundary.
    pub initial_remaining_periods: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoragePool {
    pub name: String,
    pub asset_resource: usize,
    pub units_per_asset: i64,
    /// All listed quantities must use the same storage unit.
    pub stored_resources: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapitalCycle {
    pub seasonal: Seasonal,
    pub limits: Vec<StockCapacity>,
    pub lives: Vec<AssetLife>,
    pub stores: Vec<StoragePool>,
}
#[derive(Debug, Serialize)]
pub struct CapitalEvidence {
    pub seasonal: SeasonalEvidence,
    pub retirements_by_period: Vec<Vec<i64>>,
    pub available_opening_stock_by_period: Vec<Vec<i64>>,
}
fn narrow(v: i128) -> Result<i64, MathError> {
    v.try_into().map_err(|_| MathError::Overflow)
}
impl CapitalCycle {
    fn validate(&self) -> Result<(), MathError> {
        let h = &self.seasonal.physical;
        let mut ids = BTreeSet::new();
        if self.lives.is_empty() || self.lives.len() > h.resources.len() {
            return Err(MathError::Invalid("asset life count"));
        }
        for a in &self.lives {
            if a.resource >= h.resources.len()
                || !ids.insert(a.resource)
                || a.service_periods == 0
                || a.service_periods > 10000
                || a.initial_remaining_periods == 0
                || a.initial_remaining_periods > a.service_periods
                || h.processes.iter().any(|p| p.inputs[a.resource] != 0)
                || h.periods
                    .iter()
                    .any(|p| p.supplies[a.resource] != 0 || p.needs[a.resource] != 0)
            {
                return Err(MathError::Invalid(
                    "capital identity, life or unsupported early removal/supply",
                ));
            }
        }
        let mut names = BTreeSet::new();
        let mut store_assets = BTreeSet::new();
        for s in &self.stores {
            let mut resources = BTreeSet::new();
            if s.name.trim().is_empty()
                || !names.insert(&s.name)
                || !ids.contains(&s.asset_resource)
                || !store_assets.insert(s.asset_resource)
                || s.units_per_asset <= 0
                || s.stored_resources.is_empty()
                || s.stored_resources.iter().any(|&r| {
                    r >= h.resources.len() || r == s.asset_resource || !resources.insert(r)
                })
            {
                return Err(MathError::Invalid("constructed storage definition"));
            }
        }
        Ok(())
    }
    // Matrix expression for deterministic expiry at the start of period t.
    fn expiry(&self, t: usize, a: &AssetLife) -> Result<(i64, Vec<(usize, i64)>), MathError> {
        let h = &self.seasonal.physical;
        let r = h.resources.len();
        let width = 3 * r + h.processes.len();
        let initial = if t == a.initial_remaining_periods {
            h.initial[a.resource]
        } else {
            0
        };
        let mut terms = Vec::new();
        for (j, p) in h.processes.iter().enumerate() {
            let delay = p
                .lead_periods
                .checked_add(1)
                .and_then(|n| n.checked_add(a.service_periods))
                .ok_or(MathError::Overflow)?;
            if t >= delay && p.outputs[a.resource] > 0 {
                terms.push(((t - delay) * width + 3 * r + j, p.outputs[a.resource]));
            }
        }
        Ok((initial, terms))
    }
    pub fn compile(&self) -> Result<LinearModel, MathError> {
        let mut model = self.seasonal.compile_with_capabilities(&self.limits)?;
        self.validate()?;
        let h = &self.seasonal.physical;
        let r = h.resources.len();
        let width = 3 * r + h.processes.len();
        for t in 0..h.periods.len() {
            for a in &self.lives {
                let (initial, expiry) = self.expiry(t, a)?;
                let mut terms = vec![(t * width + 2 * r + a.resource, 1)];
                terms.extend(expiry.iter().map(|&(c, v)| (c, -v)));
                model.rows.push(Row {
                    name: format!("retirement:{t}:{}", a.resource),
                    terms,
                    sense: Sense::Equal,
                    rhs: initial,
                });
                for limit in self.limits.iter().filter(|l| l.resource == a.resource) {
                    let row = model
                        .rows
                        .iter_mut()
                        .find(|row| row.name == format!("shared:{}:{t}", limit.name))
                        .ok_or(MathError::Invalid("missing capability row"))?;
                    row.rhs = narrow(
                        i128::from(row.rhs)
                            - i128::from(initial) * i128::from(limit.units_per_stock),
                    )?;
                    for &(c, v) in &expiry {
                        row.terms.push((
                            c,
                            narrow(i128::from(v) * i128::from(limit.units_per_stock))?,
                        ));
                    }
                }
            }
            for s in &self.stores {
                let life = self
                    .lives
                    .iter()
                    .find(|a| a.resource == s.asset_resource)
                    .ok_or(MathError::Invalid("storage life"))?;
                let (initial, expiry) = self.expiry(t, life)?;
                let mut terms: Vec<_> = s
                    .stored_resources
                    .iter()
                    .map(|&k| (t * width + r + k, 1))
                    .collect();
                let opening = if t == 0 {
                    h.initial[s.asset_resource]
                } else {
                    terms.push(((t - 1) * width + r + s.asset_resource, -s.units_per_asset));
                    0
                };
                for (c, v) in expiry {
                    terms.push((c, narrow(i128::from(v) * i128::from(s.units_per_asset))?));
                }
                model.rows.push(Row {
                    name: format!("built_store:{}:{t}", s.name),
                    terms,
                    sense: Sense::AtMost,
                    rhs: narrow(i128::from(opening - initial) * i128::from(s.units_per_asset))?,
                });
            }
        }
        Ok(model)
    }
    /// Independent event scheduling; does not evaluate compiled expiry rows.
    pub fn retirements(&self, plan: &[PeriodPlan]) -> Result<Vec<Vec<i64>>, MathError> {
        self.seasonal.physical.vector(plan)?;
        self.validate()?;
        let h = &self.seasonal.physical;
        let n = plan.len();
        let mut events = vec![vec![0i128; h.resources.len()]; n];
        for a in &self.lives {
            if a.initial_remaining_periods < n {
                events[a.initial_remaining_periods][a.resource] +=
                    i128::from(h.initial[a.resource]);
            }
        }
        for (t, row) in plan.iter().enumerate() {
            if row.activities.iter().any(|&x| x < 0) {
                return Err(MathError::Invalid("negative capital activity"));
            }
            for (j, p) in h.processes.iter().enumerate() {
                let completion = t.checked_add(p.lead_periods).ok_or(MathError::Overflow)?;
                for a in &self.lives {
                    let expires = completion
                        .checked_add(1)
                        .and_then(|v| v.checked_add(a.service_periods))
                        .ok_or(MathError::Overflow)?;
                    if expires < n {
                        let value =
                            i128::from(row.activities[j]) * i128::from(p.outputs[a.resource]);
                        events[expires][a.resource] = events[expires][a.resource]
                            .checked_add(value)
                            .ok_or(MathError::Overflow)?;
                    }
                }
            }
        }
        events
            .into_iter()
            .map(|row| row.into_iter().map(narrow).collect())
            .collect()
    }
    pub fn replay(&self, plan: &[PeriodPlan]) -> Result<CapitalEvidence, MathError> {
        // Validate capacity schemas; feasibility is checked independently below.
        self.seasonal
            .physical
            .compile_capabilities(&self.limits, &[])?;
        let retirements = self.retirements(plan)?;
        let seasonal = self.seasonal.replay(plan)?;
        let h = &self.seasonal.physical;
        let mut active_assets = Vec::new();
        for (t, row) in plan.iter().enumerate() {
            let opening = if t == 0 {
                &h.initial
            } else {
                &plan[t - 1].closing
            };
            let active: Vec<i64> = opening
                .iter()
                .zip(&retirements[t])
                .map(|(&s, &e)| s - e)
                .collect();
            for a in &self.lives {
                if row.discarded[a.resource] != retirements[t][a.resource] || active[a.resource] < 0
                {
                    return Err(MathError::Infeasible(format!(
                        "retirement:{t}:{}",
                        a.resource
                    )));
                }
            }
            for l in &self.limits {
                let used = row.activities.iter().zip(&l.activity_use).try_fold(
                    0i128,
                    |sum, (&x, &v)| {
                        sum.checked_add(i128::from(x) * i128::from(v))
                            .ok_or(MathError::Overflow)
                    },
                )?;
                if used > i128::from(active[l.resource]) * i128::from(l.units_per_stock) {
                    return Err(MathError::Infeasible(format!(
                        "active_capacity:{}:{t}",
                        l.name
                    )));
                }
            }
            for s in &self.stores {
                let used = s.stored_resources.iter().try_fold(0i128, |sum, &k| {
                    sum.checked_add(i128::from(row.closing[k]))
                        .ok_or(MathError::Overflow)
                })?;
                if used > i128::from(active[s.asset_resource]) * i128::from(s.units_per_asset) {
                    return Err(MathError::Infeasible(format!(
                        "active_storage:{}:{t}",
                        s.name
                    )));
                }
            }
            active_assets.push(active);
        }
        Ok(CapitalEvidence {
            seasonal,
            retirements_by_period: retirements,
            available_opening_stock_by_period: active_assets,
        })
    }
}
