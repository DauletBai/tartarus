//! Exact, offline verification. No database, model, policy adoption or issuance.
//! A feasible plan is not necessarily optimal. Only a valid dual certificate
//! proves optimality for the supplied linear model, not the model's realism.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sense {
    Equal,
    AtMost,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub name: String,
    /// (variable index, signed integer coefficient); duplicate terms are rejected.
    pub terms: Vec<(usize, i64)>,
    pub sense: Sense,
    pub rhs: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearModel {
    /// All variables are nonnegative; maximize objective dot x.
    pub objective: Vec<i64>,
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationalVector {
    pub numerators: Vec<i64>,
    pub denominator: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verification {
    pub feasible: bool,
    pub optimal_for_supplied_linear_model: bool,
    pub objective_numerator: String,
    pub objective_denominator: String,
    pub integral: bool,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MathError {
    #[error("invalid mathematical input: {0}")]
    Invalid(&'static str),
    #[error("exact arithmetic overflow")]
    Overflow,
    #[error("constraint violated: {0}")]
    Infeasible(String),
    #[error("dual certificate does not prove this optimum")]
    BadCertificate,
}
fn add(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_add(b).ok_or(MathError::Overflow)
}
fn mul(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_mul(b).ok_or(MathError::Overflow)
}

/// Check primal feasibility and, if provided, dual feasibility + zero duality gap.
/// For Ax <= b, x >= 0, max c.x: y >= 0, A^T y >= c, min b.y.
/// Equality rows have unrestricted dual multipliers.
pub fn verify_linear(
    model: &LinearModel,
    primal: &RationalVector,
    dual: Option<&RationalVector>,
) -> Result<Verification, MathError> {
    let n = model.objective.len();
    if n == 0 || primal.numerators.len() != n || primal.denominator <= 0 {
        return Err(MathError::Invalid("primal dimension or denominator"));
    }
    if primal.numerators.iter().any(|&v| v < 0) {
        return Err(MathError::Invalid("negative primal variable"));
    }
    for row in &model.rows {
        let mut seen = std::collections::BTreeSet::new();
        let mut lhs = 0i128;
        for &(index, coefficient) in &row.terms {
            if index >= n || coefficient == 0 || !seen.insert(index) {
                return Err(MathError::Invalid("invalid or duplicate matrix term"));
            }
            lhs = add(
                lhs,
                mul(coefficient.into(), primal.numerators[index].into())?,
            )?;
        }
        let rhs = mul(row.rhs.into(), primal.denominator.into())?;
        if match row.sense {
            Sense::Equal => lhs != rhs,
            Sense::AtMost => lhs > rhs,
        } {
            return Err(MathError::Infeasible(row.name.clone()));
        }
    }
    let objective = model
        .objective
        .iter()
        .zip(&primal.numerators)
        .try_fold(0i128, |sum, (&c, &x)| add(sum, mul(c.into(), x.into())?))?;
    if let Some(dual) = dual {
        if dual.denominator <= 0 || dual.numerators.len() != model.rows.len() {
            return Err(MathError::Invalid("dual dimension or denominator"));
        }
        let mut reduced = vec![0i128; n];
        let mut bound = 0i128;
        for (row, &y) in model.rows.iter().zip(&dual.numerators) {
            if matches!(row.sense, Sense::AtMost) && y < 0 {
                return Err(MathError::BadCertificate);
            }
            bound = add(bound, mul(row.rhs.into(), y.into())?)?;
            for &(index, a) in &row.terms {
                reduced[index] = add(reduced[index], mul(a.into(), y.into())?)?;
            }
        }
        for (&r, &c) in reduced.iter().zip(&model.objective) {
            if r < mul(c.into(), dual.denominator.into())? {
                return Err(MathError::BadCertificate);
            }
        }
        if mul(objective, dual.denominator.into())? != mul(bound, primal.denominator.into())? {
            return Err(MathError::BadCertificate);
        }
    }
    Ok(Verification {
        feasible: true,
        optimal_for_supplied_linear_model: dual.is_some(),
        objective_numerator: objective.to_string(),
        objective_denominator: primal.denominator.to_string(),
        integral: primal
            .numerators
            .iter()
            .all(|x| x % primal.denominator == 0),
    })
}

/// Hamilton proportional allocation of one interchangeable integer resource.
/// This is a named experimental rule, NOT max-min fairness or a clinical policy.
/// Quota errors are < 1 unit. Supply monotonicity is NOT guaranteed (Alabama paradox).
/// Stable unique IDs break equal remainders; caller order cannot alter the result.
pub fn proportional_quota(needs: &[(u64, u64)], supply: u64) -> Result<Vec<u64>, MathError> {
    let mut ids = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for &(id, need) in needs {
        if !ids.insert(id) {
            return Err(MathError::Invalid("duplicate beneficiary id"));
        }
        total = total.checked_add(need).ok_or(MathError::Overflow)?;
    }
    if total == 0 {
        return Ok(vec![0; needs.len()]);
    }
    let allocated = supply.min(total);
    let mut rows = Vec::with_capacity(needs.len());
    let mut result = Vec::with_capacity(needs.len());
    let mut used = 0u64;
    for (i, &(id, need)) in needs.iter().enumerate() {
        let product = u128::from(allocated) * u128::from(need);
        let floor = (product / u128::from(total)) as u64;
        used += floor; // sum of floors <= allocated <= u64::MAX
        result.push(floor);
        rows.push((product % u128::from(total), id, i));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, _, i) in rows.iter().take((allocated - used) as usize) {
        result[i] += 1;
    }
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Movement {
    pub from: usize,
    pub to: usize,
    pub amount: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiquidityBound {
    pub minimum_opening_by_account: Vec<String>,
    pub net_cycle_by_account: Vec<String>,
    pub total_minimum_opening: String,
    pub stationary_repeatable: bool,
}

/// Exact componentwise minimal opening balances for a FIXED ordered cycle.
/// For H repeats, B_i = -min_prefix(delta_i) + (H-1)*max(-net_i,0).
/// This does not authorize any transfer or say which circulation rule is legitimate.
pub fn liquidity_bound(
    accounts: usize,
    movements: &[Movement],
    repeats: u64,
) -> Result<LiquidityBound, MathError> {
    if accounts == 0 || repeats == 0 {
        return Err(MathError::Invalid("positive accounts and repeats required"));
    }
    let mut net = vec![0i128; accounts];
    let mut minima = net.clone();
    for movement in movements {
        if movement.from >= accounts
            || movement.to >= accounts
            || movement.from == movement.to
            || movement.amount == 0
        {
            return Err(MathError::Invalid("invalid circulation edge"));
        }
        net[movement.from] = add(net[movement.from], -i128::from(movement.amount))?;
        net[movement.to] = add(net[movement.to], movement.amount.into())?;
        minima[movement.from] = minima[movement.from].min(net[movement.from]);
        minima[movement.to] = minima[movement.to].min(net[movement.to]);
    }
    let mut opening = Vec::new();
    let mut total = 0;
    for (&low, &delta) in minima.iter().zip(&net) {
        let first = low.checked_neg().ok_or(MathError::Overflow)?;
        let deficit = if delta < 0 {
            delta.checked_neg().ok_or(MathError::Overflow)?
        } else {
            0
        };
        let balance = add(first, mul(i128::from(repeats - 1), deficit)?)?;
        total = add(total, balance)?;
        opening.push(balance.to_string());
    }
    Ok(LiquidityBound {
        minimum_opening_by_account: opening,
        net_cycle_by_account: net.iter().map(ToString::to_string).collect(),
        total_minimum_opening: total.to_string(),
        stationary_repeatable: net.iter().all(|&n| n == 0),
    })
}

#[derive(Eq, PartialEq)]
struct Quotient {
    need: u64,
    allocated: u64,
    id: u64,
    index: usize,
}
impl Ord for Quotient {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Only unsatisfied claims enter the heap, so allocated+1 cannot overflow.
        (u128::from(self.need) * u128::from(other.allocated + 1))
            .cmp(&(u128::from(other.need) * u128::from(self.allocated + 1)))
            .then_with(|| other.id.cmp(&self.id))
            .then_with(|| self.need.cmp(&other.need))
            .then_with(|| self.allocated.cmp(&other.allocated))
            .then_with(|| self.index.cmp(&other.index))
    }
}
impl PartialOrd for Quotient {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Jefferson/D'Hondt allocation, capped by declared needs. Fixed stable IDs.
/// Supply-monotone but can exceed upper proportional quota. Not moral/clinical
/// policy. Start with lower quotas; fewer than N highest-average steps remain.
/// Runtime O(N log N), independent of the number of physical minor units.
pub fn divisor_allocation(needs: &[(u64, u64)], supply: u64) -> Result<Vec<u64>, MathError> {
    let mut ids = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for &(id, need) in needs {
        if !ids.insert(id) {
            return Err(MathError::Invalid("duplicate beneficiary id"));
        }
        total = total.checked_add(need).ok_or(MathError::Overflow)?;
    }
    if total == 0 {
        return Ok(vec![0; needs.len()]);
    }
    let target = supply.min(total);
    let mut result = Vec::with_capacity(needs.len());
    let mut heap = std::collections::BinaryHeap::new();
    let mut used = 0;
    for (index, &(id, need)) in needs.iter().enumerate() {
        let allocated = (u128::from(target) * u128::from(need) / u128::from(total)) as u64;
        result.push(allocated);
        used += allocated;
        if allocated < need {
            heap.push(Quotient {
                need,
                allocated,
                id,
                index,
            });
        }
    }
    while used < target {
        let mut q = heap
            .pop()
            .ok_or(MathError::Invalid("allocation heap exhausted"))?;
        q.allocated += 1;
        result[q.index] += 1;
        used += 1;
        if q.allocated < q.need {
            heap.push(q);
        }
    }
    Ok(result)
}

/// Farkas certificate: y>=0 on <= rows, equality multipliers free,
/// A^T y>=0 and b^T y<0. Then no x>=0 can satisfy the supplied rows.
/// This proves infeasibility of the model, not merely failure of a heuristic.
pub fn verify_infeasible(model: &LinearModel, dual: &RationalVector) -> Result<(), MathError> {
    if model.objective.is_empty()
        || dual.denominator <= 0
        || dual.numerators.len() != model.rows.len()
    {
        return Err(MathError::Invalid("infeasibility certificate dimensions"));
    }
    let mut columns = vec![0i128; model.objective.len()];
    let mut bound = 0;
    for (row, &y) in model.rows.iter().zip(&dual.numerators) {
        if matches!(row.sense, Sense::AtMost) && y < 0 {
            return Err(MathError::BadCertificate);
        }
        let mut seen = std::collections::BTreeSet::new();
        for &(i, a) in &row.terms {
            if i >= columns.len() || a == 0 || !seen.insert(i) {
                return Err(MathError::Invalid("matrix term"));
            }
            columns[i] = add(columns[i], mul(a.into(), y.into())?)?;
        }
        bound = add(bound, mul(row.rhs.into(), y.into())?)?;
    }
    if bound >= 0 || columns.iter().any(|&c| c < 0) {
        return Err(MathError::BadCertificate);
    }
    Ok(())
}

/// A scheduled transfer in an offline experiment. Vector order is authoritative
/// within a period. Basis describes the economic purpose; it grants no authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledMovement {
    pub period: u32,
    pub movement: Movement,
    pub basis: String,
}

fn validate_schedule(accounts: usize, events: &[ScheduledMovement]) -> Result<(), MathError> {
    if accounts == 0 || accounts > 10_000 || events.len() > 1_000_000 {
        return Err(MathError::Invalid("bounded nonempty account set required"));
    }
    let mut last = 0;
    for event in events {
        let m = &event.movement;
        if event.period < last
            || event.basis.trim().is_empty()
            || m.from >= accounts
            || m.to >= accounts
            || m.from == m.to
            || m.amount == 0
        {
            return Err(MathError::Invalid("schedule order, purpose or movement"));
        }
        last = event.period;
    }
    Ok(())
}

/// Minimum balances for a finite, ordered, fully executed schedule. This is a
/// sufficient certificate ONLY if every edge remains available on time.
pub fn schedule_opening(
    accounts: usize,
    events: &[ScheduledMovement],
) -> Result<LiquidityBound, MathError> {
    validate_schedule(accounts, events)?;
    liquidity_bound(
        accounts,
        &events
            .iter()
            .map(|e| e.movement.clone())
            .collect::<Vec<_>>(),
        1,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleRun {
    pub opening_total: String,
    pub closing: Vec<String>,
    pub minimum_balance: Vec<String>,
    pub attempted_events: usize,
    pub settled_events: usize,
    pub settled_volume: String,
    pub unavailable_events: usize,
    pub insufficient_events: usize,
    pub first_blocked_period: Option<u32>,
    pub first_blocked_event: Option<usize>,
}

/// Each planned transfer is attempted once. Unavailable or underfunded transfers
/// move NOTHING; dependent transfers are checked against actual balances. This is
/// a deadline stress trace, not an optimizer or a retry/compensation policy.
pub fn replay_schedule(
    opening: &[u64],
    events: &[ScheduledMovement],
    unavailable: &[usize],
) -> Result<ScheduleRun, MathError> {
    validate_schedule(opening.len(), events)?;
    let denied: std::collections::BTreeSet<_> = unavailable.iter().copied().collect();
    if denied.len() != unavailable.len() || denied.iter().any(|&i| i >= events.len()) {
        return Err(MathError::Invalid("invalid or duplicate unavailable event"));
    }
    let mut balances: Vec<i128> = opening.iter().map(|&v| i128::from(v)).collect();
    let total = balances.iter().try_fold(0, |sum, &v| add(sum, v))?;
    let mut minima = balances.clone();
    let mut report = ScheduleRun {
        opening_total: total.to_string(),
        closing: Vec::new(),
        minimum_balance: Vec::new(),
        attempted_events: events.len(),
        settled_events: 0,
        settled_volume: "0".into(),
        unavailable_events: 0,
        insufficient_events: 0,
        first_blocked_period: None,
        first_blocked_event: None,
    };
    let mut volume = 0;
    for (i, event) in events.iter().enumerate() {
        let m = &event.movement;
        let amount = i128::from(m.amount);
        let blocked = if denied.contains(&i) {
            report.unavailable_events += 1;
            true
        } else if balances[m.from] < amount {
            report.insufficient_events += 1;
            true
        } else {
            false
        };
        if blocked {
            if report.first_blocked_event.is_none() {
                report.first_blocked_event = Some(i);
                report.first_blocked_period = Some(event.period);
            }
            continue;
        }
        let credit = add(balances[m.to], amount)?;
        balances[m.from] -= amount;
        balances[m.to] = credit;
        minima[m.from] = minima[m.from].min(balances[m.from]);
        report.settled_events += 1;
        volume = add(volume, amount)?;
    }
    if balances.iter().try_fold(0, |sum, &v| add(sum, v))? != total {
        return Err(MathError::Invalid("circulation conservation failed"));
    }
    report.closing = balances.iter().map(ToString::to_string).collect();
    report.minimum_balance = minima.iter().map(ToString::to_string).collect();
    report.settled_volume = volume.to_string();
    Ok(report)
}

/// An attaining prefix for one account's common opening requirement.
/// The event index is zero-based in the named, ordered scenario schedule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpeningWitness {
    pub scenario: String,
    pub event: usize,
    pub period: u32,
    pub cumulative_net: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommonOpening {
    pub minimum_opening_by_account: Vec<String>,
    pub total_minimum_opening: String,
    pub witnesses: Vec<Option<OpeningWitness>>,
    pub scenario_count: usize,
}

/// Componentwise minimum opening for a finite family of fixed ordered schedules.
/// Each account must cover its worst prefix across ALL supplied scenarios. This
/// is not max(total of each scenario), an optimizer, or protection outside this
/// family. Mathematical results can exceed runtime account limits; strings retain
/// exact i128 values and callers must check their execution representation.
pub fn common_schedule_opening(
    accounts: usize,
    scenarios: &[(&str, &[ScheduledMovement])],
) -> Result<CommonOpening, MathError> {
    if scenarios.is_empty() || scenarios.len() > 64 {
        return Err(MathError::Invalid("bounded nonempty scenario set required"));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut event_count = 0usize;
    for (name, events) in scenarios {
        if name.trim().is_empty() || name.trim() != *name || !names.insert(*name) {
            return Err(MathError::Invalid(
                "unique nonempty scenario names required",
            ));
        }
        validate_schedule(accounts, events)?;
        event_count = event_count
            .checked_add(events.len())
            .ok_or(MathError::Overflow)?;
        if event_count > 1_000_000 {
            return Err(MathError::Invalid("scenario event budget exceeded"));
        }
    }
    let mut required = vec![0i128; accounts];
    let mut witnesses = vec![None; accounts];
    for (name, events) in scenarios {
        let mut net = vec![0i128; accounts];
        for (i, event) in events.iter().enumerate() {
            let m = &event.movement;
            net[m.from] = add(net[m.from], -i128::from(m.amount))?;
            net[m.to] = add(net[m.to], i128::from(m.amount))?;
            let deficit = net[m.from].checked_neg().ok_or(MathError::Overflow)?;
            if deficit > required[m.from] {
                required[m.from] = deficit;
                witnesses[m.from] = Some(OpeningWitness {
                    scenario: (*name).to_owned(),
                    event: i,
                    period: event.period,
                    cumulative_net: net[m.from].to_string(),
                });
            }
        }
    }
    let total = required.iter().try_fold(0, |s, &n| add(s, n))?;
    Ok(CommonOpening {
        minimum_opening_by_account: required.iter().map(ToString::to_string).collect(),
        total_minimum_opening: total.to_string(),
        witnesses,
        scenario_count: scenarios.len(),
    })
}
