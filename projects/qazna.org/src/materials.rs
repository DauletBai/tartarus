//! Exact conservation of one declared physical constituent. Zero weights exclude
//! unrelated dimensions (for example energy), not unrecorded material losses.
use crate::{
    horizon::{Horizon, PeriodPlan},
    mathematics::{MathError, verify_linear},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialBasis {
    pub name: String,
    pub unit: String,
    pub source: String,
    pub quantity_per_resource_unit: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialEvidence {
    pub initial: i128,
    pub supplied: i128,
    pub delivered: i128,
    pub discarded: i128,
    pub final_stock: i128,
    /// Constituent in committed work at each closing boundary.
    pub work_in_progress: Vec<i128>,
}
fn add(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_add(b).ok_or(MathError::Overflow)
}
impl MaterialBasis {
    fn weighted(&self, q: &[i64]) -> Result<i128, MathError> {
        if q.len() != self.quantity_per_resource_unit.len() {
            return Err(MathError::Invalid("material dimensions"));
        }
        q.iter()
            .zip(&self.quantity_per_resource_unit)
            .try_fold(0i128, |s, (&a, &b)| add(s, i128::from(a) * i128::from(b)))
    }
    pub fn validate(&self, h: &Horizon) -> Result<(), MathError> {
        h.compile()?;
        if self.name.trim().is_empty()
            || self.unit.trim().is_empty()
            || self.source.trim().is_empty()
            || self.quantity_per_resource_unit.len() != h.resources.len()
            || self.quantity_per_resource_unit.iter().any(|&n| n < 0)
            || !self.quantity_per_resource_unit.iter().any(|&n| n > 0)
        {
            return Err(MathError::Invalid("material basis or provenance"));
        }
        for p in &h.processes {
            if self.weighted(&p.inputs)? != self.weighted(&p.outputs)? {
                return Err(MathError::Invalid(
                    "unbalanced constituent: declare residues and losses",
                ));
            }
        }
        Ok(())
    }
    pub fn audit(&self, h: &Horizon, plan: &[PeriodPlan]) -> Result<MaterialEvidence, MathError> {
        self.validate(h)?;
        verify_linear(&h.compile()?, &h.vector(plan)?, None)?;
        let initial = self.weighted(&h.initial)?;
        let (mut supplied, mut delivered, mut discarded) = (0i128, 0i128, 0i128);
        let mut wip = Vec::new();
        let mut completions = vec![0i128; plan.len()];
        let mut work = 0i128;
        let per_start = h
            .processes
            .iter()
            .map(|p| self.weighted(&p.inputs))
            .collect::<Result<Vec<_>, _>>()?;
        for (t, row) in plan.iter().enumerate() {
            supplied = add(supplied, self.weighted(&h.periods[t].supplies)?)?;
            delivered = add(delivered, self.weighted(&row.deliveries)?)?;
            discarded = add(discarded, self.weighted(&row.discarded)?)?;
            work = work
                .checked_sub(completions[t])
                .ok_or(MathError::Overflow)?;
            for (j, p) in h.processes.iter().enumerate() {
                let mass = per_start[j]
                    .checked_mul(i128::from(row.activities[j]))
                    .ok_or(MathError::Overflow)?;
                work = add(work, mass)?;
                if let Some(end) = t
                    .checked_add(p.lead_periods)
                    .filter(|&end| end < plan.len())
                {
                    completions[end] = add(completions[end], mass)?;
                }
            }
            let accounted = add(
                add(delivered, discarded)?,
                add(self.weighted(&row.closing)?, work)?,
            )?;
            if add(initial, supplied)? != accounted {
                return Err(MathError::Infeasible(format!("material conservation:{t}")));
            }
            wip.push(work);
        }
        Ok(MaterialEvidence {
            initial,
            supplied,
            delivered,
            discarded,
            final_stock: self.weighted(
                &plan
                    .last()
                    .ok_or(MathError::Invalid("empty material plan"))?
                    .closing,
            )?,
            work_in_progress: wip,
        })
    }
}
