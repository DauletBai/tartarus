//! Checked identity-preserving food routes derived from a physical plan.
//! Shelf lives are declared assumptions, never certification of food safety.
use crate::{
    capital::CapitalCycle,
    dated::DatedFlow,
    horizon::PeriodPlan,
    mathematics::{MathError, verify_linear},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HarvestRoute {
    pub process: usize,
    pub shelf_periods: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FoodRoutes {
    pub resources: Vec<usize>,
    pub initial_remaining_periods: Vec<u32>,
    pub harvests: Vec<HarvestRoute>,
    pub transfers: Vec<usize>,
    pub unit: String,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoodRouteEvidence {
    pub periods_verified: usize,
    pub seed_withdrawn: String,
    pub resident_delivered: String,
    pub expired: String,
    pub final_stock: Vec<String>,
    pub final_in_transit: String,
    /// Canonical identity state, not a client-supplied authoritative checkpoint.
    pub final_checkpoint: String,
}
fn invalid() -> MathError {
    MathError::Invalid("food route mapping or physical/dated mismatch")
}
fn quantity(a: i64, b: i64) -> Result<u64, MathError> {
    u64::try_from(i128::from(a) * i128::from(b)).map_err(|_| MathError::Overflow)
}
impl FoodRoutes {
    pub fn audit(
        &self,
        model: &CapitalCycle,
        plan: &[PeriodPlan],
    ) -> Result<FoodRouteEvidence, MathError> {
        let h = &model.seasonal.physical;
        verify_linear(&h.compile()?, &h.vector(plan)?, None)?;
        if self.resources.is_empty()
            || self.resources.len() > 32
            || self.resources.len() != self.initial_remaining_periods.len()
            || self.initial_remaining_periods.contains(&0)
            || self.unit.trim().is_empty()
            || self.unit.len() > 128
            || self.basis.trim().is_empty()
            || self.basis.len() > 1000
            || self.harvests.len() + self.transfers.len() > h.processes.len()
        {
            return Err(invalid());
        }
        let positions: BTreeMap<_, _> = self
            .resources
            .iter()
            .enumerate()
            .map(|(i, &r)| (r, i))
            .collect();
        if positions.len() != self.resources.len()
            || self.resources.iter().any(|&r| r >= h.resources.len())
        {
            return Err(invalid());
        }
        let mut rules = BTreeMap::new();
        let crops: BTreeSet<_> = model.seasonal.crops.iter().map(|c| c.process).collect();
        for x in &self.harvests {
            if x.shelf_periods == 0
                || !crops.contains(&x.process)
                || rules.insert(x.process, Some(x.shelf_periods)).is_some()
            {
                return Err(invalid());
            }
        }
        for &j in &self.transfers {
            if rules.insert(j, None).is_some() {
                return Err(invalid());
            }
        }
        if rules.keys().any(|&j| j >= h.processes.len()) {
            return Err(invalid());
        }
        let mut mappings = vec![];
        for (j, p) in h.processes.iter().enumerate() {
            let ins: Vec<_> = self
                .resources
                .iter()
                .copied()
                .filter(|&r| p.inputs[r] != 0)
                .collect();
            let outs: Vec<_> = self
                .resources
                .iter()
                .copied()
                .filter(|&r| p.outputs[r] != 0)
                .collect();
            if ins.is_empty() && outs.is_empty() {
                if rules.contains_key(&j) {
                    return Err(invalid());
                }
                continue;
            }
            let rule = rules.get(&j).ok_or_else(invalid)?;
            if ins.len() != 1 || outs.len() != 1 || p.lead_periods == 0 {
                return Err(invalid());
            }
            let (a, b) = (ins[0], outs[0]);
            match rule {
                Some(_) if a == b && p.inputs[a] > 0 && p.outputs[b] > p.inputs[a] => {}
                None if a != b && p.inputs[a] == 1 && p.outputs[b] == 1 => {}
                _ => return Err(invalid()),
            }
            mappings.push((j, a, b, *rule));
        }
        let mut flow = DatedFlow::new("physical-food-routes-v1", &self.unit, self.resources.len())?;
        let mut seed = 0u128;
        let mut delivered = 0u128;
        let mut origin = 0u64;
        let mut previous_expired = 0u128;
        for (t, row) in plan.iter().enumerate() {
            let period = u32::try_from(t).map_err(|_| MathError::Overflow)?;
            flow.advance(period)?;
            if t == 0 {
                for (i, &r) in self.resources.iter().enumerate() {
                    if h.initial[r] > 0 {
                        flow.introduce(
                            origin,
                            i,
                            quantity(h.initial[r], 1)?,
                            0,
                            self.initial_remaining_periods[i],
                            "Declared opening preserved stock age",
                        )?;
                        origin = origin.checked_add(1).ok_or(MathError::Overflow)?;
                    }
                }
            }
            for &r in &self.resources {
                if h.periods[t].supplies[r] != 0 {
                    return Err(invalid());
                }
            }
            for &(j, a, b, shelf) in &mappings {
                let p = &h.processes[j];
                let q = quantity(row.activities[j], p.inputs[a])?;
                if q == 0 {
                    continue;
                }
                let due = u32::try_from(t.checked_add(p.lead_periods).ok_or(MathError::Overflow)?)
                    .map_err(|_| MathError::Overflow)?;
                if let Some(life) = shelf {
                    flow.withdraw(positions[&a], q)?;
                    seed = seed.checked_add(u128::from(q)).ok_or(MathError::Overflow)?;
                    flow.introduce(
                        origin,
                        positions[&b],
                        quantity(row.activities[j], p.outputs[b])?,
                        due,
                        due.checked_add(life).ok_or(MathError::Overflow)?,
                        &format!("Declared crop {j} committed at period {t}"),
                    )?;
                    origin = origin.checked_add(1).ok_or(MathError::Overflow)?;
                } else {
                    flow.transfer(positions[&a], positions[&b], q, due)?;
                }
            }
            for (i, &r) in self.resources.iter().enumerate() {
                let q = quantity(row.deliveries[r], 1)?;
                if q > 0 {
                    flow.withdraw(i, q)?;
                    delivered = delivered
                        .checked_add(u128::from(q))
                        .ok_or(MathError::Overflow)?;
                }
            }
            let balance = flow.balance();
            let discarded = self.resources.iter().try_fold(0u128, |sum, &r| {
                sum.checked_add(u128::from(quantity(row.discarded[r], 1)?))
                    .ok_or(MathError::Overflow)
            })?;
            if balance.expired - previous_expired != discarded {
                return Err(MathError::Infeasible(format!(
                    "food expiry/discard mismatch at period {t}"
                )));
            }
            previous_expired = balance.expired;
            for (i, &r) in self.resources.iter().enumerate() {
                if balance.stock_by_resource[i] != u128::from(quantity(row.closing[r], 1)?) {
                    return Err(MathError::Infeasible(format!(
                        "food stock mismatch at period {t}, resource {r}"
                    )));
                }
            }
        }
        let balance = flow.balance();
        Ok(FoodRouteEvidence {
            periods_verified: plan.len(),
            seed_withdrawn: seed.to_string(),
            resident_delivered: delivered.to_string(),
            expired: balance.expired.to_string(),
            final_stock: balance
                .stock_by_resource
                .iter()
                .map(ToString::to_string)
                .collect(),
            final_in_transit: balance.in_transit.to_string(),
            final_checkpoint: String::from_utf8(flow.checkpoint()?).map_err(|_| invalid())?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_scenarios::ModelInput;
    fn fixture() -> ModelInput {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/extended-scenario.json")).unwrap();
        serde_json::from_value(v["input"].clone()).unwrap()
    }
    #[test]
    fn all_food_stocks_reconcile_with_origin_preserving_five_year_replay() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/extended-scenario.json")).unwrap();
        let x = fixture();
        let e = x
            .food_routes
            .as_ref()
            .unwrap()
            .audit(&x.model, &x.plan)
            .unwrap();
        assert_eq!(serde_json::to_value(&e).unwrap(), v["reference"]["food"]);
        assert_eq!(e.periods_verified, 60);
        assert_eq!(e.expired, "0");
        let f = DatedFlow::restore(
            e.final_checkpoint.as_bytes(),
            e.final_checkpoint.as_bytes(),
            "physical-food-routes-v1",
            serde_json::from_str::<serde_json::Value>(&e.final_checkpoint).unwrap()["revision"]
                .as_u64()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            f.balance().withdrawn,
            e.seed_withdrawn.parse::<u128>().unwrap()
                + e.resident_delivered.parse::<u128>().unwrap()
        );
    }
    #[test]
    fn every_touched_food_process_requires_an_unambiguous_rule() {
        let x = fixture();
        for mode in 0..4 {
            let mut r = x.food_routes.clone().unwrap();
            match mode {
                0 => {
                    r.transfers.remove(0);
                }
                1 => r.transfers.push(15),
                2 => r.transfers.push(999),
                _ => r.resources.push(0),
            }
            assert!(r.audit(&x.model, &x.plan).is_err());
        }
    }
    #[test]
    fn processing_cannot_be_relabelled_as_harvest_to_reset_expiry() {
        let x = fixture();
        let mut r = x.food_routes.clone().unwrap();
        r.transfers.remove(0);
        r.harvests.push(HarvestRoute {
            process: 15,
            shelf_periods: 999,
        });
        assert!(r.audit(&x.model, &x.plan).is_err());
    }
    #[test]
    fn short_life_rejects_the_unchanged_quantity_plan() {
        let x = fixture();
        let mut r = x.food_routes.clone().unwrap();
        r.initial_remaining_periods = vec![1; 9];
        for h in &mut r.harvests {
            h.shelf_periods = 1;
        }
        assert!(r.audit(&x.model, &x.plan).is_err());
    }
    #[test]
    fn undeclared_external_food_and_changed_balances_cannot_enter_identity_trace() {
        let mut x = fixture();
        x.model.seasonal.physical.periods[0].supplies[0] += 1;
        for row in &mut x.plan {
            row.closing[0] += 1;
        }
        assert!(
            x.food_routes
                .as_ref()
                .unwrap()
                .audit(&x.model, &x.plan)
                .is_err()
        );
        let mut x = fixture();
        x.plan[20].closing[6] += 1;
        assert!(
            x.food_routes
                .as_ref()
                .unwrap()
                .audit(&x.model, &x.plan)
                .is_err()
        );
    }
    #[test]
    fn package_mass_and_finite_deposit_have_no_hidden_growth() {
        let x = fixture();
        let b = &x.material_bases[0];
        let h = &x.model.seasonal.physical;
        let e = b.audit(h, &x.plan).unwrap();
        assert_eq!(
            e.initial + e.supplied,
            e.delivered + e.discarded + e.final_stock
        );
        assert!(e.work_in_progress.iter().any(|&q| q > 0));
        assert_eq!(h.initial[27] - x.plan[59].closing[27], 580_000_000);
        let mut forged = h.clone();
        forged.processes[43].outputs[28] += 1;
        assert!(b.validate(&forged).is_err());
    }
}
