//! Bounded offline repair of one not-yet-started service development batch.
//! No global optimality, observation authentication or execution authority.
use crate::{
    ledger::LedgerError,
    model_scenarios::{ModelInput, ModelVersion, checkpoint, preview_continuation},
};
use serde::Serialize;

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
    let h = &input.model.seasonal.physical;
    input.model.replay(&input.plan).map_err(|_| invalid())?;
    let p = h.processes.get(process).ok_or_else(invalid)?;
    if start < input.boundary
        || target < start
        || target >= input.plan.len()
        || target
            .checked_add(p.lead_periods)
            .is_none_or(|t| t >= input.plan.len())
    {
        return Err(invalid());
    }
    let quantity = *input
        .plan
        .get(start)
        .and_then(|r| r.activities.get(process))
        .ok_or_else(invalid)?;
    if quantity <= 0 {
        return Err(invalid());
    }
    let mut next = input.clone();
    if target == start {
        return Ok(next);
    }
    next.plan[start].activities[process] = 0;
    next.plan[target].activities[process] = next.plan[target].activities[process]
        .checked_add(quantity)
        .ok_or(LedgerError::Overflow)?;
    for (t, row) in next.plan.iter_mut().enumerate().skip(start) {
        for (k, value) in row.closing.iter_mut().enumerate() {
            let input_delta = if t < target {
                i128::from(p.inputs[k])
            } else {
                0
            };
            let output_delta = if t >= start + p.lead_periods && t < target + p.lead_periods {
                i128::from(p.outputs[k])
            } else {
                0
            };
            let n = i128::from(*value) + i128::from(quantity) * (input_delta - output_delta);
            *value = i64::try_from(n).map_err(|_| LedgerError::Overflow)?;
        }
    }
    Ok(next)
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
