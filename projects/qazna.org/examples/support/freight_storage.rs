//! Compare two storage media at identical orders, terminal reserves and loss rate.
use super::*;
use qazna::coupled::CirculationPolicy;
use qazna::mathematics::{Movement, ScheduledMovement, replay_schedule, schedule_opening};

struct Build {
    start: usize,
    completion: usize,
    active: std::ops::Range<usize>,
    units: i64,
    energy: i64,
    material: i64,
    work: i64,
    throughput: i64,
}

// Smallest prior stock whose surviving amount reaches the requested integer.
fn before_loss(required: i64) -> i64 {
    if required == 0 {
        0
    } else {
        required + (required - 1) / 999
    }
}
fn carried(q: i64) -> i64 {
    q - q / 1000
}

fn late_plan(s: &Supply, b: &Build, caps: &[i64], target: i64, mining: i64) -> Option<Vec<Flow>> {
    let n = s.slots.len();
    let mut mines = vec![0; n];
    let mut remaining = mining;
    for (t, slot) in s.slots.iter().enumerate().rev() {
        if t + 1 >= b.active.start {
            continue;
        }
        let work = slot.work - if t == b.start { b.work * b.units } else { 0 };
        mines[t] = remaining.min(slot.extract).min(work);
        if mines[t] < 0 {
            return None;
        }
        remaining -= mines[t];
    }
    if remaining > 0 || mining > s.deposit {
        return None;
    }
    let mut need = vec![0; n];
    let mut capacities = vec![0; n];
    for t in (0..n).rev() {
        let build = if t == b.start { b.units } else { 0 };
        let work = s.slots[t].work - mines[t] - build * b.work;
        if work < 0 {
            return None;
        }
        if b.active.contains(&t) && t + 1 < n {
            capacities[t] = (b.units * b.throughput).min(caps[t]).min(work);
        }
        let keep = if t + 1 == n {
            target
        } else {
            before_loss((need[t + 1] - capacities[t] * s.yield_ticks).max(0))
        };
        need[t] = s.slots[t].demand + mines[t] + build * b.energy + keep;
    }
    if carried(s.energy) < need[0] {
        return None;
    }
    let (mut energy, mut fuel, mut deposit) = (s.energy, s.fuel, s.deposit);
    let (mut fuel_due, mut energy_due) = (0, 0);
    let mut rows = Vec::new();
    for (t, slot) in s.slots.iter().enumerate() {
        let loss = energy - carried(energy);
        energy += energy_due - loss;
        fuel += fuel_due;
        if energy > s.energy_store || fuel > s.fuel_store {
            return None;
        }
        let build_energy = if t == b.start { b.units * b.energy } else { 0 };
        let demand = slot.demand + build_energy;
        energy -= demand + mines[t];
        if energy < 0 {
            return None;
        }
        let batches = if t + 1 == n {
            0
        } else {
            loads((need[t + 1] - carried(energy)).max(0), s.yield_ticks)
        };
        if batches > capacities[t] || batches > fuel - s.fuel_floor {
            return None;
        }
        fuel -= batches;
        deposit -= mines[t];
        rows.push(Flow {
            month: calendar(MONTHS + t),
            demand,
            extracted: mines[t],
            generated_batches: batches,
            energy_arrival: energy_due,
            fuel_arrival: fuel_due,
            closing_energy: energy,
            closing_fuel: fuel,
            closing_deposit: deposit,
            storage_loss: loss,
            unmet_energy: 0,
        });
        energy_due = batches * s.yield_ticks;
        fuel_due = mines[t];
    }
    assert!(energy >= target && energy_due == 0 && fuel_due == 0);
    Some(rows)
}

