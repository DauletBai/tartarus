use qazna::{
    capital::*, horizon::*, materials::MaterialBasis, mathematics::verify_linear,
    rolling::RollingCapital, seasonal::*,
};
fn fixture(n: usize) -> (CapitalCycle, Vec<PeriodPlan>) {
    let h = Horizon {
        resources: vec!["material".into(), "machine".into(), "food".into()],
        initial: vec![100, 1, 1],
        storage_caps: vec![100, 100, 100],
        processes: vec![
            Process {
                inputs: vec![1, 0, 0],
                outputs: vec![0, 1, 0],
                lead_periods: 1,
            },
            Process {
                inputs: vec![1, 0, 0],
                outputs: vec![0, 0, 1],
                lead_periods: 1,
            },
        ],
        periods: vec![
            Period {
                supplies: vec![0; 3],
                needs: vec![0, 0, 1],
                capacities: vec![1, 1]
            };
            n
        ],
        objective_resource: 2,
    };
    let m = CapitalCycle {
        seasonal: Seasonal {
            physical: h,
            land: vec![LandPool {
                name: "unused".into(),
                available_hectares: vec![0; n],
            }],
            crops: vec![],
            irrigation_m3: vec![0; n],
            closing_floors: vec![vec![0; 3]; n],
        },
        limits: vec![StockCapacity {
            name: "machine work".into(),
            resource: 1,
            units_per_stock: 1,
            activity_use: vec![0, 1],
        }],
        lives: vec![AssetLife {
            resource: 1,
            service_periods: 3,
            initial_remaining_periods: 3,
        }],
        stores: vec![],
    };
    let builds: Vec<usize> = (1..n - 1).step_by(3).collect();
    let mut stock = vec![100, 1, 1];
    let mut p = vec![];
    for t in 0..n {
        let b = i64::from(builds.contains(&t));
        let food = i64::from(t + 1 < n);
        let expired = i64::from(t == 3) + builds.iter().filter(|&&s| s + 5 == t).count() as i64;
        if t > 0 {
            stock[1] += i64::from(builds.contains(&(t - 1)));
            stock[2] += 1;
        }
        stock[0] -= b + food;
        stock[1] -= expired;
        stock[2] -= 1;
        p.push(PeriodPlan {
            activities: vec![b, food],
            deliveries: vec![0, 0, 1],
            closing: stock.clone(),
            discarded: vec![0, expired, 0],
        });
    }
    (m, p)
}
fn agree(c: &RollingCapital, m: &CapitalCycle, p: &[PeriodPlan], ok: bool) {
    assert_eq!(
        c.compile(m)
            .and_then(|a| verify_linear(&a, &m.seasonal.physical.vector(p)?, None))
            .is_ok(),
        ok
    );
    assert_eq!(c.replay(m, p).is_ok(), ok);
}
#[test]
fn every_boundary_retains_cohort_age_and_work_in_progress() {
    let (m, p) = fixture(14);
    for t in 1..=14 {
        let c = RollingCapital::checkpoint(&m, &p, t).unwrap();
        agree(&c, &m, &p, true);
        assert_eq!(c.snapshot().opening_stock, p[t - 1].closing);
        for q in &c.snapshot().opening_cohorts {
            assert!(q.expires >= t);
        }
        for q in &c.snapshot().pending {
            assert!(q.started < t && q.completes >= t);
        }
    }
    let c = RollingCapital::checkpoint(&m, &p, 2).unwrap();
    assert!(
        c.snapshot()
            .pending
            .iter()
            .any(|v| v.process == 0 && v.started == 1 && v.completes == 2)
    );
    let c = RollingCapital::checkpoint(&m, &p, 3).unwrap();
    assert!(c.snapshot().opening_cohorts.iter().any(|v| v.expires == 3));
    assert!(c.snapshot().opening_cohorts.iter().any(|v| v.expires == 6));
}
#[test]
fn extending_and_repeated_checkpointing_match_one_long_plan() {
    let (m, p) = fixture(8);
    let c = RollingCapital::checkpoint(&m, &p, 5).unwrap();
    let (long, q) = fixture(14);
    agree(&c, &long, &q, true);
    let d = RollingCapital::checkpoint(&long, &q, 10).unwrap();
    agree(&d, &long, &q, true);
    let (short, z) = fixture(4);
    agree(&c, &short, &z, false);
}
#[test]
fn prefix_rewrite_is_rejected_even_when_new_plan_is_physically_valid() {
    let (m, p) = fixture(14);
    let c = RollingCapital::checkpoint(&m, &p, 5).unwrap();
    let mut q = p.clone();
    q[0].discarded[0] = 1;
    for r in &mut q {
        r.closing[0] -= 1;
    }
    assert!(m.replay(&q).is_ok());
    agree(&c, &m, &q, false);
}
#[test]
fn historical_and_schema_rewrites_cannot_rejuvenate_assets() {
    let (m, p) = fixture(14);
    let c = RollingCapital::checkpoint(&m, &p, 5).unwrap();
    let mut n = m.clone();
    n.lives[0].initial_remaining_periods = 2;
    agree(&c, &n, &p, false);
    let mut n = m.clone();
    n.seasonal.physical.initial[0] += 1;
    agree(&c, &n, &p, false);
    let mut n = m.clone();
    n.seasonal.physical.periods[0].capacities[0] += 1;
    agree(&c, &n, &p, false);
    let mut n = m.clone();
    n.seasonal.physical.processes[0].lead_periods = 2;
    agree(&c, &n, &p, false);
    let mut n = m.clone();
    n.seasonal.land[0].available_hectares[0] = 1;
    agree(&c, &n, &p, false);
}
#[test]
fn future_supply_can_change_but_cannot_rewrite_the_past() {
    let (m, p) = fixture(14);
    let c = RollingCapital::checkpoint(&m, &p, 5).unwrap();
    let mut n = m.clone();
    n.seasonal.physical.periods[5].supplies[0] = 2;
    let mut q = p.clone();
    for r in &mut q[5..] {
        r.closing[0] += 2;
    }
    agree(&c, &n, &q, true);
    n.seasonal.physical.periods[4].supplies[0] = 2;
    agree(&c, &n, &q, false);
}
#[test]
fn invalid_or_unverified_checkpoint_cannot_be_imported() {
    let (m, p) = fixture(14);
    let initial = RollingCapital::checkpoint(&m, &p, 0).unwrap();
    assert_eq!(
        initial.snapshot().opening_stock,
        m.seasonal.physical.initial
    );
    assert!(initial.snapshot().pending.is_empty());
    agree(&initial, &m, &p, true);
    assert!(RollingCapital::checkpoint(&m, &p, 15).is_err());
    let mut q = p.clone();
    q[1].closing[0] += 10;
    assert!(RollingCapital::checkpoint(&m, &q, 5).is_err());
    let c = RollingCapital::checkpoint(&m, &p, 5).unwrap();
    assert!(c.replay(&m, &p[..2]).is_err());
}
#[test]
fn constituent_audit_carries_mass_in_flight_not_created_at_completion() {
    let (m, p) = fixture(14);
    let h = &m.seasonal.physical;
    let b = MaterialBasis {
        name: "synthetic constituent".into(),
        unit: "test mass units".into(),
        source: "unit fixture".into(),
        quantity_per_resource_unit: vec![1; 3],
    };
    let e = b.audit(h, &p).unwrap();
    assert_eq!(e.initial, 102);
    assert_eq!(e.work_in_progress[0], 1);
    assert_eq!(*e.work_in_progress.last().unwrap(), 0);
    assert_eq!(
        e.initial + e.supplied,
        e.final_stock + e.discarded + e.delivered
    );
    let mut wrong = b.clone();
    wrong.quantity_per_resource_unit[1] = 2;
    assert!(wrong.audit(h, &p).is_err());
    let mut wrong = b.clone();
    wrong.source.clear();
    assert!(wrong.audit(h, &p).is_err());
}
#[test]
fn material_overflow_and_unrecorded_losses_are_rejected() {
    let (mut m, _) = fixture(3);
    let mut b = MaterialBasis {
        name: "test".into(),
        unit: "test".into(),
        source: "synthetic".into(),
        quantity_per_resource_unit: vec![i64::MAX; 3],
    };
    m.seasonal.physical.processes[0].inputs = vec![i64::MAX; 3];
    assert!(matches!(
        b.validate(&m.seasonal.physical),
        Err(qazna::mathematics::MathError::Overflow)
    ));
    b.quantity_per_resource_unit = vec![1, 1, 1];
    m.seasonal.physical.processes[0].inputs = vec![2, 0, 0];
    assert!(b.validate(&m.seasonal.physical).is_err());
}

