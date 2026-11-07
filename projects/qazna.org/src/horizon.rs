//! Time-expanded physical resource model. Integer quantities, explicit delays,
//! production inputs, capacity, storage, deliveries and discarded quantities.
use crate::mathematics::{LinearModel, MathError, RationalVector, Row, Sense};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Process {
    pub inputs: Vec<i64>,
    pub outputs: Vec<i64>,
    pub lead_periods: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Period {
    pub supplies: Vec<i64>,
    pub needs: Vec<i64>,
    pub capacities: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Horizon {
    pub resources: Vec<String>,
    pub initial: Vec<i64>,
    pub storage_caps: Vec<i64>,
    pub processes: Vec<Process>,
    pub periods: Vec<Period>,
    /// Maximize delivery of this resource only; do not sum incompatible units.
    pub objective_resource: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeriodPlan {
    pub activities: Vec<i64>,
    pub deliveries: Vec<i64>,
    pub closing: Vec<i64>,
    pub discarded: Vec<i64>,
}
impl Horizon {
    fn validate(&self) -> Result<(), MathError> {
        let r = self.resources.len();
        if r == 0
            || r > 64
            || self.processes.len() > 128
            || self.periods.len() > 10_000
            || self
                .periods
                .len()
                .checked_mul(3 * r + self.processes.len())
                .is_none_or(|n| n > 250_000)
            || self.resources.iter().any(|name| name.is_empty())
            || self.periods.is_empty()
            || self.objective_resource >= r
            || self.initial.len() != r
            || self.storage_caps.len() != r
            || self
                .initial
                .iter()
                .chain(&self.storage_caps)
                .any(|&x| x < 0)
            || self
                .resources
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != r
        {
            return Err(MathError::Invalid("horizon dimensions or initial stock"));
        }
        for p in &self.processes {
            if p.inputs.len() != r
                || p.outputs.len() != r
                || p.lead_periods == 0
                || p.inputs.iter().chain(&p.outputs).any(|&x| x < 0)
                || !p.inputs.iter().any(|&x| x > 0)
                || !p.outputs.iter().any(|&x| x > 0)
            {
                return Err(MathError::Invalid(
                    "process dimensions, coefficients or delay",
                ));
            }
        }
        for t in &self.periods {
            if t.supplies.len() != r
                || t.needs.len() != r
                || t.capacities.len() != self.processes.len()
                || t.supplies
                    .iter()
                    .chain(&t.needs)
                    .chain(&t.capacities)
                    .any(|&x| x < 0)
            {
                return Err(MathError::Invalid("period dimensions or negative data"));
            }
        }
        Ok(())
    }
    /// Per-period columns: deliveries[R], closing[R], discarded[R], starts[J].
    pub fn compile(&self) -> Result<LinearModel, MathError> {
        self.validate()?;
        let r = self.resources.len();
        let width = 3 * r + self.processes.len();
        let mut objective = vec![0; self.periods.len() * width];
        let mut rows = Vec::new();
        for (t, period) in self.periods.iter().enumerate() {
            let base = t * width;
            objective[base + self.objective_resource] = 1;
            for k in 0..r {
                let mut terms = vec![(base + k, 1), (base + r + k, 1), (base + 2 * r + k, 1)];
                if t > 0 {
                    terms.push(((t - 1) * width + r + k, -1));
                }
                for (j, p) in self.processes.iter().enumerate() {
                    if p.inputs[k] > 0 {
                        terms.push((base + 3 * r + j, p.inputs[k]));
                    }
                    if t >= p.lead_periods && p.outputs[k] > 0 {
                        terms.push(((t - p.lead_periods) * width + 3 * r + j, -p.outputs[k]));
                    }
                }
                let rhs = period.supplies[k]
                    .checked_add(if t == 0 { self.initial[k] } else { 0 })
                    .ok_or(MathError::Overflow)?;
                rows.push(Row {
                    name: format!("balance:{t}:{k}"),
                    terms,
                    sense: Sense::Equal,
                    rhs,
                });
                rows.push(Row {
                    name: format!("need:{t}:{k}"),
                    terms: vec![(base + k, 1)],
                    sense: Sense::AtMost,
                    rhs: period.needs[k],
                });
                rows.push(Row {
                    name: format!("storage:{t}:{k}"),
                    terms: vec![(base + r + k, 1)],
                    sense: Sense::AtMost,
                    rhs: self.storage_caps[k],
                });
            }
            for (j, p) in self.processes.iter().enumerate() {
                // Close the horizon: no unsupported promise outside the modelled period.
                let finishes_inside = p.lead_periods < self.periods.len() - t;
                rows.push(Row {
                    name: format!("capacity:{t}:{j}"),
                    terms: vec![(base + 3 * r + j, 1)],
                    sense: Sense::AtMost,
                    rhs: if finishes_inside {
                        period.capacities[j]
                    } else {
                        0
                    },
                });
            }
        }
        Ok(LinearModel { objective, rows })
    }
    pub fn vector(&self, plan: &[PeriodPlan]) -> Result<RationalVector, MathError> {
        self.validate()?;
        let r = self.resources.len();
        if plan.len() != self.periods.len() {
            return Err(MathError::Invalid("plan horizon"));
        }
        let mut numerators = Vec::new();
        for p in plan {
            if p.deliveries.len() != r
                || p.closing.len() != r
                || p.discarded.len() != r
                || p.activities.len() != self.processes.len()
            {
                return Err(MathError::Invalid("plan dimensions"));
            }
            numerators.extend_from_slice(&p.deliveries);
            numerators.extend_from_slice(&p.closing);
            numerators.extend_from_slice(&p.discarded);
            numerators.extend_from_slice(&p.activities);
        }
        Ok(RationalVector {
            numerators,
            denominator: 1,
        })
    }
    /// Demand itself is an upper bound. This certificate works ONLY when every
    /// objective-resource need is delivered; otherwise verification rejects it.
    pub fn full_coverage_dual(&self, model: &LinearModel) -> RationalVector {
        RationalVector {
            numerators: model
                .rows
                .iter()
                .map(|r| {
                    i64::from(
                        r.name.starts_with("need:")
                            && r.name.ends_with(&format!(":{}", self.objective_resource)),
                    )
                })
                .collect(),
            denominator: 1,
        }
    }
}

/// A shared use constraint. Stock is retained, not consumed like an ingredient.
/// Skilled capacity represents available qualified work capacity, not ownership
/// of people. Capacity earned by completion becomes usable next period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StockCapacity {
    pub name: String,
    pub resource: usize,
    pub units_per_stock: i64,
    pub activity_use: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Withdrawal {
    pub period: usize,
    pub resource: usize,
    pub quantity: i64,
    pub basis: String,
}
impl Horizon {
    /// Add shared equipment, buildings and qualified-capacity limits to the
    /// same physical matrix. Withdrawals are explicit losses/exits, never sales.
    pub fn compile_capabilities(
        &self,
        limits: &[StockCapacity],
        exits: &[Withdrawal],
    ) -> Result<LinearModel, MathError> {
        let mut model = self.compile()?;
        let r = self.resources.len();
        let j = self.processes.len();
        let width = 3 * r + j;
        let mut withdrawals = std::collections::BTreeMap::new();
        for e in exits {
            if e.period >= self.periods.len()
                || e.resource >= r
                || e.quantity <= 0
                || e.basis.trim().is_empty()
                || withdrawals
                    .insert((e.period, e.resource), e.quantity)
                    .is_some()
            {
                return Err(MathError::Invalid("invalid or duplicate withdrawal"));
            }
            if e.period == 0 && self.initial[e.resource] < e.quantity {
                return Err(MathError::Invalid("withdrawal exceeds opening stock"));
            }
            // Original compiler emits 3 resource rows then J capacity rows per period.
            let row = &mut model.rows[e.period * (3 * r + j) + 3 * e.resource];
            row.rhs = row.rhs.checked_sub(e.quantity).ok_or(MathError::Overflow)?;
            if e.period > 0 {
                model.rows.push(Row {
                    name: format!("exit:{}:{}", e.period, e.resource),
                    terms: vec![((e.period - 1) * width + r + e.resource, -1)],
                    sense: Sense::AtMost,
                    rhs: -e.quantity,
                });
            }
        }
        let mut names = std::collections::BTreeSet::new();
        for limit in limits {
            if limit.name.trim().is_empty()
                || !names.insert(&limit.name)
                || limit.resource >= r
                || limit.units_per_stock <= 0
                || limit.activity_use.len() != j
                || limit.activity_use.iter().any(|&a| a < 0)
            {
                return Err(MathError::Invalid("invalid shared stock capacity"));
            }
            for t in 0..self.periods.len() {
                let mut terms: Vec<_> = limit
                    .activity_use
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| **a > 0)
                    .map(|(p, &a)| (t * width + 3 * r + p, a))
                    .collect();
                let removed = withdrawals.get(&(t, limit.resource)).copied().unwrap_or(0);
                let rhs = if t == 0 {
                    (self.initial[limit.resource] - removed)
                        .checked_mul(limit.units_per_stock)
                        .ok_or(MathError::Overflow)?
                } else {
                    terms.push(((t - 1) * width + r + limit.resource, -limit.units_per_stock));
                    (-removed)
                        .checked_mul(limit.units_per_stock)
                        .ok_or(MathError::Overflow)?
                };
                model.rows.push(Row {
                    name: format!("shared:{}:{t}", limit.name),
                    terms,
                    sense: Sense::AtMost,
                    rhs,
                });
            }
        }
        Ok(model)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PaymentSource {
    Input { process: usize, resource: usize },
    Provision { resource: usize },
    Capital { process: usize, resource: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CashLink {
    pub period: usize,
    pub source: PaymentSource,
    pub quantity: i64,
    /// Canonical integer string: exact normative measure, never a float.
    pub amount: String,
}
impl Horizon {
    /// Check completeness and uniqueness of goods/payment links against a plan.
    /// This does not establish physical feasibility, liquidity, ownership or the
    /// legitimacy of norms; those require their separate checks.
    pub fn verify_cash_links(
        &self,
        plan: &[PeriodPlan],
        unit_measure: &[i64],
        capital_resources: &[usize],
        links: &[CashLink],
    ) -> Result<(), MathError> {
        self.vector(plan)?;
        let r = self.resources.len();
        let capital: std::collections::BTreeSet<_> = capital_resources.iter().copied().collect();
        if unit_measure.len() != r
            || unit_measure.iter().any(|&v| v < 0)
            || capital.len() != capital_resources.len()
            || capital.iter().any(|&k| k >= r)
            || links.len() > 1_000_000
        {
            return Err(MathError::Invalid(
                "invalid monetary link dimensions or norms",
            ));
        }
        let mut expected = std::collections::BTreeMap::new();
        let mut record =
            |t: usize, source: PaymentSource, q: i64, k: usize| -> Result<(), MathError> {
                if q < 0 {
                    return Err(MathError::Invalid("negative physical quantity"));
                }
                if q > 0 && unit_measure[k] > 0 {
                    expected.insert(
                        (t, source),
                        (q, (i128::from(q) * i128::from(unit_measure[k])).to_string()),
                    );
                }
                Ok(())
            };
        for (t, p) in plan.iter().enumerate() {
            if p.activities.iter().any(|&n| n < 0) {
                return Err(MathError::Invalid("negative activity"));
            }
            for k in 0..r {
                if capital.contains(&k) && p.deliveries[k] != 0 {
                    return Err(MathError::Invalid(
                        "capital procurement cannot also be billed as provision",
                    ));
                }
                record(
                    t,
                    PaymentSource::Provision { resource: k },
                    p.deliveries[k],
                    k,
                )?;
            }
            for (j, process) in self.processes.iter().enumerate() {
                for k in 0..r {
                    let q = process.inputs[k]
                        .checked_mul(p.activities[j])
                        .ok_or(MathError::Overflow)?;
                    record(
                        t,
                        PaymentSource::Input {
                            process: j,
                            resource: k,
                        },
                        q,
                        k,
                    )?;
                    if capital.contains(&k) && t >= process.lead_periods {
                        let q = process.outputs[k]
                            .checked_mul(plan[t - process.lead_periods].activities[j])
                            .ok_or(MathError::Overflow)?;
                        record(
                            t,
                            PaymentSource::Capital {
                                process: j,
                                resource: k,
                            },
                            q,
                            k,
                        )?;
                    }
                }
            }
        }
        for link in links {
            let Some((q, amount)) = expected.remove(&(link.period, link.source.clone())) else {
                return Err(MathError::Invalid("duplicate or unsupported goods payment"));
            };
            if link.quantity != q || link.amount != amount {
                return Err(MathError::Invalid(
                    "payment does not match physical quantity and norm",
                ));
            }
        }
        if !expected.is_empty() {
            return Err(MathError::Invalid("missing goods payment"));
        }
        Ok(())
    }
}

/// Closing inventory is bounded by base space plus completed storage assets.
/// Assets completed this period may store goods at its close; production-use
/// capacities remain governed separately by prior-period StockCapacity rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuiltStorage {
    pub name: String,
    pub stored_resource: usize,
    pub asset_resource: usize,
    pub base_capacity: i64,
    pub capacity_per_asset: i64,
}
impl Horizon {
    pub fn compile_reserves(
        &self,
        limits: &[StockCapacity],
        exits: &[Withdrawal],
        stores: &[BuiltStorage],
    ) -> Result<LinearModel, MathError> {
        let mut model = self.compile_capabilities(limits, exits)?;
        let r = self.resources.len();
        let width = 3 * r + self.processes.len();
        let mut names = std::collections::BTreeSet::new();
        for store in stores {
            if store.name.trim().is_empty()
                || !names.insert(&store.name)
                || store.stored_resource >= r
                || store.asset_resource >= r
                || store.stored_resource == store.asset_resource
                || store.base_capacity < 0
                || store.capacity_per_asset <= 0
            {
                return Err(MathError::Invalid("invalid built storage capacity"));
            }
            for t in 0..self.periods.len() {
                model.rows.push(Row {
                    name: format!("built_storage:{}:{t}", store.name),
                    terms: vec![
                        (t * width + r + store.stored_resource, 1),
                        (
                            t * width + r + store.asset_resource,
                            -store.capacity_per_asset,
                        ),
                    ],
                    sense: Sense::AtMost,
                    rhs: store.base_capacity,
                });
            }
        }
        Ok(model)
    }
}
