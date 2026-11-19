//! Bounded offline repair of unstarted service development batches.
//! No global optimality, observation authentication or execution authority.
use crate::{
    ledger::LedgerError,
    model_scenarios::{ModelInput, ModelVersion, checkpoint, preview_continuation},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BatchMove {
    pub process: usize,
    pub start: usize,
    pub target: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchWindow {
    pub process: usize,
    pub start: usize,
    pub latest: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairOutcome {
    Found,
    Exhausted,
    BudgetExceeded,
}

#[derive(Debug, Serialize)]
pub struct JointAttempt {
    pub moves: Vec<BatchMove>,
    pub rejection: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JointRepair {
    pub outcome: RepairOutcome,
    pub search_space: usize,
    pub attempts: Vec<JointAttempt>,
    pub input: Option<ModelInput>,
}

/// Apply simultaneous deltas from original source quantities, including moves
/// onto another batch's source. No intermediate schedule acquires authority.
pub fn defer_batches(input: &ModelInput, moves: &[BatchMove]) -> Result<ModelInput, LedgerError> {
    if moves.is_empty() || moves.len() > 4 {
        return Err(invalid());
    }
    input.model.replay(&input.plan).map_err(|_| invalid())?;
    let mut seen = BTreeSet::new();
    let mut activity: Vec<Vec<i128>> = input
        .plan
        .iter()
        .map(|r| r.activities.iter().map(|&n| i128::from(n)).collect())
        .collect();
    let mut closing: Vec<Vec<i128>> = input
        .plan
        .iter()
        .map(|r| r.closing.iter().map(|&n| i128::from(n)).collect())
        .collect();
    for m in moves {
        let p = input
            .model
            .seasonal
            .physical
            .processes
            .get(m.process)
            .ok_or_else(invalid)?;
        if !seen.insert((m.process, m.start))
            || m.start < input.boundary
            || m.target < m.start
            || m.target
                .checked_add(p.lead_periods)
                .is_none_or(|t| t >= input.plan.len())
        {
            return Err(invalid());
        }
        let n = i128::from(
            *input
                .plan
                .get(m.start)
                .and_then(|r| r.activities.get(m.process))
                .ok_or_else(invalid)?,
        );
        if n <= 0 {
            return Err(invalid());
        }
        activity[m.start][m.process] -= n;
        activity[m.target][m.process] += n;
        for (t, row) in closing.iter_mut().enumerate().skip(m.start) {
            for (k, value) in row.iter_mut().enumerate() {
                let a = if t < m.target {
                    i128::from(p.inputs[k])
                } else {
                    0
                };
                let b = if t >= m.start + p.lead_periods && t < m.target + p.lead_periods {
                    i128::from(p.outputs[k])
                } else {
                    0
                };
                *value = value
                    .checked_add(n.checked_mul(a - b).ok_or(LedgerError::Overflow)?)
                    .ok_or(LedgerError::Overflow)?;
            }
        }
    }
    let mut next = input.clone();
    for (t, row) in next.plan.iter_mut().enumerate() {
        row.activities = activity[t]
            .iter()
            .map(|&n| i64::try_from(n).map_err(|_| LedgerError::Overflow))
            .collect::<Result<_, _>>()?;
        row.closing = closing[t]
            .iter()
            .map(|&n| i64::try_from(n).map_err(|_| LedgerError::Overflow))
            .collect::<Result<_, _>>()?;
    }
    Ok(next)
}

/// Lexicographic search in caller-specified batch order, at most 4 windows of
/// 24 dates and 256 full previews. Budget exhaustion is not infeasibility.
/// Reserves, demand, committed work and money policy cannot be relaxed here.
pub fn repair_service_batches(
    before: &ModelVersion,
    observed: &ModelInput,
    windows: &[BatchWindow],
    budget: usize,
) -> Result<JointRepair, LedgerError> {
    let old = checkpoint(before)?.ok_or_else(invalid)?;
    if windows.is_empty()
        || windows.len() > 4
        || !(1..=256).contains(&budget)
        || !before.definition.dated_bindings.is_empty()
        || observed.boundary < old.input.boundary
    {
        return Err(invalid());
    }
    observed
        .services
        .as_ref()
        .ok_or_else(invalid)?
        .verify_continuation(
            old.input.services.as_ref().ok_or_else(invalid)?,
            observed.boundary,
        )
        .map_err(|_| invalid())?;
    let mut search_space = 1usize;
    let mut moves = Vec::new();
    for w in windows {
        let p = observed
            .model
            .seasonal
            .physical
            .processes
            .get(w.process)
            .ok_or_else(invalid)?;
        if w.latest < w.start
            || w.latest - w.start >= 24
            || w.latest
                .checked_add(p.lead_periods)
                .is_none_or(|t| t >= observed.plan.len())
        {
            return Err(invalid());
        }
        search_space *= w.latest - w.start + 1;
        moves.push(BatchMove {
            process: w.process,
            start: w.start,
            target: w.start,
        });
    }
    // Validate all source windows once, independently of search truncation.
    defer_batches(observed, &moves)?;
    let mut result = JointRepair {
        outcome: RepairOutcome::Exhausted,
        search_space,
        attempts: vec![],
        input: None,
    };
    for index in 0..search_space.min(budget) {
        let mut ordinal = index;
        for (m, w) in moves.iter_mut().zip(windows).rev() {
            let width = w.latest - w.start + 1;
            m.target = w.start + ordinal % width;
            ordinal /= width;
        }
        let candidate = defer_batches(observed, &moves);
        let checked = candidate
            .and_then(|input| preview_continuation(before.clone(), input.clone()).map(|_| input));
        match checked {
            Ok(input) => {
                result.attempts.push(JointAttempt {
                    moves: moves.clone(),
                    rejection: None,
                });
                result.input = Some(input);
                result.outcome = RepairOutcome::Found;
                return Ok(result);
            }
            Err(e) => result.attempts.push(JointAttempt {
                moves: moves.clone(),
                rejection: Some(e.to_string()),
            }),
        }
    }
    if budget < search_space {
        result.outcome = RepairOutcome::BudgetExceeded;
    }
    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct RepairAttempt {
    pub start: usize,
    pub rejection: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct BatchRepair {
    pub attempts: Vec<RepairAttempt>,
    pub input: Option<ModelInput>,
}
fn invalid() -> LedgerError {
    LedgerError::Invalid("batch deferral outside bounded uncommitted horizon")
}

/// Move one entire unstarted batch, adjusting stock by the exact input/output
/// timing delta. Independent full validation is still required before approval.
pub fn defer_batch(
    input: &ModelInput,
    process: usize,
    start: usize,
    target: usize,
) -> Result<ModelInput, LedgerError> {
    defer_batches(
        input,
        &[BatchMove {
            process,
            start,
            target,
        }],
    )
}

/// Try at most 24 start dates in increasing order. Every candidate passes the
/// ordinary continuation verifier; failures retain their reasons in the result.
pub fn repair_service_batch(
    before: &ModelVersion,
    observed: &ModelInput,
    process: usize,
    start: usize,
    latest: usize,
) -> Result<BatchRepair, LedgerError> {
    let old = checkpoint(before)?.ok_or_else(invalid)?;
    if !before.definition.dated_bindings.is_empty()
        || observed.boundary < old.input.boundary
        || start < observed.boundary
        || latest < start
        || latest - start >= 24
        || latest >= observed.plan.len()
    {
        return Err(invalid());
    }
    let next = observed.services.as_ref().ok_or_else(invalid)?;
    next.verify_continuation(
        old.input.services.as_ref().ok_or_else(invalid)?,
        observed.boundary,
    )
    .map_err(|_| invalid())?;
    let mut result = BatchRepair {
        attempts: vec![],
        input: None,
    };
    for target in start..=latest {
        let candidate = defer_batch(observed, process, start, target)?;
        match preview_continuation(before.clone(), candidate.clone()) {
            Ok(_) => {
                result.attempts.push(RepairAttempt {
                    start: target,
                    rejection: None,
                });
                result.input = Some(candidate);
                break;
            }
            Err(e) => result.attempts.push(RepairAttempt {
                start: target,
                rejection: Some(e.to_string()),
            }),
        }
    }
    Ok(result)
}

/// Combine 2..16 declarations from the same base and boundary. Equal overlapping
/// cells are deduplicated; conflicting values require an explicit correction.
/// No summing percentages, choosing a minimum or silently preferring a source.
pub fn combine_service_revisions(
    base: &crate::services::ServicePlan,
    revisions: &[crate::services::ServiceRevision],
    basis: String,
    source_reference: String,
) -> Result<crate::services::ServiceRevision, LedgerError> {
    use crate::services::{AvailabilityChange, ServiceRevision};
    use std::collections::BTreeMap;
    let bad = || LedgerError::Invalid("incompatible or oversized service incident combination");
    if !(2..=16).contains(&revisions.len()) {
        return Err(bad());
    }
    let boundary = revisions[0].boundary;
    let mut cells = BTreeMap::new();
    for revision in revisions {
        if revision.boundary != boundary || revision.changes.is_empty() {
            return Err(bad());
        }
        base.with_revision(revision.clone()).map_err(|_| bad())?;
        for change in &revision.changes {
            let key = (change.pool.clone(), change.period);
            if cells
                .insert(key, change.clone())
                .is_some_and(|old| old != *change)
            {
                return Err(bad());
            }
        }
    }
    if cells.len() > 128 {
        return Err(bad());
    }
    let mut changes: Vec<AvailabilityChange> = Vec::with_capacity(cells.len());
    for pool in &base.pools {
        for period in 0..pool.availability_bps.len() {
            if let Some(change) = cells.remove(&(pool.id.clone(), period)) {
                changes.push(change);
            }
        }
    }
    if !cells.is_empty() {
        return Err(bad());
    }
    let combined = ServiceRevision {
        boundary,
        basis,
        source_reference,
        changes,
    };
    base.with_revision(combined.clone()).map_err(|_| bad())?;
    Ok(combined)
}
