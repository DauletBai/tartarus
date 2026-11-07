//! Dated cohorts and generator-service history used by the full physical graph.
use qazna::lifecycle::DatedStock;
use std::collections::BTreeMap;
#[derive(Clone)]
struct GeneratorCohort {
    quantity: i64,
    ready: usize,
    repair_due: Option<usize>,
    second_stage_ready: Option<usize>,
}
pub struct Lifecycle {
    food: [DatedStock; 4],
    product_life: u32,
    daily_service: bool,
    pending: BTreeMap<usize, Vec<(usize, u32, u64, bool)>>,
    generators: Vec<GeneratorCohort>,
    pub expired: [u64; 4],
    pub damaged: [u64; 4],
    pub maintenance_starts: i64,
}
impl Lifecycle {
    pub fn new(initial: &[i64]) -> Self {
        Self::preserved(initial, 5)
    }
    pub fn preserved(initial: &[i64], product_life: u32) -> Self {
        assert!(product_life > 0);
        let mut food: [DatedStock; 4] = std::array::from_fn(|_| DatedStock::default());
        for k in 0..4 {
            food[k]
                .add(
                    if k < 2 { 90 } else { product_life },
                    initial[k + 10] as u64,
                )
                .unwrap();
        }
        Self {
            food,
            product_life,
            daily_service: false,
            pending: BTreeMap::new(),
            generators: vec![],
            expired: [0; 4],
            damaged: [0; 4],
            maintenance_starts: 0,
        }
    }
    pub fn with_daily_service(mut self) -> Self {
        assert!(self.generators.is_empty());
        self.daily_service = true;
        self
    }
    pub fn second_stage_need(&self, day: usize) -> i64 {
        self.generators
            .iter()
            .filter(|g| g.second_stage_ready.is_some_and(|d| d <= day))
            .map(|g| g.quantity)
            .sum()
    }
    pub fn finish_service(&mut self, day: usize, quantity: i64) {
        assert!(self.daily_service && quantity > 0 && quantity <= self.second_stage_need(day));
        let due = day.checked_add(1).expect("service deadline overflow");
        let mut left = quantity;
        let mut finished = vec![];
        for g in &mut self.generators {
            if !g.second_stage_ready.is_some_and(|d| d <= day) {
                continue;
            }
            let n = left.min(g.quantity);
            if n == 0 {
                continue;
            }
            g.quantity -= n;
            left -= n;
            finished.push(GeneratorCohort {
                quantity: n,
                ready: g.ready,
                repair_due: Some(due),
                second_stage_ready: None,
            });
        }
        assert_eq!(left, 0);
        self.generators.retain(|g| g.quantity > 0);
        self.generators.extend(finished);
    }
    pub fn advance(&mut self, day: usize, arrivals: &[i64], damaged_transport: bool) -> [i64; 4] {
        let mut removed = [0; 4];
        for (k, item) in removed.iter_mut().enumerate() {
            let q = self.food[k].expire(day as u32);
            self.expired[k] += q;
            *item += q as i64;
        }
        let mut gross = [0u64; 4];
        for (k, e, q, transport) in self.pending.remove(&day).unwrap_or_default() {
            gross[k] += q;
            // Damage observed at arrival precedes expiration of the remainder.
            let loss = if damaged_transport && transport {
                q.div_ceil(10)
            } else {
                0
            };
            self.damaged[k] += loss;
            removed[k] += loss as i64;
            if e <= day as u32 {
                self.expired[k] += q - loss;
                removed[k] += (q - loss) as i64;
            } else {
                self.food[k].add(e, q - loss).unwrap();
            }
        }
        for k in 0..4 {
            assert_eq!(gross[k], arrivals[k + 10] as u64);
        }
        if arrivals[16] > 0 {
            self.generators.push(GeneratorCohort {
                quantity: arrivals[16],
                ready: day + 1,
                repair_due: None,
                second_stage_ready: None,
            });
        }
        if self.daily_service {
            let started: i64 = self
                .generators
                .iter()
                .filter(|g| g.second_stage_ready == Some(day))
                .map(|g| g.quantity)
                .sum();
            assert_eq!(started, arrivals[21]);
        }
        let mut repaired = 0;
        for g in &mut self.generators {
            if g.repair_due == Some(day) {
                repaired += g.quantity;
                g.repair_due = None;
                g.ready = day + 1;
            }
        }
        assert_eq!(repaired, arrivals[18]);
        removed
    }
    pub fn food_total(&self, k: usize) -> i64 {
        self.food[k - 10].total() as i64
    }
    pub fn eligible(&self, k: usize, day: usize) -> i64 {
        self.food[k - 10].usable_at(day as u32) as i64
    }
    pub fn position_at(&self, k: usize, day: usize) -> i64 {
        self.eligible(k, day)
            + self
                .pending
                .iter()
                .filter(|(due, _)| **due <= day)
                .flat_map(|(_, lots)| lots)
                .filter(|(index, expiry, _, _)| *index == k - 10 && *expiry > day as u32)
                .map(|(_, _, q, _)| *q as i64)
                .sum::<i64>()
    }
    pub fn consume(&mut self, k: usize, q: i64, day: usize) {
        self.food[k - 10].take(q as u64, day as u32).unwrap();
    }
    pub fn generator_capacity(&self, day: usize) -> i64 {
        self.generators
            .iter()
            .filter(|g| {
                g.repair_due.is_none()
                    && g.second_stage_ready.is_none()
                    && day >= g.ready
                    && day < g.ready + 20
            })
            .map(|g| g.quantity * 10)
            .sum()
    }
    pub fn service_need(&self, day: usize) -> i64 {
        self.generators
            .iter()
            .filter(|g| {
                g.repair_due.is_none() && g.second_stage_ready.is_none() && day >= g.ready + 20
            })
            .map(|g| g.quantity)
            .sum()
    }
    pub fn start_service(&mut self, day: usize, quantity: i64) {
        assert!(quantity > 0 && quantity <= self.service_need(day));
        let second_ready = day.checked_add(1).expect("service deadline overflow");
        let completed = day.checked_add(2).expect("service deadline overflow");
        let mut left = quantity;
        let mut repairing = vec![];
        for g in &mut self.generators {
            if g.repair_due.is_some() || g.second_stage_ready.is_some() || day < g.ready + 20 {
                continue;
            }
            let n = left.min(g.quantity);
            if n == 0 {
                continue;
            }
            g.quantity -= n;
            left -= n;
            repairing.push(GeneratorCohort {
                quantity: n,
                ready: g.ready,
                repair_due: (!self.daily_service).then_some(completed),
                second_stage_ready: self.daily_service.then_some(second_ready),
            });
        }
        assert_eq!(left, 0);
        self.generators.retain(|g| g.quantity > 0);
        self.generators.extend(repairing);
        self.maintenance_starts += quantity;
    }
    pub fn start_food(&mut self, day: usize, process: usize, lead: usize, quantity: i64) {
        let due = day + lead;
        let q = quantity as u64;
        let (out, cohorts, transport) = match process {
            5 => (0, vec![(due as u32 + 90, q)], false),
            6 | 12 => (1, self.food[0].take(q, due as u32).unwrap(), true),
            7 => {
                self.food[1].take(q, day as u32).unwrap();
                (
                    2,
                    vec![(
                        (due as u32)
                            .checked_add(self.product_life)
                            .expect("product expiry overflow"),
                        q,
                    )],
                    false,
                )
            }
            8 | 13 => (3, self.food[2].take(q, due as u32).unwrap(), true),
            _ => panic!("not a food process"),
        };
        for (expiry, n) in cohorts {
            self.pending
                .entry(due)
                .or_default()
                .push((out, expiry, n, transport));
        }
    }
    pub fn is_drained(&self) -> bool {
        self.pending.is_empty()
            && self
                .generators
                .iter()
                .all(|g| g.repair_due.is_none() && g.second_stage_ready.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_carries_expiry_without_renewing_food() {
        let mut initial = vec![0; 19];
        initial[12] = 10;
        let mut l = Lifecycle::new(&initial);
        l.start_food(0, 8, 2, 10);
        for day in 0..=5 {
            let mut arriving = vec![0; 19];
            if day == 2 {
                arriving[13] = 10;
            }
            let loss = l.advance(day, &arriving, false);
            if day == 2 {
                assert_eq!(l.food_total(13), 10);
                assert_eq!(l.eligible(13, 5), 0);
            }
            if day == 5 {
                assert_eq!(loss[3], 10);
                assert_eq!(l.food_total(13), 0);
            }
        }
        assert!(l.is_drained());
        assert_eq!(l.expired[3], 10);
    }
    #[test]
    fn cooking_and_transport_are_different_age_operations() {
        let mut initial = vec![0; 19];
        initial[11] = 10;
        let mut l = Lifecycle::new(&initial);
        for day in 0..=95 {
            let mut arriving = vec![0; 19];
            if day == 90 {
                arriving[12] = 10;
            }
            l.advance(day, &arriving, false);
            if day == 89 {
                l.start_food(day, 7, 1, 10);
            }
            if day == 90 {
                assert_eq!(l.eligible(12, 94), 10);
                assert_eq!(l.eligible(12, 95), 0);
            }
        }
        assert_eq!(l.expired[2], 10);
        assert_eq!(l.food_total(11), 0);
    }
    #[test]
    fn transport_loss_and_expiry_do_not_double_count() {
        let mut initial = vec![0; 19];
        initial[12] = 11;
        let mut l = Lifecycle::new(&initial);
        l.start_food(0, 8, 2, 11);
        for day in 0..=5 {
            let mut arriving = vec![0; 19];
            if day == 2 {
                arriving[13] = 11;
            }
            l.advance(day, &arriving, day == 2);
        }
        assert_eq!(l.damaged[3], 2);
        assert_eq!(l.expired[3], 9);
    }
    #[test]
    fn only_built_and_completed_service_cohorts_supply_backup_capacity() {
        let mut l = Lifecycle::new(&[0; 19]);
        for day in 0..=25 {
            let mut arriving = vec![0; 19];
            if day == 1 {
                arriving[16] = 2;
            }
            if day == 24 {
                arriving[18] = 1;
            }
            l.advance(day, &arriving, false);
            let expected = match day {
                0 | 1 | 22..=24 => 0,
                25 => 10,
                _ => 20,
            };
            assert_eq!(l.generator_capacity(day), expected);
            if day == 22 {
                assert_eq!(l.service_need(day), 2);
                l.start_service(day, 1);
                assert_eq!(l.service_need(day), 1);
            }
        }
        assert_eq!(l.service_need(25), 1);
        assert_eq!(l.maintenance_starts, 1);
        assert!(l.is_drained());
    }
    #[test]
    #[should_panic]
    fn fabricated_service_completion_is_rejected() {
        let mut l = Lifecycle::new(&[0; 19]);
        let mut arrivals = vec![0; 19];
        arrivals[18] = 1;
        l.advance(0, &arrivals, false);
    }
    #[test]
    #[should_panic]
    fn undated_food_arrival_is_rejected() {
        let mut l = Lifecycle::new(&[0; 19]);
        let mut arrivals = vec![0; 19];
        arrivals[13] = 1;
        l.advance(0, &arrivals, false);
    }
    #[test]
    fn future_position_excludes_expired_and_late_arrivals() {
        let mut initial = vec![0; 21];
        initial[12] = 30;
        let mut l = Lifecycle::new(&initial);
        l.start_food(0, 8, 1, 10);
        l.start_food(0, 13, 2, 20);
        assert_eq!(l.position_at(13, 0), 0);
        assert_eq!(l.position_at(13, 1), 10);
        assert_eq!(l.position_at(13, 2), 30);
        assert_eq!(l.position_at(13, 5), 0);
        let mut arrival = vec![0; 21];
        arrival[13] = 10;
        l.advance(1, &arrival, false);
        l.consume(13, 7, 1);
        assert_eq!(l.position_at(13, 2), 23);
    }
    #[test]
    fn long_lived_sealed_batches_keep_original_deadlines_in_transport() {
        // Abstract duration tests, not assertions that particular foods are safe.
        for life in [730, 3650, 10950] {
            let mut initial = vec![0; 21];
            initial[12] = 11;
            let mut l = Lifecycle::preserved(&initial, life);
            l.start_food(life as usize - 2, 8, 1, 11);
            let mut arrivals = vec![0; 21];
            arrivals[13] = 11;
            l.advance(life as usize - 1, &arrivals, true);
            assert_eq!(l.food_total(13), 9);
            assert_eq!(l.eligible(13, life as usize), 0);
            let expired = l.advance(life as usize, &[0; 21], false);
            assert_eq!(expired[3], 9);
            assert_eq!(l.expired[3] + l.damaged[3], 11);
            assert!(l.is_drained());
        }
    }
    #[test]
    #[should_panic(expected = "product expiry overflow")]
    fn preservation_deadline_cannot_wrap_to_a_false_date() {
        let mut initial = vec![0; 21];
        initial[11] = 10;
        let mut l = Lifecycle::preserved(&initial, u32::MAX);
        l.start_food(0, 7, 1, 10);
    }
    #[test]
    fn daily_repairs_match_individual_generator_oracle_for_729_capacity_paths() {
        // Three individually modelled machines versus aggregate cohort splitting.
        // Every 0/1/2-person-capacity pattern over six days, then full recovery.
        for pattern in 0..729 {
            let mut l = Lifecycle::new(&[0; 22]).with_daily_service();
            let mut ready_again = [None; 3];
            for day in 0..35 {
                let mut arrivals = [0; 22];
                if day == 0 {
                    arrivals[16] = 3;
                }
                if day == 22 {
                    arrivals[21] = 3;
                }
                arrivals[18] = ready_again.iter().filter(|&&r| r == Some(day + 1)).count() as i64;
                l.advance(day, &arrivals, false);
                let expected = if (1..21).contains(&day) {
                    30
                } else {
                    10 * ready_again
                        .iter()
                        .filter(|r| r.is_some_and(|t| t <= day))
                        .count() as i64
                };
                assert_eq!(
                    l.generator_capacity(day),
                    expected,
                    "pattern {pattern}, day {day}"
                );
                if day == 21 {
                    l.start_service(day, 3);
                }
                if day >= 22 {
                    let capacity = if day < 28 {
                        (pattern / 3usize.pow((day - 22) as u32)) % 3
                    } else {
                        3
                    };
                    let waiting = ready_again.iter().filter(|r| r.is_none()).count();
                    assert_eq!(l.second_stage_need(day), waiting as i64);
                    let n = capacity.min(waiting);
                    if n > 0 {
                        l.finish_service(day, n as i64);
                        for r in ready_again.iter_mut().filter(|r| r.is_none()).take(n) {
                            *r = Some(day + 2);
                        }
                    }
                }
            }
            assert!(l.is_drained());
            assert_eq!(l.maintenance_starts, 3);
        }
    }
    #[test]
    fn absent_second_day_cannot_be_replaced_by_elapsed_calendar_time() {
        let mut l = Lifecycle::new(&[0; 22]).with_daily_service();
        for day in 0..80 {
            let mut arrivals = [0; 22];
            if day == 0 {
                arrivals[16] = 2;
            }
            if day == 22 {
                arrivals[21] = 2;
            }
            l.advance(day, &arrivals, false);
            if day == 21 {
                l.start_service(day, 2);
            }
            if day >= 22 {
                assert_eq!(l.generator_capacity(day), 0);
                assert_eq!(l.second_stage_need(day), 2);
                assert_eq!(l.service_need(day), 0);
                assert!(!l.is_drained());
            }
        }
    }
    #[test]
    #[should_panic]
    fn phase_two_cannot_start_before_phase_one_arrives() {
        let mut l = Lifecycle::new(&[0; 22]).with_daily_service();
        let mut a = [0; 22];
        a[16] = 1;
        l.advance(0, &a, false);
        l.start_service(21, 1);
        l.finish_service(21, 1);
    }
    #[test]
    #[should_panic]
    fn phase_one_certificate_cannot_be_fabricated() {
        let mut l = Lifecycle::new(&[0; 22]).with_daily_service();
        let mut a = [0; 22];
        a[21] = 1;
        l.advance(0, &a, false);
    }
    #[test]
    #[should_panic]
    fn pending_service_cannot_be_started_twice() {
        let mut l = Lifecycle::new(&[0; 22]).with_daily_service();
        let mut a = [0; 22];
        a[16] = 2;
        l.advance(0, &a, false);
        l.start_service(21, 2);
        l.start_service(22, 1);
    }
}
