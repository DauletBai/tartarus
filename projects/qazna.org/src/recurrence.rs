//! Bounded recurrence experiments. No issuance, live postings or stock resets.
use crate::{
    coupled::{self, CoupledCheckpoint, CoupledVersion},
    horizon::PeriodPlan,
    ledger::LedgerError,
    mathematics::verify_linear,
    model_scenarios::ModelInput,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RenewalTransfer {
    pub cycle: usize,
    pub from: usize,
    pub to: usize,
    pub amount: String,
}
#[derive(Debug, Serialize)]
pub struct CashRecurrence {
    pub requested_cycles: usize,
    pub completed_cycles: usize,
    pub opening_total: String,
    pub closing: Vec<String>,
    pub cycle_closings: Vec<Vec<String>>,
    pub renewal_transfers: Vec<RenewalTransfer>,
    pub settled_events: usize,
    pub settled_volume: String,
    pub blocked: Option<String>,
}
fn invalid() -> LedgerError {
    LedgerError::Invalid("invalid recurrence input or arithmetic")
}
fn bounds(n: usize) -> Result<(), LedgerError> {
    if !(1..=128).contains(&n) {
        return Err(invalid());
    }
    Ok(())
}
fn total(v: &[u128]) -> Result<u128, LedgerError> {
    v.iter()
        .try_fold(0u128, |a, &b| a.checked_add(b).ok_or(LedgerError::Overflow))
}
fn add_signed(target: &mut i128, delta: i128) -> Result<(), LedgerError> {
    *target = target.checked_add(delta).ok_or(LedgerError::Overflow)?;
    Ok(())
}
fn strings(v: &[u128]) -> Vec<String> {
    v.iter().map(ToString::to_string).collect()
}

/// Repeat the ordered monetary pattern only. The separate physical experiment
/// must pass before this can be interpreted as repeatable production. Renewal
/// funds each unit's proven next-cycle prefix deficit from existing treasury
/// measure. It does not erase surplus or change norms. Stop at the first failure.
pub fn repeat_cash(
    v: &CoupledVersion,
    cycles: usize,
    renew: bool,
) -> Result<CashRecurrence, LedgerError> {
    bounds(cycles)?;
    let saved = coupled::checkpoint(v)?.ok_or_else(invalid)?;
    let e = &saved.evidence;
    if !e.unavailable_events.is_empty()
        || !e.all_scheduled_payments_settled
        || e.events
            .len()
            .checked_mul(cycles)
            .is_none_or(|n| n > 2_000_000)
    {
        return Err(invalid());
    }
    // Re-derive, rather than trust caller-edited events or balance evidence.
    let checked = coupled::preview(v.definition.clone(), e.physical.clone(), vec![])?;
    let checked: CoupledCheckpoint =
        serde_json::from_str(checked.checkpoint.as_ref().ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
    if serde_json::to_value(&checked.evidence).map_err(|_| invalid())?
        != serde_json::to_value(e).map_err(|_| invalid())?
    {
        return Err(invalid());
    }
    let mut balances = v
        .definition
        .policy
        .opening
        .iter()
        .map(|s| s.parse::<u128>().map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    let floors = e
        .bound
        .minimum_opening_by_account
        .iter()
        .map(|s| s.parse::<u128>().map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    let initial_total = total(&balances)?;
    let mut out = CashRecurrence {
        requested_cycles: cycles,
        completed_cycles: 0,
        opening_total: initial_total.to_string(),
        closing: vec![],
        cycle_closings: vec![],
        renewal_transfers: vec![],
        settled_events: 0,
        settled_volume: "0".into(),
        blocked: None,
    };
    let mut volume = 0u128;
    'cycles: for cycle in 0..cycles {
        if cycle > 0 && renew {
            let draws: Vec<_> = floors
                .iter()
                .zip(&balances)
                .enumerate()
                .skip(1)
                .map(|(i, (&floor, &balance))| (i, floor.saturating_sub(balance)))
                .collect();
            let needed = total(&draws.iter().map(|x| x.1).collect::<Vec<_>>())?;
            if balances[0] < needed.checked_add(floors[0]).ok_or(LedgerError::Overflow)? {
                out.blocked = Some(format!(
                    "cycle {cycle}: insufficient existing treasury for renewal"
                ));
                break;
            }
            for (to, amount) in draws.into_iter().filter(|x| x.1 > 0) {
                balances[0] -= amount;
                balances[to] = balances[to]
                    .checked_add(amount)
                    .ok_or(LedgerError::Overflow)?;
                out.renewal_transfers.push(RenewalTransfer {
                    cycle,
                    from: 0,
                    to,
                    amount: amount.to_string(),
                });
            }
        }
        let mut settled = vec![false; e.events.len()];
        for (index, event) in e.events.iter().enumerate() {
            let amount = event.amount.parse::<u128>().map_err(|_| invalid())?;
            if event.requires.is_some_and(|i| !settled[i]) || balances[event.from] < amount {
                out.blocked = Some(format!(
                    "cycle {cycle}, period {}, event {index}: insufficient measure or dependency",
                    event.period
                ));
                break 'cycles;
            }
            let credit = balances[event.to]
                .checked_add(amount)
                .ok_or(LedgerError::Overflow)?;
            balances[event.from] -= amount;
            balances[event.to] = credit;
            volume = volume.checked_add(amount).ok_or(LedgerError::Overflow)?;
            settled[index] = true;
            out.settled_events += 1;
            if total(&balances)? != initial_total {
                return Err(invalid());
            }
        }
        out.completed_cycles += 1;
        out.cycle_closings.push(strings(&balances));
    }
    if total(&balances)? != initial_total {
        return Err(invalid());
    }
    out.closing = strings(&balances);
    out.settled_volume = volume.to_string();
    Ok(out)
}

#[derive(Debug, Serialize)]
pub struct StockDeficit {
    pub resource: usize,
    pub name: String,
    pub first_period: usize,
    pub first_deficit: String,
}

#[derive(Debug, Serialize)]
pub struct PhysicalRecurrence {
    pub cycles: usize,
    pub periods: usize,
    pub feasible: bool,
    pub failure: Option<String>,
    /// Algebraic candidate balances after a failure are not executed stocks.
    pub candidate_cycle_balances: Vec<Vec<i64>>,
    pub negative_stock_diagnostics: Vec<StockDeficit>,
}

/// Extend absolute dates, preserving initial endowment once and original asset
/// lifetimes. Repeat declared external flows and work/demand, not initial stocks.
/// Food ages and constituent checks are rerun across the entire extended graph.
/// Service/demographic profiles require their own extension and are rejected here.
pub fn repeat_physical(
    input: &ModelInput,
    cycles: usize,
) -> Result<PhysicalRecurrence, LedgerError> {
    bounds(cycles)?;
    if input.services.is_some() || input.population.is_some() || !input.dated_generations.is_empty()
    {
        return Err(invalid());
    }
    let source = &input.model;
    let h = &source.seasonal.physical;
    verify_linear(
        &source.compile().map_err(|_| invalid())?,
        &h.vector(&input.plan).map_err(|_| invalid())?,
        None,
    )
    .map_err(|_| invalid())?;
    source.replay(&input.plan).map_err(|_| invalid())?;
    let span = input.plan.len();
    let length = span.checked_mul(cycles).ok_or(LedgerError::Overflow)?;
    if length > 600 || span == 0 {
        return Err(invalid());
    }
    let mut model = source.clone();
    model.seasonal.physical.periods = h.periods.iter().cycle().take(length).cloned().collect();
    model.seasonal.closing_floors = source
        .seasonal
        .closing_floors
        .iter()
        .cycle()
        .take(length)
        .cloned()
        .collect();
    model.seasonal.irrigation_m3 = source.seasonal.irrigation_m3.repeat(cycles);
    for land in &mut model.seasonal.land {
        land.available_hectares = land.available_hectares.repeat(cycles);
    }
    for crop in &mut model.seasonal.crops {
        crop.sowing_periods = (0..cycles)
            .flat_map(|n| crop.sowing_periods.iter().map(move |t| t + n * span))
            .collect();
    }
    let r = h.resources.len();
    let mut stock: Vec<i128> = h.initial.iter().map(|&x| x.into()).collect();
    let mut arrivals = vec![vec![0i128; r]; length];
    let mut expiry = vec![vec![0i128; r]; length];
    for life in &model.lives {
        if life.initial_remaining_periods < length {
            expiry[life.initial_remaining_periods][life.resource] +=
                i128::from(h.initial[life.resource]);
        }
    }
    let mut plan: Vec<PeriodPlan> = Vec::new();
    let mut out = PhysicalRecurrence {
        cycles,
        periods: length,
        feasible: false,
        failure: None,
        candidate_cycle_balances: vec![],
        negative_stock_diagnostics: vec![],
    };
    for t in 0..length {
        let original = &input.plan[t % span];
        let mut row = original.clone();
        for life in &model.lives {
            row.discarded[life.resource] =
                expiry[t][life.resource].try_into().map_err(|_| invalid())?;
        }
        for k in 0..r {
            add_signed(&mut stock[k], arrivals[t][k])?;
            add_signed(&mut stock[k], i128::from(h.periods[t % span].supplies[k]))?;
        }
        for (j, process) in h.processes.iter().enumerate() {
            let x = i128::from(row.activities[j]);
            if x == 0 {
                continue;
            }
            let end = t + process.lead_periods;
            if end >= length {
                return Err(invalid());
            }
            for k in 0..r {
                add_signed(&mut stock[k], -(x * i128::from(process.inputs[k])))?;
                add_signed(&mut arrivals[end][k], x * i128::from(process.outputs[k]))?;
            }
            for life in &model.lives {
                let until = end + 1 + life.service_periods;
                if until < length {
                    add_signed(
                        &mut expiry[until][life.resource],
                        x * i128::from(process.outputs[life.resource]),
                    )?;
                }
            }
        }
        for (k, s) in stock.iter_mut().enumerate() {
            add_signed(
                s,
                -(i128::from(row.deliveries[k]) + i128::from(row.discarded[k])),
            )?;
            if *s < 0
                && !out
                    .negative_stock_diagnostics
                    .iter()
                    .any(|d| d.resource == k)
            {
                out.negative_stock_diagnostics.push(StockDeficit {
                    resource: k,
                    name: h.resources[k].clone(),
                    first_period: t,
                    first_deficit: (-*s).to_string(),
                });
            }
            if *s < 0 && out.failure.is_none() {
                out.failure = Some(format!(
                    "negative stock at absolute period {t}: {} deficit {}",
                    h.resources[k], -*s
                ));
            }
            row.closing[k] = (*s).try_into().map_err(|_| LedgerError::Overflow)?;
        }
        if (t + 1) % span == 0 {
            out.candidate_cycle_balances.push(row.closing.clone());
        }
        plan.push(row);
    }
    let checked = (|| {
        verify_linear(
            &model.compile().map_err(|e| format!("{e:?}"))?,
            &model
                .seasonal
                .physical
                .vector(&plan)
                .map_err(|e| format!("{e:?}"))?,
            None,
        )
        .map_err(|e| format!("{e:?}"))?;
        model.replay(&plan).map_err(|e| format!("{e:?}"))?;
        if let Some(routes) = &input.food_routes {
            routes
                .audit(&model, &plan)
                .map_err(|e| format!("food: {e:?}"))?;
        }
        for basis in &input.material_bases {
            basis
                .audit(&model.seasonal.physical, &plan)
                .map_err(|e| format!("material: {e:?}"))?;
        }
        Ok::<_, String>(())
    })();
    match checked {
        Ok(()) => {
            out.feasible = true;
        }
        Err(reason) => {
            if out.failure.is_none() {
                out.failure = Some(reason);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        coupled::CoupledDefinition,
        horizon::{Period, Process},
        model_scenarios::{self, ModelDefinition, ModelResource},
    };
    use uuid::Uuid;
    fn physical() -> ModelInput {
        let mut input: ModelInput =
            serde_json::from_str(include_str!("../fixtures/model-scenario.json")).unwrap();
        let m = &mut input.model;
        let h = &mut m.seasonal.physical;
        h.initial = vec![10, 1, 1];
        h.processes = vec![
            Process {
                inputs: vec![1, 0, 0],
                outputs: vec![0, 0, 2],
                lead_periods: 1,
            },
            Process {
                inputs: vec![1, 0, 0],
                outputs: vec![0, 1, 0],
                lead_periods: 1,
            },
        ];
        h.periods = vec![
            Period {
                supplies: vec![0; 3],
                needs: vec![0, 0, 1],
                capacities: vec![1, 1]
            };
            2
        ];
        m.seasonal.land[0].available_hectares = vec![0; 2];
        m.seasonal.irrigation_m3 = vec![0; 2];
        m.seasonal.closing_floors = vec![vec![0; 3]; 2];
        m.limits[0].activity_use = vec![1, 0];
        m.lives[0].service_periods = 2;
        m.lives[0].initial_remaining_periods = 2;
        input.plan = vec![
            PeriodPlan {
                activities: vec![1, 1],
                deliveries: vec![0, 0, 1],
                closing: vec![8, 1, 0],
                discarded: vec![0; 3],
            },
            PeriodPlan {
                activities: vec![0, 0],
                deliveries: vec![0, 0, 1],
                closing: vec![8, 2, 1],
                discarded: vec![0; 3],
            },
        ];
        input.material_bases.clear();
        input.boundary = 0;
        input
    }
    fn money() -> CoupledVersion {
        let input = physical();
        let definition = ModelDefinition {
            id: Uuid::nil(),
            format_version: 1,
            resources: input
                .model
                .seasonal
                .physical
                .resources
                .iter()
                .map(|name| ModelResource {
                    name: name.clone(),
                    unit: "test units".into(),
                })
                .collect(),
            period_unit: "day".into(),
            time_origin: "zero".into(),
            dated_bindings: vec![],
            basis: "recurrence test".into(),
        };
        let p = model_scenarios::preview(definition, input).unwrap();
        let policy =
            serde_json::from_str(include_str!("../fixtures/circulation-policy.json")).unwrap();
        coupled::preview(
            CoupledDefinition {
                id: Uuid::nil(),
                format_version: 1,
                model_id: p.definition.id,
                policy,
                basis: "test".into(),
            },
            p,
            vec![],
        )
        .unwrap()
    }
    #[test]
    fn finite_endowment_and_original_retirement_dates_survive_six_cycles() {
        let input = physical();
        for n in 1..=5 {
            let r = repeat_physical(&input, n).unwrap();
            assert!(r.feasible, "{:?}", r.failure);
            assert_eq!(
                r.candidate_cycle_balances.last().unwrap(),
                &vec![10 - 2 * n as i64, 2, 1]
            );
        }
        let failed = repeat_physical(&input, 6).unwrap();
        assert!(!failed.feasible);
        assert!(
            failed
                .failure
                .unwrap()
                .contains("absolute period 10: material deficit 2")
        );
        // Restarting each cycle from the initial vector falsely passes all six.
        assert!((0..6).all(|_| repeat_physical(&input, 1).unwrap().feasible));
    }
    #[test]
    fn no_replacement_cannot_rejuvenate_initial_equipment() {
        let mut input = physical();
        input.plan[0].activities[1] = 0;
        input.plan[0].closing[0] = 9;
        input.plan[1].closing = vec![9, 1, 1];
        assert!(repeat_physical(&input, 1).unwrap().feasible);
        assert!(!repeat_physical(&input, 2).unwrap().feasible);
    }
    #[test]
    fn food_expiry_is_not_reset_at_cycle_boundary() {
        use crate::food_routes::FoodRoutes;
        let mut input = physical();
        input.model.seasonal.physical.initial[2] = 8;
        input.model.seasonal.physical.processes[0].outputs = vec![0, 1, 0];
        input.plan[0].activities[0] = 0;
        input.plan[0].closing = vec![9, 1, 7];
        input.plan[1].closing = vec![9, 2, 6];
        input.food_routes = Some(FoodRoutes {
            resources: vec![2],
            initial_remaining_periods: vec![3],
            harvests: vec![],
            transfers: vec![],
            unit: "units".into(),
            basis: "expiry fixture".into(),
        });
        let first = repeat_physical(&input, 1).unwrap();
        assert!(first.feasible, "{:?}", first.failure);
        assert!(!repeat_physical(&input, 2).unwrap().feasible);
    }
    #[test]
    fn independent_event_oracle_matches_carried_balances_and_failure_prefix() {
        let v = money();
        let c = coupled::checkpoint(&v).unwrap().unwrap();
        for cycles in 1..=16 {
            let mut b: Vec<i128> = v
                .definition
                .policy
                .opening
                .iter()
                .map(|s| s.parse().unwrap())
                .collect();
            let mut count = 0;
            let mut volume = 0i128;
            'outer: for _ in 0..cycles {
                for e in &c.evidence.events {
                    let amount: i128 = e.amount.parse().unwrap();
                    if b[e.from] < amount {
                        break 'outer;
                    }
                    b[e.from] -= amount;
                    b[e.to] += amount;
                    count += 1;
                    volume += amount;
                }
            }
            let result = repeat_cash(&v, cycles, false).unwrap();
            assert_eq!(
                result.closing,
                b.iter().map(ToString::to_string).collect::<Vec<_>>()
            );
            assert_eq!(result.settled_events, count);
            assert_eq!(result.settled_volume, volume.to_string());
            assert_eq!(b.iter().sum::<i128>().to_string(), result.opening_total);
        }
    }
    #[test]
    fn renewal_circulates_existing_measure_preserves_surplus_and_every_account() {
        let base = money();
        let mut d = base.definition.clone();
        d.policy.common_measure = d.policy.unit_measure.clone();
        let physical = coupled::checkpoint(&base)
            .unwrap()
            .unwrap()
            .evidence
            .physical;
        let v = coupled::preview(d, physical, vec![]).unwrap();
        let r = repeat_cash(&v, 64, true).unwrap();
        assert_eq!(r.completed_cycles, 64);
        assert!(r.blocked.is_none());
        let initial: u128 = r.opening_total.parse().unwrap();
        for b in &r.cycle_closings {
            assert_eq!(
                b.iter().map(|x| x.parse::<u128>().unwrap()).sum::<u128>(),
                initial
            );
            assert_eq!(b[1], "0");
        }
        let c = coupled::checkpoint(&v).unwrap().unwrap();
        let mut b: Vec<u128> = v
            .definition
            .policy
            .opening
            .iter()
            .map(|x| x.parse().unwrap())
            .collect();
        for cycle in 0..64 {
            for t in r.renewal_transfers.iter().filter(|t| t.cycle == cycle) {
                let amount: u128 = t.amount.parse().unwrap();
                assert_eq!(t.from, 0);
                b[t.from] -= amount;
                b[t.to] += amount;
            }
            for e in &c.evidence.events {
                let amount: u128 = e.amount.parse().unwrap();
                b[e.from] -= amount;
                b[e.to] += amount;
            }
            assert_eq!(strings(&b), r.cycle_closings[cycle]);
        }
    }
    #[test]
    fn retained_unit_surplus_eventually_exhausts_treasury_without_any_issuance() {
        let v = money();
        let r = repeat_cash(&v, 64, true).unwrap();
        assert_eq!(r.completed_cycles, 49);
        assert!(
            r.blocked
                .as_ref()
                .unwrap()
                .contains("insufficient existing treasury")
        );
        assert_eq!(
            total(
                &r.closing
                    .iter()
                    .map(|s| s.parse().unwrap())
                    .collect::<Vec<_>>()
            )
            .unwrap()
            .to_string(),
            r.opening_total
        );
        assert!(r.closing[4].parse::<u128>().unwrap() > 30);
    }
    #[test]
    fn altered_events_or_balances_are_not_trusted_and_limits_are_enforced() {
        let v = money();
        for mode in 0..3 {
            let mut bad = v.clone();
            let mut c = coupled::checkpoint(&v).unwrap().unwrap();
            match mode {
                0 => c.evidence.events[0].amount = "999".into(),
                1 => c.evidence.money.closing[0] = "999".into(),
                _ => c.evidence.bound.minimum_opening_by_account[0] = "0".into(),
            };
            bad.checkpoint = Some(serde_json::to_string(&c).unwrap());
            assert!(repeat_cash(&bad, 2, true).is_err());
        }
        assert!(repeat_cash(&v, 0, false).is_err());
        assert!(repeat_cash(&v, 129, false).is_err());
        assert!(repeat_physical(&physical(), 0).is_err());
        let mut bad = physical();
        bad.plan[0].closing[0] += 1;
        assert!(repeat_physical(&bad, 2).is_err());
    }
}