#[test]
fn closed_terminal_stock_does_not_promise_next_period_service() {
    let (m, p) = fixture(8);
    let c = RollingCapital::checkpoint(&m, &p, 8).unwrap();
    let (long, q) = fixture(14);
    // A longer plan would have started food in period 7. That start was not
    // committed by the closed 8-period plan, so extension cannot invent it.
    agree(&c, &long, &q, false);
}

#[test]
fn constituent_in_flight_matches_direct_oracle_for_160_delay_schedules() {
    for lag in 1..=5 {
        for mask in 0..32u32 {
            let h = Horizon {
                resources: vec!["ore".into(), "product".into(), "residue".into()],
                initial: vec![100, 0, 0],
                storage_caps: vec![100; 3],
                processes: vec![Process {
                    inputs: vec![2, 0, 0],
                    outputs: vec![0, 1, 1],
                    lead_periods: lag,
                }],
                periods: vec![
                    Period {
                        supplies: vec![0; 3],
                        needs: vec![0; 3],
                        capacities: vec![1]
                    };
                    10
                ],
                objective_resource: 1,
            };
            let starts: Vec<usize> = (0..5).filter(|&s| mask & (1 << s) != 0).collect();
            let p: Vec<PeriodPlan> = (0..10)
                .map(|t| {
                    let started = starts.iter().filter(|&&s| s <= t).count() as i64;
                    let finished = starts.iter().filter(|&&s| s + lag <= t).count() as i64;
                    PeriodPlan {
                        activities: vec![i64::from(starts.contains(&t))],
                        deliveries: vec![0; 3],
                        closing: vec![100 - 2 * started, finished, finished],
                        discarded: vec![0; 3],
                    }
                })
                .collect();
            let basis = MaterialBasis {
                name: "test".into(),
                unit: "mass".into(),
                source: "independent oracle".into(),
                quantity_per_resource_unit: vec![1; 3],
            };
            let e = basis.audit(&h, &p).unwrap();
            for t in 0..10 {
                assert_eq!(
                    e.work_in_progress[t],
                    2 * starts.iter().filter(|&&s| s <= t && t < s + lag).count() as i128
                );
            }
            assert_eq!(e.final_stock, 100);
        }
    }
}
