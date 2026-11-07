//! Exact, bounded improvement of a supplied physical plan. No solver/AI authority.
//! Keeps every delivery and explicit terminal reserves; reduces starts only.
//! Coordinate irreducibility is NOT global or multi-objective optimality.
use crate::{
    horizon::{Horizon, PeriodPlan, StockCapacity, Withdrawal},
    mathematics::{MathError, verify_linear},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TrimEvidence {
    pub passes: usize,
    pub feasibility_probes: usize,
    pub accepted_changes: usize,
    pub coordinate_irreducible: bool,
    pub terminal_floors: Vec<i64>,
    pub removed_starts_by_process: Vec<String>,
}

struct Change {
    day: usize,
    resource: usize,
    closing: i64,
    discarded: i64,
}

struct Trimmer<'a> {
    model: &'a Horizon,
    limits: &'a [StockCapacity],
    exits: Vec<Vec<i64>>,
    minimum_closing: Vec<Vec<i64>>,
}
impl Trimmer<'_> {
    fn refresh_floor(&mut self, day: usize, plan: &[PeriodPlan]) -> Result<(), MathError> {
        if day + 1 == plan.len() {
            return Ok(());
        }
        self.minimum_closing[day].clone_from(&self.exits[day + 1]);
        for limit in self.limits {
            let used = limit
                .activity_use
                .iter()
                .zip(&plan[day + 1].activities)
                .try_fold(0i128, |sum, (&a, &x)| {
                    sum.checked_add(i128::from(a) * i128::from(x))
                        .ok_or(MathError::Overflow)
                })?;
            let rate = i128::from(limit.units_per_stock);
            let stock = used / rate
                + i128::from(used % rate > 0)
                + i128::from(self.exits[day + 1][limit.resource]);
            let stock = i64::try_from(stock).map_err(|_| MathError::Overflow)?;
            self.minimum_closing[day][limit.resource] =
                self.minimum_closing[day][limit.resource].max(stock);
        }
        Ok(())
    }
    /// Replay only affected stock coordinates, with greedy overflow disposal.
    /// Supply, deliveries and every other activity remain fixed. All arithmetic
    /// uses i128 until checked conversion back to the model's i64 quantities.
    fn candidate(
        &self,
        plan: &[PeriodPlan],
        day: usize,
        process: usize,
        reduction: i64,
        record_changes: bool,
    ) -> Result<Option<Vec<Change>>, MathError> {
        let recipe = &self.model.processes[process];
        let completion = day + recipe.lead_periods;
        let mut changes = Vec::new();
        for k in 0..self.model.resources.len() {
            if recipe.inputs[k] == 0 && recipe.outputs[k] == 0 {
                continue;
            }
            let mut delta = 0i128;
            for (d, period_plan) in plan.iter().enumerate().skip(day) {
                if d == day {
                    delta += i128::from(recipe.inputs[k]) * i128::from(reduction);
                }
                if d == completion {
                    delta -= i128::from(recipe.outputs[k]) * i128::from(reduction);
                }
                let available = i128::from(period_plan.closing[k])
                    + i128::from(period_plan.discarded[k])
                    + delta;
                if available < 0 {
                    return Ok(None);
                }
                let closing = available.min(i128::from(self.model.storage_caps[k]));
                let discarded = available - closing;
                if closing < i128::from(self.minimum_closing[d][k]) {
                    return Ok(None);
                }
                let closing = i64::try_from(closing).map_err(|_| MathError::Overflow)?;
                let discarded = i64::try_from(discarded).map_err(|_| MathError::Overflow)?;
                delta = i128::from(closing) - i128::from(period_plan.closing[k]);
                if record_changes && (delta != 0 || discarded != period_plan.discarded[k]) {
                    changes.push(Change {
                        day: d,
                        resource: k,
                        closing,
                        discarded,
                    });
                }
                // A future lost output must still be propagated after an earlier
                // stock difference was absorbed by overflow disposal.
                if delta == 0 && d >= completion {
                    break;
                }
            }
        }
        Ok(Some(changes))
    }
}