fn metrics(rows: &[Flow], build: &Build, units: i64) -> Value {
    let mine: i64 = rows.iter().map(|r| r.extracted).sum();
    let gen_count: i64 = rows.iter().map(|r| r.generated_batches).sum();
    json!({"extraction":mine,"generation":gen_count,"storage_loss":rows.iter().map(|r|r.storage_loss).sum::<i64>(),
        "upstream_work_ticks":mine+gen_count+units*build.work,"construction_material":units*build.material,
        "construction_energy":units*build.energy,"modules":units,"closing_energy":rows.last().unwrap().closing_energy,
        "closing_fuel":rows.last().unwrap().closing_fuel,"peak_fuel_before_generation":rows.iter().map(|r|r.closing_fuel+r.generated_batches).max(),
        "first_generation":rows.iter().find(|r|r.generated_batches>0).map(|r|&r.month),
        "last_generation":rows.iter().rev().find(|r|r.generated_batches>0).map(|r|&r.month)})
}

pub(super) fn compare(
    s: &Supply,
    early: &[Flow],
    input: &ModelInput,
    boundary: &RollingSnapshot,
    route: &[Month],
    policy: &CirculationPolicy,
    cash: &[String],
) -> Value {
    let m = &input.model;
    let h = &m.seasonal.physical;
    let factory = m
        .limits
        .iter()
        .find(|l| l.name == "shared_factory")
        .unwrap();
    let work = m
        .limits
        .iter()
        .find(|l| l.name == "shared_human_work_ticks")
        .unwrap();
    let power = m
        .limits
        .iter()
        .find(|l| l.name == "fuel_generation")
        .unwrap();
    let life = m
        .lives
        .iter()
        .find(|l| l.resource == power.resource)
        .unwrap();
    let recipe = &h.processes[37];
    assert_eq!(recipe.outputs.iter().sum::<i64>(), 1);
    assert_eq!(recipe.outputs[power.resource], 1);
    assert_eq!(
        recipe.inputs.iter().sum::<i64>(),
        recipe.inputs[9] + recipe.inputs[21]
    );
    assert_eq!(factory.activity_use[37], 1);
    assert!(
        m.limits
            .iter()
            .all(|l| l.name == factory.name || l.name == work.name || l.activity_use[37] == 0)
    );
    let room: Vec<_> = route
        .iter()
        .enumerate()
        .map(|(t, r)| {
            let active = boundary
                .opening_cohorts
                .iter()
                .filter(|c| c.resource == factory.resource && c.expires > MONTHS + t)
                .map(|c| c.quantity)
                .sum::<i64>();
            (active * factory.units_per_stock - r.shared_work_and_factory_slots)
                .min(h.periods[MONTHS - YEAR + t % YEAR].capacities[37])
        })
        .collect();
    let start = room.iter().rposition(|&q| q > 0).unwrap();
    let completion = start + recipe.lead_periods;
    let first = completion + 1;
    let mut b = Build {
        start,
        completion,
        active: first..first + life.service_periods,
        units: 0,
        energy: recipe.inputs[9],
        material: recipe.inputs[21],
        work: work.activity_use[37],
        throughput: power.units_per_stock,
    };
    let caps: Vec<_> = (0..route.len())
        .map(|t| h.periods[MONTHS - YEAR + t % YEAR].capacities[31])
        .collect();
    let available_material =
        boundary.opening_stock[21] - m.seasonal.closing_floors.last().unwrap()[21];
    let max_units = room[start]
        .min(available_material / b.material)
        .min(s.slots[start].work / b.work);
    let target = early.last().unwrap().closing_energy;
    let initial_mine = early.iter().map(|r| r.extracted).sum::<i64>();
    // Leave the inherited extraction sites before expiry. Solve for how much
    // fuel to mine while charging the energy used by those same starts.
    let mut selected = None;
    for units in 1..=max_units.min(loads(*caps.iter().max().unwrap(), b.throughput)) {
        b.units = units;
        let mut mine = initial_mine;
        while let Some(rows) = late_plan(s, &b, &caps, target, mine) {
            let actual: i64 = rows.iter().map(|r| r.generated_batches).sum();
            let required = (actual - (s.fuel - s.fuel_floor)).max(0);
            if required == mine {
                selected = Some(rows);
                break;
            }
            assert!(required < mine);
            mine = required;
        }
        if selected.is_some() {
            break;
        }
    }
    let late = selected.expect("a replacement fleet must cover the comparison");
    let end = late.last().unwrap();
    let loss: i64 = late.iter().map(|r| r.storage_loss).sum();
    let mined: i64 = late.iter().map(|r| r.extracted).sum();
    let generated: i64 = late.iter().map(|r| r.generated_batches).sum();
    assert_eq!(
        s.energy + s.yield_ticks * generated,
        end.closing_energy
            + mined
            + loss
            + b.energy * b.units
            + s.slots.iter().map(|v| v.demand).sum::<i64>()
    );
    assert_eq!(s.deposit - end.closing_deposit, mined);
    assert_eq!(s.fuel + mined - generated, end.closing_fuel);
    let settle = |rows, units| settlement(policy, cash, input, route, rows, &b, units);
    let payments = json!({"opening_accounts":cash,"energy_store":settle(early,0),"fuel_store":settle(&late,b.units)});
    json!({"payments":payments,"energy_store":metrics(early,&b,0),"fuel_store":metrics(&late,&b,b.units),"build_start":calendar(MONTHS+start),
        "build_completion":calendar(MONTHS+b.completion),"usable_from":calendar(MONTHS+first),"retires":calendar(MONTHS+b.active.end),
        "terminal_energy_floor":target,"fuel_store_months":late})
}

