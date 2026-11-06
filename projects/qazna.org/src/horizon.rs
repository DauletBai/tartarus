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