/// Monotone coordinate descent in a caller-declared process order. Returns a new
/// plan; never mutates caller inputs. All deliveries are fixed, so a shortage in
/// the seed is retained rather than claimed solved by trimming production.
///
/// For a single reduction q, greedy stock closing is a minimum of affine
/// functions of q (concave). Lower-stock, withdrawal and shared-capacity guards
/// therefore give an interval containing q=0. Integer bisection finds the largest
/// feasible reduction in this coordinate. No cross-coordinate optimality follows.
pub fn trim_plan(
    model: &Horizon,
    seed: &[PeriodPlan],
    limits: &[StockCapacity],
    exits: &[Withdrawal],
    terminal_floors: &[i64],
    process_order: &[usize],
    max_passes: usize,
) -> Result<(Vec<PeriodPlan>, TrimEvidence), MathError> {
    let matrix = model.compile_capabilities(limits, exits)?;
    verify_linear(&matrix, &model.vector(seed)?, None)?;
    let r = model.resources.len();
    let j = model.processes.len();
    if terminal_floors.len() != r
        || terminal_floors
            .iter()
            .enumerate()
            .any(|(k, &n)| n < 0 || n > seed.last().unwrap().closing[k])
        || max_passes == 0
        || max_passes > 32
        || process_order.is_empty()
        || process_order.len() > j
        || process_order.iter().any(|&p| p >= j)
        || process_order
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != process_order.len()
    {
        return Err(MathError::Invalid(
            "trim reserves, process order or pass budget",
        ));
    }
    let mut withdrawals = vec![vec![0; r]; seed.len()];
    for e in exits {
        withdrawals[e.period][e.resource] = e.quantity;
    }
    let mut trimmer = Trimmer {
        model,
        limits,
        exits: withdrawals,
        minimum_closing: vec![vec![0; r]; seed.len()],
    };
    trimmer
        .minimum_closing
        .last_mut()
        .unwrap()
        .clone_from_slice(terminal_floors);
    for day in 0..seed.len() {
        trimmer.refresh_floor(day, seed)?;
    }
    let mut plan = seed.to_vec();
    let mut evidence = TrimEvidence {
        passes: 0,
        feasibility_probes: 0,
        accepted_changes: 0,
        coordinate_irreducible: false,
        terminal_floors: terminal_floors.to_vec(),
        removed_starts_by_process: vec![],
    };
    let mut removed = vec![0i128; j];
    for _ in 0..max_passes {
        evidence.passes += 1;
        let before = evidence.accepted_changes;
        for day in (0..plan.len()).rev() {
            for &p in process_order {
                let n = plan[day].activities[p];
                if n == 0 {
                    continue;
                }
                // Most starts become essential after the first pass. A one-unit
                // probe avoids logarithmically many rejected large candidates.
                evidence.feasibility_probes += 1;
                if trimmer.candidate(&plan, day, p, 1, false)?.is_none() {
                    continue;
                }
                let mut low = 1;
                let mut high = n;
                while low < high {
                    let q = low + (high - low) / 2 + (high - low) % 2;
                    evidence.feasibility_probes += 1;
                    if trimmer.candidate(&plan, day, p, q, false)?.is_some() {
                        low = q;
                    } else {
                        high = q - 1;
                    }
                }
                if low > 0 {
                    let changes = trimmer
                        .candidate(&plan, day, p, low, true)?
                        .ok_or(MathError::Invalid("inconsistent trim candidate"))?;
                    for c in changes {
                        plan[c.day].closing[c.resource] = c.closing;
                        plan[c.day].discarded[c.resource] = c.discarded;
                    }
                    plan[day].activities[p] -= low;
                    if day > 0 {
                        trimmer.refresh_floor(day - 1, &plan)?;
                    }
                    removed[p] += i128::from(low);
                    evidence.accepted_changes += 1;
                }
            }
        }
        // Independent, original matrix verifies every accepted pass. No floating
        // tolerance, rounding repair or relaxation of a physical row is allowed.
        verify_linear(&matrix, &model.vector(&plan)?, None)?;
        if evidence.accepted_changes == before {
            evidence.coordinate_irreducible = true;
            break;
        }
    }
    evidence.removed_starts_by_process = removed.iter().map(ToString::to_string).collect();
    Ok((plan, evidence))
}
