//! Offline rolling plans retain an immutable, verified prefix on an absolute
//! timeline. A checkpoint is mathematical evidence, not proof of real execution.
use crate::{
    capital::{CapitalCycle, CapitalEvidence},
    horizon::PeriodPlan,
    mathematics::{LinearModel, MathError, Row, Sense, verify_linear},
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Commitment {
    pub process: usize,
    pub started: usize,
    pub completes: usize,
    pub activities: i64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Cohort {
    pub resource: usize,
    pub quantity: i64,
    pub expires: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct RollingSnapshot {
    /// First period that may be replanned. All dates remain absolute.
    pub boundary: usize,
    pub opening_stock: Vec<i64>,
    pub pending: Vec<Commitment>,
    /// Includes cohorts expiring at the boundary; these cannot operate there.
    pub opening_cohorts: Vec<Cohort>,
}
#[derive(Clone, Debug)]
pub struct RollingCapital {
    original: CapitalCycle,
    prefix: Vec<PeriodPlan>,
    snapshot: RollingSnapshot,
}
fn equal<T: Serialize>(a: &T, b: &T) -> Result<bool, MathError> {
    Ok(
        serde_json::to_value(a).map_err(|_| MathError::Invalid("checkpoint encoding"))?
            == serde_json::to_value(b).map_err(|_| MathError::Invalid("checkpoint encoding"))?,
    )
}
fn require<T: Serialize>(a: &T, b: &T) -> Result<(), MathError> {
    if equal(a, b)? {
        Ok(())
    } else {
        Err(MathError::Invalid(
            "rolling history or physical schema changed",
        ))
    }
}
impl RollingCapital {
    pub fn checkpoint(
        model: &CapitalCycle,
        plan: &[PeriodPlan],
        boundary: usize,
    ) -> Result<Self, MathError> {
        if boundary == 0 || boundary > plan.len() {
            return Err(MathError::Invalid("checkpoint boundary"));
        }
        verify_linear(
            &model.compile()?,
            &model.seasonal.physical.vector(plan)?,
            None,
        )?;
        model.replay(plan)?;
        let h = &model.seasonal.physical;
        let mut pending = Vec::new();
        let mut cohorts = Vec::new();
        for life in &model.lives {
            if life.initial_remaining_periods >= boundary && h.initial[life.resource] > 0 {
                cohorts.push(Cohort {
                    resource: life.resource,
                    quantity: h.initial[life.resource],
                    expires: life.initial_remaining_periods,
                });
            }
        }
        for (t, row) in plan[..boundary].iter().enumerate() {
            for (j, p) in h.processes.iter().enumerate() {
                let n = row.activities[j];
                if n == 0 {
                    continue;
                }
                let completion = t.checked_add(p.lead_periods).ok_or(MathError::Overflow)?;
                if completion >= boundary {
                    pending.push(Commitment {
                        process: j,
                        started: t,
                        completes: completion,
                        activities: n,
                    });
                } else {
                    for life in &model.lives {
                        let expires = completion
                            .checked_add(1)
                            .and_then(|x| x.checked_add(life.service_periods))
                            .ok_or(MathError::Overflow)?;
                        if expires >= boundary && p.outputs[life.resource] > 0 {
                            cohorts.push(Cohort {
                                resource: life.resource,
                                quantity: n
                                    .checked_mul(p.outputs[life.resource])
                                    .ok_or(MathError::Overflow)?,
                                expires,
                            });
                        }
                    }
                }
            }
        }
        let snapshot = RollingSnapshot {
            boundary,
            opening_stock: plan[boundary - 1].closing.clone(),
            pending,
            opening_cohorts: cohorts,
        };
        Ok(Self {
            original: model.clone(),
            prefix: plan[..boundary].to_vec(),
            snapshot,
        })
    }
    pub fn snapshot(&self) -> &RollingSnapshot {
        &self.snapshot
    }
    fn compatible(&self, next: &CapitalCycle) -> Result<(), MathError> {
        let a = &self.original.seasonal;
        let b = &next.seasonal;
        let t = self.snapshot.boundary;
        let h = &a.physical;
        let n = &b.physical;
        if n.periods.len() < t || b.land.len() != a.land.len() || b.crops.len() != a.crops.len() {
            return Err(MathError::Invalid("rolling horizon or seasonal identity"));
        }
        require(
            &(
                &h.resources,
                &h.initial,
                &h.storage_caps,
                &h.processes,
                h.objective_resource,
            ),
            &(
                &n.resources,
                &n.initial,
                &n.storage_caps,
                &n.processes,
                n.objective_resource,
            ),
        )?;
        require(
            &(
                &self.original.limits,
                &self.original.lives,
                &self.original.stores,
            ),
            &(&next.limits, &next.lives, &next.stores),
        )?;
        require(&&h.periods[..t], &&n.periods[..t])?;
        require(&a.irrigation_m3.get(..t), &b.irrigation_m3.get(..t))?;
        require(&a.closing_floors.get(..t), &b.closing_floors.get(..t))?;
        for (x, y) in a.land.iter().zip(&b.land) {
            require(
                &(&x.name, x.available_hectares.get(..t)),
                &(&y.name, y.available_hectares.get(..t)),
            )?;
        }
        for (x, y) in a.crops.iter().zip(&b.crops) {
            require(
                &(
                    x.process,
                    x.land_pool,
                    x.hectares_per_activity,
                    &x.irrigation_m3_per_activity,
                    &x.basis,
                ),
                &(
                    y.process,
                    y.land_pool,
                    y.hectares_per_activity,
                    &y.irrigation_m3_per_activity,
                    &y.basis,
                ),
            )?;
            require(
                &x.sowing_periods
                    .iter()
                    .filter(|&&s| s < t)
                    .collect::<Vec<_>>(),
                &y.sowing_periods
                    .iter()
                    .filter(|&&s| s < t)
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(())
    }
    pub fn compile(&self, next: &CapitalCycle) -> Result<LinearModel, MathError> {
        let mut m = next.compile()?;
        self.compatible(next)?;
        let r = next.seasonal.physical.resources.len();
        let width = 3 * r + next.seasonal.physical.processes.len();
        for (t, p) in self.prefix.iter().enumerate() {
            for (k, &q) in p
                .deliveries
                .iter()
                .chain(&p.closing)
                .chain(&p.discarded)
                .chain(&p.activities)
                .enumerate()
            {
                m.rows.push(Row {
                    name: format!("committed:{t}:{k}"),
                    terms: vec![(t * width + k, 1)],
                    sense: Sense::Equal,
                    rhs: q,
                });
            }
        }
        Ok(m)
    }
    pub fn replay(
        &self,
        next: &CapitalCycle,
        plan: &[PeriodPlan],
    ) -> Result<CapitalEvidence, MathError> {
        self.compatible(next)?;
        let prefix = plan
            .get(..self.prefix.len())
            .ok_or(MathError::Invalid("short rolling plan"))?;
        require(&self.prefix.as_slice(), &prefix)?;
        next.replay(plan)
    }
}
