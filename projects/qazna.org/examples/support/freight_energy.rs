//! Supply the November collection schedule from dated fuel and power assets.
use super::{MONTHS, ModelInput, Month, RollingSnapshot, YEAR, calendar, loads};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct Slot {
    demand: i64,
    work: i64,
    extract: i64,
    generate: i64,
}

struct Supply {
    energy: i64,
    fuel: i64,
    deposit: i64,
    fuel_floor: i64,
    energy_store: i64,
    fuel_store: i64,
    yield_ticks: i64,
    slots: Vec<Slot>,
}

#[derive(Serialize)]
struct Flow {
    month: String,
    demand: i64,
    extracted: i64,
    generated_batches: i64,
    fuel_arrival: i64,
    energy_arrival: i64,
    closing_fuel: i64,
    closing_deposit: i64,
    closing_energy: i64,
    storage_loss: i64,
    unmet_energy: i64,
}

impl Supply {
    // G batches require max(0,G-free_fuel) single-tick extraction starts.
    // Net output strictly increases when yield > 1. Integer rounding is upward.
    fn targets(&self) -> (i64, i64) {
        assert!(self.yield_ticks > 1 && self.fuel >= self.fuel_floor);
        let free = self.fuel - self.fuel_floor;
        let deficit = (self.slots.iter().map(|s| s.demand).sum::<i64>() - self.energy).max(0);
        let batches = loads(deficit, self.yield_ticks)
            .max(loads((deficit - free).max(0), self.yield_ticks - 1));
        (batches, (batches - free).max(0))
    }

    // Both processes deliver one month later. Protect the fuel reserve even
    // within the month and bound storage at receipt, before next month's use.
    fn replay(&self, start: usize, outage: Option<usize>) -> Vec<Flow> {
        self.dispatch(start, outage, 0, self.targets().0)
    }

    fn dispatch(
        &self,
        start: usize,
        outage: Option<usize>,
        per_thousand: i64,
        target_batches: i64,
    ) -> Vec<Flow> {
        assert!((0..=1000).contains(&per_thousand));
        let mut batches = target_batches;
        let mut extraction = (batches - (self.fuel - self.fuel_floor)).max(0);
        let (mut energy, mut fuel, mut deposit) = (self.energy, self.fuel, self.deposit);
        let (mut fuel_due, mut energy_due) = (0, 0);
        let mut rows = Vec::new();
        for (t, s) in self.slots.iter().enumerate() {
            // Only stock carried from the previous close incurs this month's loss.
            let storage_loss = energy * per_thousand / 1000;
            energy -= storage_loss;
            fuel += fuel_due;
            energy += energy_due;
            assert!(fuel <= self.fuel_store && energy <= self.energy_store);
            let unmet = (s.demand - energy).max(0);
            energy -= s.demand - unmet;
            let gen_cap = if t >= start && t + 1 < self.slots.len() && outage != Some(t) {
                s.generate
            } else {
                0
            };
            let generation = gen_cap
                .min(batches)
                .min(fuel - self.fuel_floor)
                .min(s.work)
                .min((self.energy_store - energy) / self.yield_ticks);
            fuel -= generation;
            batches -= generation;
            let mine_cap = if t >= start && t + 2 < self.slots.len() {
                s.extract
            } else {
                0
            };
            let mine = mine_cap
                .min(extraction)
                .min(deposit)
                .min(energy)
                .min(s.work - generation)
                .min(self.fuel_store - fuel);
            extraction -= mine;
            deposit -= mine;
            energy -= mine;
            rows.push(Flow {
                month: calendar(MONTHS + t),
                demand: s.demand,
                extracted: mine,
                generated_batches: generation,
                fuel_arrival: fuel_due,
                energy_arrival: energy_due,
                closing_fuel: fuel,
                closing_deposit: deposit,
                closing_energy: energy,
                storage_loss,
                unmet_energy: unmet,
            });
            fuel_due = mine;
            energy_due = generation * self.yield_ticks;
        }
        rows
    }

    fn latest(&self, outage: Option<usize>) -> Option<usize> {
        (0..self.slots.len())
            .rev()
            .find(|&t| self.replay(t, outage).iter().all(|r| r.unmet_energy == 0))
    }
}

fn shortage(rows: &[Flow]) -> Value {
    json!({"first":rows.iter().find(|r|r.unmet_energy>0).map(|r|&r.month),
        "unserved_ticks":rows.iter().map(|r|r.unmet_energy).sum::<i64>()})
}

