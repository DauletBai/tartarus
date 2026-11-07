//! Causal committed-output memory for offline rolling experiments.
//! Output in transit is a promise, not available inventory or resident receipt.
use crate::{horizon::Process, mathematics::MathError};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Commitment {
    due: usize,
    inputs: Vec<i64>,
    outputs: Vec<i64>,
    starts: i64,
    lead: usize,
}

pub struct ProductionPipeline {
    resources: usize,
    next_period: usize,
    pending: Vec<i64>,
    arrivals: BTreeMap<usize, Vec<i64>>,
    recorded: BTreeMap<(usize, usize), Commitment>,
}
impl ProductionPipeline {
    pub fn new(resources: usize) -> Result<Self, MathError> {
        if resources == 0 || resources > 64 {
            return Err(MathError::Invalid("pipeline dimensions"));
        }
        Ok(Self {
            resources,
            next_period: 0,
            pending: vec![0; resources],
            arrivals: BTreeMap::new(),
            recorded: BTreeMap::new(),
        })
    }
    pub fn pending(&self) -> &[i64] {
        &self.pending
    }
    /// Open the next period once. An interruption prevents NEW starts; it does
    /// not cancel already dispatched output without an explicit loss event.
    pub fn advance(&mut self, period: usize) -> Result<Vec<i64>, MathError> {
        if period != self.next_period {
            return Err(MathError::Invalid("nonsequential pipeline period"));
        }
        let next = period.checked_add(1).ok_or(MathError::Overflow)?;
        let arriving = self
            .arrivals
            .get(&period)
            .cloned()
            .unwrap_or(vec![0; self.resources]);
        let pending: Vec<_> = self
            .pending
            .iter()
            .zip(&arriving)
            .map(|(&a, &b)| {
                a.checked_sub(b)
                    .filter(|&n| n >= 0)
                    .ok_or(MathError::Invalid("pipeline balance"))
            })
            .collect::<Result<_, _>>()?;
        self.arrivals.remove(&period);
        self.pending = pending;
        self.next_period = next;
        Ok(arriving)
    }
    /// Record one process start per (period, process). Identical retry returns
    /// false; changed retry fails. Validation and overflow precede every mutation.
    pub fn commit(
        &mut self,
        start: usize,
        process: usize,
        recipe: &Process,
        starts: i64,
    ) -> Result<bool, MathError> {
        if recipe.inputs.len() != self.resources
            || recipe.outputs.len() != self.resources
            || recipe.lead_periods == 0
            || starts <= 0
            || recipe.inputs.iter().chain(&recipe.outputs).any(|&n| n < 0)
            || !recipe.outputs.iter().any(|&n| n > 0)
        {
            return Err(MathError::Invalid("pipeline recipe or quantity"));
        }
        let due = start
            .checked_add(recipe.lead_periods)
            .ok_or(MathError::Overflow)?;
        if let Some(old) = self.recorded.get(&(start, process)) {
            if old.due == due
                && old.starts == starts
                && old.lead == recipe.lead_periods
                && old.inputs == recipe.inputs
                && old.outputs == recipe.outputs
            {
                return Ok(false);
            }
            return Err(MathError::Invalid("changed pipeline retry"));
        }
        if self.next_period.checked_sub(1) != Some(start) {
            return Err(MathError::Invalid("commit outside open period"));
        }
        let quantities: Vec<_> = recipe
            .outputs
            .iter()
            .map(|&n| n.checked_mul(starts).ok_or(MathError::Overflow))
            .collect::<Result<_, _>>()?;
        let pending: Vec<_> = self
            .pending
            .iter()
            .zip(&quantities)
            .map(|(&a, &b)| a.checked_add(b).ok_or(MathError::Overflow))
            .collect::<Result<_, _>>()?;
        let old = self
            .arrivals
            .get(&due)
            .cloned()
            .unwrap_or(vec![0; self.resources]);
        let arriving: Vec<_> = old
            .iter()
            .zip(&quantities)
            .map(|(&a, &b)| a.checked_add(b).ok_or(MathError::Overflow))
            .collect::<Result<_, _>>()?;
        self.recorded.insert(
            (start, process),
            Commitment {
                due,
                inputs: recipe.inputs.clone(),
                outputs: recipe.outputs.clone(),
                starts,
                lead: recipe.lead_periods,
            },
        );
        self.arrivals.insert(due, arriving);
        self.pending = pending;
        Ok(true)
    }
}
