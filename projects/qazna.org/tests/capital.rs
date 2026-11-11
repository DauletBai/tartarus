use qazna::{
    capital::*,
    horizon::{Horizon, Period, PeriodPlan, Process, StockCapacity},
    mathematics::verify_linear,
    seasonal::{LandPool, Seasonal},
};
fn model() -> CapitalCycle {
    CapitalCycle {
        seasonal: Seasonal {
            physical: Horizon {
                resources: vec!["material".into(), "machine".into(), "food".into()],
                initial: vec![100, 1, 1],
                storage_caps: vec![100, 20, 10],
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
                periods: (0..7)
                    .map(|_| Period {
                        supplies: vec![0; 3],
                        needs: vec![0, 0, 1],
                        capacities: vec![1, 1],
                    })
                    .collect(),
                objective_resource: 2,
            },
            land: vec![LandPool {
                name: "unused".into(),
                available_hectares: vec![0; 7],
            }],
            crops: vec![],
            irrigation_m3: vec![0; 7],
            closing_floors: vec![vec![0; 3]; 7],
        },
        limits: vec![StockCapacity {
            name: "work".into(),
            resource: 1,
            units_per_stock: 1,
            activity_use: vec![0, 1],
        }],
        lives: vec![AssetLife {
            resource: 1,
            service_periods: 2,
            initial_remaining_periods: 2,
        }],
        stores: vec![StoragePool {
            name: "store".into(),
            asset_resource: 1,
            units_per_asset: 100,
            stored_resources: vec![0],
        }],
    }
}
// Unit-cohort oracle, independent of the compiler and its expiry expression.
fn candidate(m: &CapitalCycle, builds: &[usize]) -> (Vec<PeriodPlan>, bool) {
    let mut stock = m.seasonal.physical.initial.clone();
    let mut out = Vec::new();
    let mut ok = true;
    let life = &m.lives[0];
    for t in 0..7 {
        let initial_live = t < life.initial_remaining_periods;
        let live_new = builds
            .iter()
            .filter(|&&s| s + 2 <= t && t < s + 2 + life.service_periods)
            .count() as i64;
        let retired = i64::from(t == life.initial_remaining_periods)
            + builds
                .iter()
                .filter(|&&s| t == s + 2 + life.service_periods)
                .count() as i64;
        let build = i64::from(builds.contains(&t));
        let food = i64::from(t < 6);
        if t > 0 {
            stock[1] += i64::from(builds.contains(&(t - 1)));
            stock[2] += 1;
        }
        stock[0] -= build + food;
        stock[1] -= retired;
        stock[2] -= 1;
        let active = i64::from(initial_live) + live_new;
        ok &= food <= active && stock[0] <= 100 * active;
        out.push(PeriodPlan {
            activities: vec![build, food],
            deliveries: vec![0, 0, 1],
            closing: stock.clone(),
            discarded: vec![0, retired, 0],
        });
    }
    (out, ok)
}
fn agrees(m: &CapitalCycle, p: &[PeriodPlan], expected: bool) {
    assert_eq!(m.replay(p).is_ok(), expected);
    assert_eq!(
        verify_linear(
            &m.compile().unwrap(),
            &m.seasonal.physical.vector(p).unwrap(),
            None
        )
        .is_ok(),
        expected
    );
}
#[test]
fn cohorts_replace_expiring_capacity_without_same_period_completion_credit() {
    let m = model();
    let (p, ok) = candidate(&m, &[0, 2, 4]);
    assert!(ok);
    agrees(&m, &p, true);
    let e = m.replay(&p).unwrap();
    assert_eq!(e.retirements_by_period[2][1], 1);
    assert_eq!(e.available_opening_stock_by_period[1][1], 1);
    assert_eq!(e.available_opening_stock_by_period[2][1], 1);
    let (late, ok) = candidate(&m, &[1, 3, 5]);
    assert!(!ok);
    agrees(&m, &late, false);
}
#[test]
fn exhaustive_unit_cohort_oracle_checks_calendar_and_shared_storage() {
    for life in 1..=3 {
        for remaining in 1..=life {
            for mask in 0..32 {
                let mut m = model();
                m.lives[0].service_periods = life;
                m.lives[0].initial_remaining_periods = remaining;
                let builds: Vec<_> = (0..5).filter(|i| mask & (1 << i) != 0).collect();
                let (p, ok) = candidate(&m, &builds);
                agrees(&m, &p, ok);
            }
        }
    }
}
#[test]
fn retirement_cannot_be_omitted_or_invented_to_rewrite_stock() {
    let m = model();
    let (mut p, _) = candidate(&m, &[0, 2, 4]);
    p[2].discarded[1] = 0;
    for row in &mut p[2..] {
        row.closing[1] += 1;
    }
    // Ordinary physical balances alone pass this forged immortal stock.
    assert!(m.seasonal.replay(&p).is_ok());
    agrees(&m, &p, false);
    let (mut p, _) = candidate(&m, &[0, 2, 4]);
    p[0].discarded[1] = 1;
    for row in &mut p {
        row.closing[1] -= 1;
    }
    agrees(&m, &p, false);
}
#[test]
fn storage_pool_is_shared_and_retired_warehouse_cannot_hold_goods() {
    let mut m = model();
    m.stores[0].units_per_asset = 90;
    let (p, _) = candidate(&m, &[0, 2, 4]);
    agrees(&m, &p, false);
    m.stores[0].units_per_asset = 100;
    let mut duplicate = m.clone();
    let mut other = duplicate.stores[0].clone();
    other.name = "second_pool_same_building".into();
    duplicate.stores.push(other);
    assert!(duplicate.compile().is_err());
    m.stores[0].stored_resources.push(0);
    assert!(m.compile().is_err());
}
#[test]
fn malformed_lives_and_untracked_asset_supply_are_rejected() {
    let m = model();
    for edit in 0..5 {
        let mut bad = m.clone();
        match edit {
            0 => bad.lives[0].service_periods = 0,
            1 => bad.lives.push(bad.lives[0].clone()),
            2 => bad.seasonal.physical.periods[1].supplies[1] = 1,
            3 => bad.seasonal.physical.processes[0].inputs[1] = 1,
            _ => bad.lives[0].initial_remaining_periods = 10001,
        }
        assert!(bad.compile().is_err());
    }
}
#[test]
fn expiry_scaling_overflow_is_reported() {
    let mut m = model();
    m.seasonal.physical.processes[0].outputs[1] = i64::MAX;
    let (mut p, _) = candidate(&model(), &[0, 2, 4]);
    p[0].activities[0] = 2;
    assert!(m.retirements(&p).is_err());
    m.limits[0].units_per_stock = 2;
    assert!(m.compile().is_err());
}