pub(super) fn study(input: &ModelInput, boundary: &RollingSnapshot, route: &[Month]) -> Value {
    let m = &input.model;
    let h = &m.seasonal.physical;
    let opening = &boundary.opening_stock;
    let limit = |name| m.limits.iter().find(|l| l.name == name).unwrap();
    let work = limit("shared_human_work_ticks");
    let extractor = limit("shared_extraction");
    let power = limit("fuel_generation");
    // Fail loudly if the reference recipe acquires a coproduct or another input.
    let terms = |v: &[i64]| {
        v.iter()
            .copied()
            .enumerate()
            .filter(|&(_, q)| q != 0)
            .collect::<Vec<_>>()
    };
    let mine = &h.processes[30];
    let generate = &h.processes[31];
    assert_eq!((mine.lead_periods, generate.lead_periods), (1, 1));
    assert_eq!(terms(&mine.inputs), [(9, 1), (18, 1)]);
    assert_eq!(terms(&mine.outputs), [(20, 1)]);
    assert_eq!(terms(&generate.inputs), [(20, 1)]);
    let yield_ticks = generate.outputs[9];
    assert_eq!(terms(&generate.outputs), [(9, yield_ticks)]);
    for l in &m.limits {
        let uses = (l.activity_use[30], l.activity_use[31]);
        let expected = match l.name.as_str() {
            "shared_human_work_ticks" => (1, 1),
            "shared_extraction" => (1, 0),
            "fuel_generation" => (0, 1),
            _ => (0, 0),
        };
        assert_eq!(uses, expected);
    }
    let active = |r, t| {
        boundary
            .opening_cohorts
            .iter()
            .filter(|c| c.resource == r && c.expires > MONTHS + t)
            .map(|c| c.quantity)
            .sum::<i64>()
    };
    let slots = route
        .iter()
        .enumerate()
        .map(|(t, row)| {
            let f = row.freight.as_ref().unwrap();
            let p = &h.periods[MONTHS - YEAR + t % YEAR];
            let remaining = opening[work.resource] * work.units_per_stock
                - f.transport_work_ticks
                - row.shared_work_and_factory_slots;
            assert!(remaining >= 0);
            Slot {
                demand: row.energy_used_ticks + f.dispatch_energy_ticks + f.return_energy_ticks,
                work: remaining,
                extract: p.capacities[30]
                    .min(active(extractor.resource, t) * extractor.units_per_stock),
                generate: p.capacities[31].min(active(power.resource, t) * power.units_per_stock),
            }
        })
        .collect();
    let supply = Supply {
        energy: opening[9],
        fuel: opening[20],
        deposit: opening[18],
        fuel_floor: m.seasonal.closing_floors.last().unwrap()[20],
        energy_store: h.storage_caps[9],
        fuel_store: h.storage_caps[20],
        yield_ticks,
        slots,
    };
    let start = supply
        .latest(None)
        .expect("existing generators must serve the reference route");
    let success = supply.replay(start, None);
    let baseline = supply.replay(route.len(), None);
    assert!(baseline.iter().zip(route).all(|(r,m)| r.unmet_energy==m.freight.as_ref().unwrap().extra_dispatch_energy_ticks));
    let (batches, extraction) = supply.targets();
    assert_eq!(
        success.iter().map(|r| r.generated_batches).sum::<i64>(),
        batches
    );
    assert_eq!(success.iter().map(|r| r.extracted).sum::<i64>(), extraction);
    let demand: i64 = success.iter().map(|r| r.demand).sum();
    let last = success.last().unwrap();
    assert_eq!(
        supply.energy + batches * yield_ticks,
        demand + extraction + last.closing_energy
    );
    assert_eq!(supply.fuel + extraction, batches + last.closing_fuel);
    assert_eq!(supply.deposit, extraction + last.closing_deposit);
    // Lose the busiest generation month of the last feasible launch.
    let peak = success
        .iter()
        .enumerate()
        .max_by_key(|(_, r)| r.generated_batches)
        .unwrap()
        .0;
    let repaired = supply.latest(Some(peak));
    let lossy = supply.dispatch(start, None, 1, batches);
    let expanded = batches + loads(batches, 10);
    let (loss_start, protected) = (0..route.len())
        .rev()
        .find_map(|t| {
            let rows = supply.dispatch(t, None, 1, expanded);
            (rows.iter().all(|r| r.unmet_energy == 0)
                && rows.iter().map(|r| r.generated_batches).sum::<i64>() == expanded)
                .then_some((t, rows))
        })
        .expect("additional finite fuel must cover the storage stress");
    let lost: i64 = protected.iter().map(|r| r.storage_loss).sum();
    let mined: i64 = protected.iter().map(|r| r.extracted).sum();
    let end = protected.last().unwrap();
    assert_eq!(
        supply.energy + expanded * yield_ticks,
        demand + mined + lost + end.closing_energy
    );
    assert_eq!(supply.fuel + mined, expanded + end.closing_fuel);
    assert_eq!(supply.deposit, mined + end.closing_deposit);
    let final_generation = success
        .iter()
        .rposition(|r| r.generated_batches > 0)
        .unwrap();
    json!({"scope":"Unchanged grain and wrapping orders, same work pool, no new modules or outside energy. Energy inventory carries without loss. Generation and extraction outputs arrive next month. Start is the latest under this dispatch policy.",
        "opening_energy_ticks":supply.energy,"opening_fuel_ticks":supply.fuel,"opening_deposit_ticks":supply.deposit,
        "protected_fuel_ticks":supply.fuel_floor,"load_ticks":demand,"energy_per_batch":yield_ticks,
        "generation_batches":batches,"extraction_ticks":extraction,"latest_start":calendar(MONTHS+start),
        "last_generation_start":success[final_generation].month,"final_energy_arrival":success[final_generation+1].month,
        "without_production":shortage(&baseline),"one_month_later":shortage(&supply.replay(start+1,None)),
        "storage_loss_stress":{"per_thousand_each_month":1,"unchanged_production_targets":true,
            "lost_ticks":lossy.iter().map(|r|r.storage_loss).sum::<i64>(),"shortage":shortage(&lossy),
            "replanned":{"generation_batches":expanded,"extraction_ticks":mined,"latest_start_for_full_target":calendar(MONTHS+loss_start),
                "lost_ticks":lost,"closing_energy_ticks":end.closing_energy,"shortage":shortage(&protected),"months":protected}},
        "generator_outage":{"month":success[peak].month,"original_start":shortage(&supply.replay(start,Some(peak))),
            "latest_replanned_start":repaired.map(|t|calendar(MONTHS+t))},
        "peak_pre_dispatch_energy_ticks":success.iter().map(|r|r.closing_energy+r.extracted+r.demand).max(),
        "months":success})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lead_times_reserve_and_integer_boundary() {
        let mut s = Supply {
            energy: 2,
            fuel: 3,
            fuel_floor: 3,
            deposit: 1,
            energy_store: 20,
            fuel_store: 4,
            yield_ticks: 10,
            slots: vec![
                Slot {
                    demand: 0,
                    work: 1,
                    extract: 1,
                    generate: 1
                };
                3
            ],
        };
        s.slots[2].demand = 11;
        assert_eq!(s.targets(), (1, 1));
        let rows = s.replay(0, None);
        assert_eq!(
            (
                rows[0].generated_batches,
                rows[1].generated_batches,
                rows[2].energy_arrival
            ),
            (0, 1, 10)
        );
        assert_eq!(rows[2].closing_energy, 0);
        assert_eq!(s.latest(None), Some(0));
        assert_eq!(s.latest(Some(1)), None);
        s.slots[2].demand += 1;
        assert_eq!(s.targets(), (2, 2));
        assert_eq!(s.replay(0, None)[2].unmet_energy, 1);
    }

    #[test]
    fn only_carried_energy_loses_charge_before_the_next_receipt() {
        let mut s = Supply {
            energy: 1_000,
            fuel: 1,
            fuel_floor: 0,
            deposit: 0,
            energy_store: 2_000,
            fuel_store: 1,
            yield_ticks: 1_000,
            slots: vec![
                Slot {
                    demand: 0,
                    work: 1,
                    extract: 0,
                    generate: 1
                };
                3
            ],
        };
        s.slots[2].demand = 2_000;
        let rows = s.dispatch(0, None, 1, 1);
        assert_eq!(
            (
                rows[0].storage_loss,
                rows[1].storage_loss,
                rows[2].storage_loss
            ),
            (1, 0, 1)
        );
        assert_eq!(rows[1].energy_arrival, 1_000);
        assert_eq!(rows[2].unmet_energy, 2);
    }

    #[test]
    fn storage_and_shared_work_can_block_generation() {
        let mut s = Supply {
            energy: 0,
            fuel: 3,
            fuel_floor: 1,
            deposit: 0,
            energy_store: 9,
            fuel_store: 3,
            yield_ticks: 10,
            slots: vec![
                Slot {
                    demand: 0,
                    work: 1,
                    extract: 0,
                    generate: 2
                };
                2
            ],
        };
        s.slots[1].demand = 20;
        assert_eq!(s.replay(0, None)[1].unmet_energy, 20);
        s.energy_store = 20;
        assert_eq!(s.replay(0, None)[1].unmet_energy, 10);
        s.slots[0].work = 2;
        assert_eq!(s.replay(0, None)[1].unmet_energy, 0);
    }
}
