//! Derived, exact research circulation for an approved physical plan.
//! This module never posts to the live ledger or certifies observed delivery.
use crate::{
    horizon::{CashLink, PaymentSource},
    ledger::LedgerError,
    mathematics::{
        LiquidityBound, Movement, ScheduleRun, ScheduledMovement, replay_schedule, schedule_opening,
    },
    model_scenarios::{ModelProposal, ModelVersion, checkpoint as physical_checkpoint, text},
    rolling::RollingCapital,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const MAX_COUPLED_BYTES: usize = 8 * 1024 * 1024;
const MAX_EVENTS: usize = 20_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CirculationPolicy {
    /// Index 0 is common treasury; index 1 is the aggregate resident scope.
    pub accounts: Vec<String>,
    pub resource_suppliers: Vec<usize>,
    pub process_operators: Vec<usize>,
    pub process_order: Vec<usize>,
    pub capital_resources: Vec<usize>,
    /// Canonical nonnegative integer strings; fixed experimental accounting norms.
    pub unit_measure: Vec<String>,
    pub common_measure: Vec<String>,
    pub opening: Vec<String>,
    pub measure_unit: String,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledDefinition {
    pub id: Uuid,
    pub format_version: u32,
    pub model_id: Uuid,
    pub policy: CirculationPolicy,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledProposal {
    pub id: Uuid,
    pub expected_generation: i64,
    pub model_generation: i64,
    pub unavailable_events: Vec<usize>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    ProvisionAllocation,
    Goods,
    CommonCirculation,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoupledEvent {
    pub period: usize,
    pub from: usize,
    pub to: usize,
    pub amount: String,
    pub kind: EventKind,
    pub link: usize,
    /// A common share circulates only after the associated goods payment settles.
    pub requires: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuedFlow {
    pub physical: CashLink,
    pub payer: usize,
    pub supplier: usize,
    /// None means an internal use within the same unit, not an omitted payment.
    pub goods_event: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailedEvent {
    pub event: usize,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoupledEvidence {
    pub physical: ModelVersion,
    pub links: Vec<ValuedFlow>,
    pub events: Vec<CoupledEvent>,
    pub unavailable_events: Vec<usize>,
    pub bound: LiquidityBound,
    pub opening_deficits: Vec<String>,
    pub money: ScheduleRun,
    pub failed: Vec<FailedEvent>,
    pub closing_by_period: Vec<Vec<String>>,
    pub all_scheduled_payments_settled: bool,
    pub resident_zero_each_period: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoupledCheckpoint {
    pub format_version: u32,
    pub scope: Uuid,
    pub generation: i64,
    pub evidence: CoupledEvidence,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoupledVersion {
    pub definition: CoupledDefinition,
    pub generation: i64,
    pub checkpoint: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoupledDraft {
    pub scenario_id: Uuid,
    pub request: CoupledProposal,
    pub candidate: CoupledVersion,
    pub state: String,
    pub decision_basis: Option<String>,
}
fn invalid() -> LedgerError {
    LedgerError::Invalid("invalid circulation policy or physical/payment mapping")
}
fn number(s: &str) -> Result<u64, LedgerError> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }
    s.parse().map_err(|_| LedgerError::Overflow)
}
impl CoupledDefinition {
    pub(crate) fn validate(&self, physical: &ModelVersion) -> Result<(), LedgerError> {
        text(&self.basis, 1000)?;
        text(&self.policy.basis, 1000)?;
        text(&self.policy.measure_unit, 128)?;
        let c = physical_checkpoint(physical)?
            .ok_or(LedgerError::Invalid("approved physical model required"))?;
        let h = &c.input.model.seasonal.physical;
        let p = &self.policy;
        let n = p.accounts.len();
        let r = h.resources.len();
        let j = h.processes.len();
        if self.format_version != 1
            || self.model_id != physical.definition.id
            || !(3..=64).contains(&n)
            || p.resource_suppliers.len() != r
            || p.unit_measure.len() != r
            || p.common_measure.len() != r
            || p.process_operators.len() != j
            || p.process_order.len() != j
            || p.opening.len() != n
            || p.process_order.iter().copied().collect::<BTreeSet<_>>() != (0..j).collect()
            || p.resource_suppliers.iter().any(|&x| x < 2 || x >= n)
            || p.process_operators.iter().any(|&x| x < 2 || x >= n)
        {
            return Err(invalid());
        }
        let mut names = BTreeSet::new();
        for name in &p.accounts {
            text(name, 128)?;
            if !names.insert(name) || name != name.trim() {
                return Err(invalid());
            }
        }
        let declared = p.capital_resources.iter().copied().collect::<BTreeSet<_>>();
        let actual = c
            .input
            .model
            .lives
            .iter()
            .map(|v| v.resource)
            .collect::<BTreeSet<_>>();
        if declared.len() != p.capital_resources.len() || declared != actual {
            return Err(LedgerError::Invalid(
                "capital mapping must include every declared finite-life asset",
            ));
        }
        for (a, b) in p.unit_measure.iter().zip(&p.common_measure) {
            let v = number(a)?;
            let common = number(b)?;
            if v > i64::MAX as u64 || common > v {
                return Err(invalid());
            }
        }
        for v in &p.opening {
            number(v)?;
        }
        if p.opening[1] != "0" {
            return Err(LedgerError::Invalid(
                "resident opening reserve is not part of this common-provision policy",
            ));
        }
        Ok(())
    }
}
struct Builder<'a> {
    policy: &'a CirculationPolicy,
    links: Vec<ValuedFlow>,
    events: Vec<CoupledEvent>,
}
impl Builder<'_> {
    fn event(
        &mut self,
        period: usize,
        endpoints: (usize, usize),
        amount: u64,
        kind: EventKind,
        link: usize,
        requires: Option<usize>,
    ) -> Result<usize, LedgerError> {
        if self.events.len() >= MAX_EVENTS {
            return Err(LedgerError::Invalid("circulation event budget exceeded"));
        }
        let index = self.events.len();
        self.events.push(CoupledEvent {
            period,
            from: endpoints.0,
            to: endpoints.1,
            amount: amount.to_string(),
            kind,
            link,
            requires,
        });
        Ok(index)
    }
    fn trade(
        &mut self,
        period: usize,
        source: PaymentSource,
        resource: usize,
        quantity: i64,
        payer: usize,
        allocate: bool,
    ) -> Result<(), LedgerError> {
        let norm = number(&self.policy.unit_measure[resource])?;
        if quantity == 0 || norm == 0 {
            return Ok(());
        }
        let q = u64::try_from(quantity).map_err(|_| invalid())?;
        let amount = q.checked_mul(norm).ok_or(LedgerError::Overflow)?;
        let common = q
            .checked_mul(number(&self.policy.common_measure[resource])?)
            .ok_or(LedgerError::Overflow)?;
        let supplier = self.policy.resource_suppliers[resource];
        let index = self.links.len();
        if allocate {
            self.event(
                period,
                (0, 1),
                amount,
                EventKind::ProvisionAllocation,
                index,
                None,
            )?;
        }
        let goods = if payer != supplier {
            let goods = self.event(
                period,
                (payer, supplier),
                amount,
                EventKind::Goods,
                index,
                None,
            )?;
            if common > 0 {
                self.event(
                    period,
                    (supplier, 0),
                    common,
                    EventKind::CommonCirculation,
                    index,
                    Some(goods),
                )?;
            }
            Some(goods)
        } else {
            None
        };
        self.links.push(ValuedFlow {
            physical: CashLink {
                period,
                source,
                quantity,
                amount: amount.to_string(),
            },
            payer,
            supplier,
            goods_event: goods,
        });
        Ok(())
    }
}
fn schedule(events: &[CoupledEvent]) -> Result<Vec<ScheduledMovement>, LedgerError> {
    events
        .iter()
        .map(|e| {
            Ok(ScheduledMovement {
                period: e.period as u32,
                movement: Movement {
                    from: e.from,
                    to: e.to,
                    amount: number(&e.amount)?,
                },
                basis: format!("{:?}: physical link {}", e.kind, e.link),
            })
        })
        .collect()
}
pub(crate) fn checkpoint(v: &CoupledVersion) -> Result<Option<CoupledCheckpoint>, LedgerError> {
    let Some(bytes) = &v.checkpoint else {
        return if v.generation == 0 {
            Ok(None)
        } else {
            Err(LedgerError::InvalidStoredResponse)
        };
    };
    if bytes.len() > MAX_COUPLED_BYTES {
        return Err(LedgerError::InvalidStoredResponse);
    }
    let c: CoupledCheckpoint =
        serde_json::from_str(bytes).map_err(|_| LedgerError::InvalidStoredResponse)?;
    if c.format_version != 1
        || c.scope != v.definition.id
        || c.generation != v.generation
        || serde_json::to_string(&c).map_err(|_| LedgerError::InvalidStoredResponse)? != *bytes
    {
        return Err(LedgerError::InvalidStoredResponse);
    }
    Ok(Some(c))
}
pub(crate) fn calculate(
    before: CoupledVersion,
    request: CoupledProposal,
    physical: ModelVersion,
) -> Result<CoupledVersion, LedgerError> {
    before.definition.validate(&physical)?;
    text(&request.basis, 1000)?;
    if request.expected_generation != before.generation
        || request.model_generation != physical.generation
    {
        return Err(LedgerError::ScenarioStale);
    }
    let current = physical_checkpoint(&physical)?.ok_or(LedgerError::InvalidStoredResponse)?;
    // Independently revalidate saved physical assumptions; no caller-owned plan or
    // authoritative payment list is accepted by this API.
    crate::model_scenarios::calculate(
        ModelVersion {
            definition: physical.definition.clone(),
            generation: 0,
            checkpoint: None,
        },
        ModelProposal {
            id: request.id,
            expected_generation: 0,
            input: current.input.clone(),
            basis: request.basis.clone(),
        },
        current.dated.clone(),
    )?;
    let policy = &before.definition.policy;
    let h = &current.input.model.seasonal.physical;
    let plan = &current.input.plan;
    let mut b = Builder {
        policy,
        links: vec![],
        events: vec![],
    };
    for (t, row) in plan.iter().enumerate() {
        for (j, process) in h.processes.iter().enumerate() {
            if t >= process.lead_periods {
                for &r in &policy.capital_resources {
                    let q = process.outputs[r]
                        .checked_mul(plan[t - process.lead_periods].activities[j])
                        .ok_or(LedgerError::Overflow)?;
                    b.trade(
                        t,
                        PaymentSource::Capital {
                            process: j,
                            resource: r,
                        },
                        r,
                        q,
                        0,
                        false,
                    )?;
                }
            }
        }
        for r in 0..h.resources.len() {
            b.trade(
                t,
                PaymentSource::Provision { resource: r },
                r,
                row.deliveries[r],
                1,
                true,
            )?;
        }
        for &j in &policy.process_order {
            for (r, &coefficient) in h.processes[j].inputs.iter().enumerate() {
                let q = coefficient
                    .checked_mul(row.activities[j])
                    .ok_or(LedgerError::Overflow)?;
                b.trade(
                    t,
                    PaymentSource::Input {
                        process: j,
                        resource: r,
                    },
                    r,
                    q,
                    policy.process_operators[j],
                    false,
                )?;
            }
        }
    }
    let links: Vec<_> = b.links.iter().map(|v| v.physical.clone()).collect();
    let norms = policy
        .unit_measure
        .iter()
        .map(|s| number(s).map(|x| x as i64))
        .collect::<Result<Vec<_>, _>>()?;
    h.verify_cash_links(plan, &norms, &policy.capital_resources, &links)
        .map_err(|_| invalid())?;
    let denied = request
        .unavailable_events
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if denied.len() != request.unavailable_events.len()
        || denied.len() > 256
        || denied.iter().any(|&x| x >= b.events.len())
    {
        return Err(LedgerError::Invalid("invalid unavailable event set"));
    }
    if let Some(old) = checkpoint(&before)? {
        let previous = physical_checkpoint(&old.evidence.physical)?
            .ok_or(LedgerError::InvalidStoredResponse)?;
        if current.input.boundary < previous.input.boundary {
            return Err(invalid());
        }
        RollingCapital::checkpoint(
            &previous.input.model,
            &previous.input.plan,
            current.input.boundary,
        )
        .and_then(|prefix| prefix.replay(&current.input.model, plan))
        .map_err(|_| LedgerError::Invalid("coupled physical history changed"))?;
        let boundary = current.input.boundary;
        let prefix = |events: &[CoupledEvent]| {
            events
                .iter()
                .take_while(|e| e.period < boundary)
                .cloned()
                .collect::<Vec<_>>()
        };
        if prefix(&old.evidence.events) != prefix(&b.events) {
            return Err(LedgerError::Invalid("committed circulation changed"));
        }
        let size = prefix(&b.events).len();
        if old
            .evidence
            .unavailable_events
            .iter()
            .copied()
            .filter(|&e| e < size)
            .collect::<BTreeSet<_>>()
            != denied.iter().copied().filter(|&e| e < size).collect()
        {
            return Err(LedgerError::Invalid("committed payment outcome changed"));
        }
    }
    verify_events(policy, &b.links, &b.events)?;
    let events = schedule(&b.events)?;
    let bound = schedule_opening(policy.accounts.len(), &events).map_err(|_| invalid())?;
    let opening = policy
        .opening
        .iter()
        .map(|s| number(s))
        .collect::<Result<Vec<_>, _>>()?;
    let mut balances: Vec<u128> = opening.iter().map(|&x| u128::from(x)).collect();
    let mut settled = vec![false; events.len()];
    let mut failed = vec![];
    let mut effective_denied = denied.clone();
    let mut closing = vec![];
    let mut offset = 0;
    for t in 0..plan.len() {
        while offset < events.len() && b.events[offset].period == t {
            let e = &b.events[offset];
            let amount = u128::from(number(&e.amount)?);
            let reason = if e.requires.is_some_and(|parent| !settled[parent]) {
                effective_denied.insert(offset);
                Some("dependency")
            } else if denied.contains(&offset) {
                Some("unavailable")
            } else if balances[e.from] < amount {
                Some("insufficient_measure")
            } else {
                None
            };
            if let Some(reason) = reason {
                failed.push(FailedEvent {
                    event: offset,
                    reason: reason.into(),
                });
            } else {
                balances[e.from] -= amount;
                balances[e.to] = balances[e.to]
                    .checked_add(amount)
                    .ok_or(LedgerError::Overflow)?;
                settled[offset] = true;
            }
            offset += 1;
        }
        closing.push(balances.iter().map(ToString::to_string).collect::<Vec<_>>());
    }
    let money = replay_schedule(
        &opening,
        &events,
        &effective_denied.into_iter().collect::<Vec<_>>(),
    )
    .map_err(|_| invalid())?;
    if money.closing != balances.iter().map(ToString::to_string).collect::<Vec<_>>()
        || money.settled_events != settled.iter().filter(|&&s| s).count()
    {
        return Err(LedgerError::InvalidStoredResponse);
    }
    let deficits = bound
        .minimum_opening_by_account
        .iter()
        .zip(&opening)
        .map(|(b, &o)| {
            b.parse::<u128>()
                .map(|b| b.saturating_sub(u128::from(o)).to_string())
                .map_err(|_| LedgerError::InvalidStoredResponse)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let evidence = CoupledEvidence {
        physical,
        links: b.links,
        events: b.events,
        unavailable_events: denied.into_iter().collect(),
        bound,
        opening_deficits: deficits,
        all_scheduled_payments_settled: failed.is_empty(),
        resident_zero_each_period: closing.iter().all(|p| p[1] == "0"),
        money,
        failed,
        closing_by_period: closing,
    };
    let generation = before
        .generation
        .checked_add(1)
        .ok_or(LedgerError::Overflow)?;
    let state = CoupledCheckpoint {
        format_version: 1,
        scope: before.definition.id,
        generation,
        evidence,
    };
    let bytes = serde_json::to_string(&state).map_err(|_| LedgerError::InvalidStoredResponse)?;
    if bytes.len() > MAX_COUPLED_BYTES {
        return Err(LedgerError::Invalid("coupled checkpoint exceeds 8 MiB"));
    }
    Ok(CoupledVersion {
        definition: before.definition,
        generation,
        checkpoint: Some(bytes),
    })
}

/// Independent event-to-goods reconciliation. In particular, a correct list of
/// goods links cannot justify an extra, missing or redirected monetary movement.
fn verify_events(
    policy: &CirculationPolicy,
    links: &[ValuedFlow],
    events: &[CoupledEvent],
) -> Result<(), LedgerError> {
    // Re-derive ownership and valuation from the immutable policy, not from
    // mutually consistent but jointly corrupted links and events.
    for v in links {
        let (r, payer) = match v.physical.source {
            PaymentSource::Input { process, resource } => (
                resource,
                *policy.process_operators.get(process).ok_or_else(invalid)?,
            ),
            PaymentSource::Provision { resource } => (resource, 1),
            PaymentSource::Capital { resource, .. } => (resource, 0),
        };
        let supplier = *policy.resource_suppliers.get(r).ok_or_else(invalid)?;
        let norm = number(policy.unit_measure.get(r).ok_or_else(invalid)?)?;
        let quantity = u64::try_from(v.physical.quantity).map_err(|_| invalid())?;
        let amount = quantity.checked_mul(norm).ok_or(LedgerError::Overflow)?;
        if amount == 0
            || v.payer != payer
            || v.supplier != supplier
            || v.physical.amount != amount.to_string()
        {
            return Err(invalid());
        }
    }
    let mut seen = BTreeSet::new();
    for (i, e) in events.iter().enumerate() {
        let v = links.get(e.link).ok_or_else(invalid)?;
        let r = match v.physical.source {
            PaymentSource::Input { resource, .. }
            | PaymentSource::Provision { resource }
            | PaymentSource::Capital { resource, .. } => resource,
        };
        let key = match e.kind {
            EventKind::ProvisionAllocation => 0,
            EventKind::Goods => 1,
            EventKind::CommonCirculation => 2,
        };
        if !seen.insert((e.link, key)) || e.period != v.physical.period || v.payer == v.supplier {
            return Err(invalid());
        }
        match e.kind {
            EventKind::ProvisionAllocation => {
                if !matches!(v.physical.source, PaymentSource::Provision { .. })
                    || (e.from, e.to) != (0, 1)
                    || e.amount != v.physical.amount
                    || e.requires.is_some()
                {
                    return Err(invalid());
                }
            }
            EventKind::Goods => {
                if (e.from, e.to) != (v.payer, v.supplier)
                    || e.amount != v.physical.amount
                    || e.requires.is_some()
                    || v.goods_event != Some(i)
                {
                    return Err(invalid());
                }
            }
            EventKind::CommonCirculation => {
                let q = u64::try_from(v.physical.quantity).map_err(|_| invalid())?;
                let common = q
                    .checked_mul(number(&policy.common_measure[r])?)
                    .ok_or(LedgerError::Overflow)?;
                if common == 0
                    || (e.from, e.to) != (v.supplier, 0)
                    || e.amount != common.to_string()
                    || e.requires != v.goods_event
                    || !e.requires.is_some_and(|parent| {
                        parent < i
                            && events[parent].kind == EventKind::Goods
                            && events[parent].link == e.link
                    })
                {
                    return Err(invalid());
                }
            }
        }
    }
    for (i, v) in links.iter().enumerate() {
        let r = match v.physical.source {
            PaymentSource::Input { resource, .. }
            | PaymentSource::Provision { resource }
            | PaymentSource::Capital { resource, .. } => resource,
        };
        let internal = v.payer == v.supplier;
        if internal && v.goods_event.is_some() {
            return Err(invalid());
        }
        let expected = [
            !internal && matches!(v.physical.source, PaymentSource::Provision { .. }),
            !internal,
            !internal && number(&policy.common_measure[r])? > 0,
        ];
        for (kind, needed) in expected.iter().enumerate() {
            if seen.contains(&(i, kind)) != *needed {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_scenarios::{ModelDefinition, ModelResource};
    fn fixture() -> (CoupledVersion, CoupledProposal, ModelVersion) {
        let input: crate::model_scenarios::ModelInput =
            serde_json::from_str(include_str!("../fixtures/model-scenario.json")).unwrap();
        let def = ModelDefinition {
            id: Uuid::new_v4(),
            format_version: 1,
            resources: input
                .model
                .seasonal
                .physical
                .resources
                .iter()
                .map(|name| ModelResource {
                    name: name.clone(),
                    unit: "test unit".into(),
                })
                .collect(),
            period_unit: "day".into(),
            time_origin: "synthetic zero".into(),
            dated_bindings: vec![],
            basis: "fixture".into(),
        };
        let p = crate::model_scenarios::calculate(
            ModelVersion {
                definition: def,
                generation: 0,
                checkpoint: None,
            },
            ModelProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                input,
                basis: "fixture".into(),
            },
            vec![],
        )
        .unwrap();
        let d = CoupledDefinition {
            id: Uuid::new_v4(),
            format_version: 1,
            model_id: p.definition.id,
            policy: serde_json::from_str(include_str!("../fixtures/circulation-policy.json"))
                .unwrap(),
            basis: "Synthetic common-unit policy".into(),
        };
        (
            CoupledVersion {
                definition: d,
                generation: 0,
                checkpoint: None,
            },
            CoupledProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                model_generation: 1,
                unavailable_events: vec![],
                basis: "Synthetic test".into(),
            },
            p,
        )
    }
    #[test]
    fn complete_physical_uses_have_exactly_one_goods_link_and_money_conserves() {
        let (v, r, p) = fixture();
        let c = checkpoint(&calculate(v, r, p).unwrap()).unwrap().unwrap();
        let e = &c.evidence;
        assert_eq!(e.links.len(), 35);
        assert_eq!(e.events.len(), 84);
        assert_eq!(e.money.settled_volume, "198");
        assert_eq!(e.money.opening_total, "160");
        assert_eq!(e.money.closing, vec!["98", "0", "0", "30", "32"]);
        assert!(e.all_scheduled_payments_settled && e.resident_zero_each_period);
        assert_eq!(
            e.links
                .iter()
                .filter(|v| matches!(v.physical.source, PaymentSource::Capital { .. }))
                .map(|v| v.physical.period)
                .collect::<Vec<_>>(),
            vec![2, 5, 8, 11]
        );
        // The first replacement is paid when completed at 2, never at its start 1.
        assert_eq!(
            e.events[..5]
                .iter()
                .map(|v| (v.from, v.to, v.amount.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (0, 1, "3"),
                (1, 4, "3"),
                (4, 0, "1"),
                (4, 2, "2"),
                (2, 0, "2")
            ]
        );
    }
    #[test]
    fn blocked_goods_never_circulate_a_common_share_and_no_funds_are_created() {
        let (v, mut r, p) = fixture();
        r.unavailable_events = vec![1];
        let e = checkpoint(&calculate(v, r, p).unwrap())
            .unwrap()
            .unwrap()
            .evidence;
        assert!(!e.all_scheduled_payments_settled);
        assert!(!e.resident_zero_each_period);
        assert!(e.failed.contains(&FailedEvent {
            event: 1,
            reason: "unavailable".into()
        }));
        assert!(e.failed.contains(&FailedEvent {
            event: 2,
            reason: "dependency".into()
        }));
        assert_eq!(e.closing_by_period[0][1], "3");
        assert_eq!(
            e.money
                .closing
                .iter()
                .map(|x| x.parse::<u128>().unwrap())
                .sum::<u128>(),
            160
        );
    }
    #[test]
    fn underfunded_and_physically_invalid_are_distinct_failures() {
        let (mut v, r, p) = fixture();
        v.definition.policy.opening = vec!["0".into(); 5];
        let e = checkpoint(&calculate(v.clone(), r.clone(), p.clone()).unwrap())
            .unwrap()
            .unwrap()
            .evidence;
        assert!(!e.all_scheduled_payments_settled);
        assert!(e.money.insufficient_events > 0);
        assert!(e.opening_deficits.iter().any(|x| x != "0"));
        assert_eq!(e.money.opening_total, "0");
        let mut bad = p;
        let mut physical = physical_checkpoint(&bad).unwrap().unwrap();
        physical.input.plan[0].closing[0] += 1;
        bad.checkpoint = Some(serde_json::to_string(&physical).unwrap());
        assert!(calculate(v, r, bad).is_err());
    }
    #[test]
    fn independent_reconciliation_rejects_forged_missing_duplicate_or_redirected_events() {
        let (v, r, p) = fixture();
        let e = checkpoint(&calculate(v.clone(), r, p).unwrap())
            .unwrap()
            .unwrap()
            .evidence;
        for change in 0..5 {
            let mut events = e.events.clone();
            match change {
                0 => events[1].amount = "4".into(),
                1 => events[1].to = 3,
                2 => events[2].requires = Some(0),
                3 => {
                    events.pop();
                }
                _ => events.push(events[1].clone()),
            }
            assert!(verify_events(&v.definition.policy, &e.links, &events).is_err());
        }
        let mut links = e.links.clone();
        let mut events = e.events.clone();
        links[0].supplier = 3;
        events[1].to = 3;
        events[2].from = 3;
        assert!(verify_events(&v.definition.policy, &links, &events).is_err());
        let mut links = e.links.clone();
        let mut events = e.events.clone();
        links[0].physical.amount = "4".into();
        events[0].amount = "4".into();
        events[1].amount = "4".into();
        assert!(verify_events(&v.definition.policy, &links, &events).is_err());
    }
    #[test]
    fn internal_use_is_recorded_without_fictitious_payment_or_common_receipt() {
        let (mut v, r, p) = fixture();
        v.definition.policy.resource_suppliers[0] = 4;
        let e = checkpoint(&calculate(v, r, p).unwrap())
            .unwrap()
            .unwrap()
            .evidence;
        let internal = e
            .links
            .iter()
            .filter(|v| v.payer == v.supplier)
            .collect::<Vec<_>>();
        assert_eq!(internal.len(), 13);
        assert!(internal.iter().all(|v| v.goods_event.is_none()));
        assert_eq!(e.links.len(), 35);
        assert_eq!(e.events.len(), 58);
    }
    #[test]
    fn coupled_replanning_cannot_change_frozen_failures() {
        let (v, r, p) = fixture();
        let saved = calculate(v, r.clone(), p.clone()).unwrap();
        let mut next = r;
        next.expected_generation = 1;
        next.unavailable_events = vec![1];
        assert!(calculate(saved.clone(), next.clone(), p.clone()).is_err());
        next.unavailable_events = vec![83];
        assert!(calculate(saved, next, p).is_ok());
    }
    #[test]
    fn exact_strings_reject_noncanonical_norms_and_excess_common_share() {
        let (v, r, p) = fixture();
        for bad in ["01", "1.0", "-1", "1e3"] {
            let mut x = v.clone();
            x.definition.policy.unit_measure[0] = bad.into();
            assert!(calculate(x, r.clone(), p.clone()).is_err());
        }
        let mut x = v;
        x.definition.policy.common_measure[0] = "3".into();
        assert!(calculate(x, r, p).is_err());
    }
    #[test]
    fn exact_opening_bound_is_sufficient_and_each_positive_component_is_necessary() {
        let (v, r, p) = fixture();
        let reference = checkpoint(&calculate(v.clone(), r.clone(), p.clone()).unwrap())
            .unwrap()
            .unwrap();
        let mut funded = v;
        funded.definition.policy.opening = reference.evidence.bound.minimum_opening_by_account;
        let e = checkpoint(&calculate(funded.clone(), r.clone(), p.clone()).unwrap())
            .unwrap()
            .unwrap()
            .evidence;
        assert!(e.all_scheduled_payments_settled);
        for i in 0..funded.definition.policy.opening.len() {
            let n = number(&funded.definition.policy.opening[i]).unwrap();
            if n > 0 {
                let mut low = funded.clone();
                low.definition.policy.opening[i] = (n - 1).to_string();
                let e = checkpoint(&calculate(low, r.clone(), p.clone()).unwrap())
                    .unwrap()
                    .unwrap()
                    .evidence;
                assert!(!e.all_scheduled_payments_settled);
                assert!(e.money.insufficient_events > 0);
            }
        }
    }
    #[test]
    fn all_84_single_event_outages_conserve_funds_and_block_dependent_common_receipts() {
        let (v, mut r, p) = fixture();
        for denied in 0..84 {
            r.unavailable_events = vec![denied];
            let e = checkpoint(&calculate(v.clone(), r.clone(), p.clone()).unwrap())
                .unwrap()
                .unwrap()
                .evidence;
            assert!(!e.all_scheduled_payments_settled);
            for failed in &e.failed {
                for (i, event) in e.events.iter().enumerate() {
                    if event.requires == Some(failed.event) {
                        assert!(
                            e.failed
                                .iter()
                                .any(|f| f.event == i && f.reason == "dependency")
                        );
                    }
                }
            }
            for period in e.closing_by_period {
                assert_eq!(
                    period
                        .iter()
                        .map(|x| x.parse::<u128>().unwrap())
                        .sum::<u128>(),
                    160
                );
            }
        }
    }
    #[test]
    fn aggregate_balances_above_u64_remain_exact_strings() {
        let (mut v, r, p) = fixture();
        v.definition.policy.opening = vec![
            u64::MAX.to_string(),
            "0".into(),
            u64::MAX.to_string(),
            u64::MAX.to_string(),
            u64::MAX.to_string(),
        ];
        let e = checkpoint(&calculate(v, r, p).unwrap())
            .unwrap()
            .unwrap()
            .evidence;
        assert!(e.all_scheduled_payments_settled);
        assert_eq!(
            e.money.opening_total,
            (4 * u128::from(u64::MAX)).to_string()
        );
        assert!(
            e.money
                .closing
                .iter()
                .any(|v| v.parse::<u128>().unwrap() > u128::from(u64::MAX))
        );
    }
}