fn settlement(
    p: &CirculationPolicy,
    cash: &[String],
    input: &ModelInput,
    route: &[Month],
    rows: &[Flow],
    b: &Build,
    units: i64,
) -> Value {
    let mut events = Vec::new();
    let h = &input.model.seasonal.physical;
    let opening: Vec<u64> = cash.iter().map(|v| v.parse().unwrap()).collect();
    let norm: Vec<u64> = p.unit_measure.iter().map(|v| v.parse().unwrap()).collect();
    let common: Vec<u64> = p
        .common_measure
        .iter()
        .map(|v| v.parse().unwrap())
        .collect();
    for (t, (row, flow)) in route.iter().zip(rows).enumerate() {
        let mut trade = |payer: usize, r: usize, q: i64, reason: &str| {
            let supplier = p.resource_suppliers[r];
            if payer == supplier || q == 0 || norm[r] == 0 {
                return;
            }
            let q = u64::try_from(q).unwrap();
            for (from, to, rate) in [(payer, supplier, norm[r]), (supplier, 0, common[r])] {
                if rate > 0 {
                    events.push(ScheduledMovement {
                        period: t as u32,
                        movement: Movement {
                            from,
                            to,
                            amount: q.checked_mul(rate).unwrap(),
                        },
                        basis: reason.to_owned(),
                    });
                }
            }
        };
        if t == b.completion {
            trade(0, 22, units, "Completed power modules");
        }
        let season = &h.periods[MONTHS - YEAR + t % YEAR];
        let freight = row.freight.as_ref().unwrap();
        assert_eq!(season.needs[6..9].iter().sum::<i64>(), freight.dispatch_kg);
        for &j in &p.process_order {
            let provider = p.process_operators[j];
            match j {
                17 | 21 | 25 => trade(
                    provider,
                    9,
                    season.needs[6 + (j - 17) / 4] * h.processes[j].inputs[9],
                    "Outbound haul energy",
                ),
                30 | 31 | 33 | 37 | 43 => {
                    let quantity = match j {
                        30 => flow.extracted,
                        31 => flow.generated_batches,
                        33 => row.fabricated_kg,
                        37 if t == b.start => units,
                        43 => row.sorted_kg / h.processes[43].inputs[29],
                        _ => 0,
                    };
                    for (r, &factor) in h.processes[j].inputs.iter().enumerate() {
                        trade(
                            provider,
                            r,
                            quantity * factor,
                            "Fuel, construction or wrapping inputs",
                        );
                    }
                    if j == 43 {
                        trade(
                            provider,
                            9,
                            freight.return_energy_ticks,
                            "Return haul energy charged to recovery",
                        );
                    }
                }
                _ => {}
            }
        }
    }
    let base = replay_schedule(&opening, &events, &[]).unwrap();
    let min = schedule_opening(opening.len(), &events).unwrap();
    let mut funded = Vec::new();
    let mut transfers = Vec::new();
    let mut allocated = 0;
    for (i, (need, &available)) in min
        .minimum_opening_by_account
        .iter()
        .zip(&opening)
        .enumerate()
        .skip(1)
    {
        let delta = need.parse::<u64>().unwrap().saturating_sub(available);
        if delta > 0 {
            assert_ne!(i, 1);
            funded.push(ScheduledMovement {
                period: 0,
                movement: Movement {
                    from: 0,
                    to: i,
                    amount: delta,
                },
                basis: "Existing treasury allocation at opening".into(),
            });
            transfers.push(json!({"account":p.accounts[i],"amount":delta}));
            allocated += delta;
        }
    }
    assert!(opening[0] >= allocated + min.minimum_opening_by_account[0].parse::<u64>().unwrap());
    funded.extend(events);
    let replay = replay_schedule(&opening, &funded, &[]).unwrap();
    assert_eq!(replay.attempted_events, replay.settled_events);
    assert_eq!(replay.opening_total, base.opening_total);
    let mut balances = opening.clone();
    let mut first_event = 0;
    let mut months = Vec::new();
    for (t, row) in route.iter().enumerate() {
        let count = funded[first_event..].partition_point(|e| e.period == t as u32);
        let run =
            replay_schedule(&balances, &funded[first_event..first_event + count], &[]).unwrap();
        assert_eq!(run.settled_events, count);
        assert_eq!(run.opening_total, replay.opening_total);
        balances = run.closing.iter().map(|x| x.parse().unwrap()).collect();
        months.push(json!({"month":row.month,"settled":count,"treasury_close":balances[0],"volume":run.settled_volume}));
        first_event += count;
    }
    assert_eq!(first_event, funded.len());
    assert!(
        balances
            .iter()
            .map(u64::to_string)
            .eq(replay.closing.iter().cloned())
    );
    let failed = base.first_blocked_event.map(|i| {
        let e = &funded[transfers.len() + i];
        json!({"month":calendar(MONTHS+e.period as usize),"payer":p.accounts[e.movement.from],
            "amount":e.movement.amount,"basis":e.basis})
    });
    json!({"first_blocked_without_allocation":failed,"operating_payments":base.attempted_events,
        "opening_total":replay.opening_total,"treasury_allocated":allocated,"allocations":transfers,
        "settled_including_allocations":replay.settled_events,"closing_accounts":replay.closing,"months":months})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loss_inverse_is_minimal_at_rounding_boundaries() {
        for q in 0..=3100 {
            let x = before_loss(q);
            assert_eq!(carried(x), q);
            assert!(x == 0 || carried(x - 1) < q);
        }
    }
    #[test]
    fn last_operating_month_can_only_supply_the_following_month() {
        let mut s = Supply {
            energy: 1,
            fuel: 1,
            deposit: 0,
            fuel_floor: 0,
            energy_store: 11,
            fuel_store: 1,
            yield_ticks: 10,
            slots: vec![
                Slot {
                    demand: 0,
                    work: 1,
                    extract: 0,
                    generate: 0
                };
                4
            ],
        };
        let b = Build {
            start: 0,
            completion: 1,
            active: 2..3,
            units: 1,
            energy: 1,
            material: 1,
            work: 1,
            throughput: 1,
        };
        s.slots[3].demand = 10;
        let rows = late_plan(&s, &b, &[1; 4], 0, 0).unwrap();
        assert_eq!(
            (
                rows[1].generated_batches,
                rows[2].generated_batches,
                rows[3].energy_arrival
            ),
            (0, 1, 10)
        );
        s.slots[3].demand = 0;
        s.slots[2].demand = 10;
        assert!(late_plan(&s, &b, &[1; 4], 0, 0).is_none());
    }
}
