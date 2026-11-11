use qazna::{
    horizon::{Horizon, Period, PeriodPlan, Process},
    mathematics::{MathError, verify_linear},
    seasonal::*,
};
fn fixture() -> (Seasonal, Vec<PeriodPlan>) {
    let h = Horizon {
        resources: vec!["grain_kg".into()],
        initial: vec![10],
        storage_caps: vec![50],
        processes: vec![Process {
            inputs: vec![2],
            outputs: vec![10],
            lead_periods: 2,
        }],
        periods: (0..8)
            .map(|_| Period {
                supplies: vec![0],
                needs: vec![1],
                capacities: vec![2],
            })
            .collect(),
        objective_resource: 0,
    };
    let m = Seasonal {
        physical: h,
        land: vec![LandPool {
            name: "shared".into(),
            available_hectares: vec![2; 8],
        }],
        crops: vec![CropUse {
            process: 0,
            land_pool: 0,
            hectares_per_activity: 1,
            sowing_periods: vec![1, 5],
            irrigation_m3_per_activity: vec![3, 5],
            basis: "synthetic".into(),
        }],
        irrigation_m3: vec![10; 8],
        closing_floors: vec![vec![0]; 8],
    };
    let p = build(&m, &[0, 1, 0, 0, 0, 1, 0, 0]);
    (m, p)
}
fn build(m: &Seasonal, starts: &[i64]) -> Vec<PeriodPlan> {
    let h = &m.physical;
    let mut s = h.initial[0];
    (0..h.periods.len())
        .map(|t| {
            if t >= h.processes[0].lead_periods {
                s += starts[t - h.processes[0].lead_periods] * h.processes[0].outputs[0];
            }
            s += h.periods[t].supplies[0]
                - h.periods[t].needs[0]
                - starts[t] * h.processes[0].inputs[0];
            PeriodPlan {
                activities: vec![starts[t]],
                deliveries: h.periods[t].needs.clone(),
                closing: vec![s],
                discarded: vec![0],
            }
        })
        .collect()
}
fn agrees(m: &Seasonal, p: &[PeriodPlan], ok: bool) {
    assert_eq!(m.replay(p).is_ok(), ok);
    assert_eq!(
        verify_linear(&m.compile().unwrap(), &m.physical.vector(p).unwrap(), None).is_ok(),
        ok
    );
}
#[test]
fn land_released_at_harvest_and_water_traced_by_growth_age() {
    let (m, p) = fixture();
    agrees(&m, &p, true);
    let e = m.replay(&p).unwrap();
    assert_eq!(e.irrigation_m3_by_period, vec![0, 3, 5, 0, 0, 3, 5, 0]);
    assert_eq!(e.land_hectares_by_period[3], vec![0]);
}
#[test]
fn future_harvest_cannot_feed_present() {
    let (mut m, _) = fixture();
    m.physical.initial[0] = 2;
    let p = build(&m, &[0, 1, 0, 0, 0, 1, 0, 0]);
    agrees(&m, &p, false);
}
#[test]
fn abundant_annual_water_cannot_cover_peak_month_shortage() {
    let (mut m, p) = fixture();
    m.irrigation_m3[0] = 1_000_000;
    m.irrigation_m3[2] = 4;
    agrees(&m, &p, false);
}
#[test]
fn food_and_feed_crops_compete_for_same_land() {
    let (mut m, mut p) = fixture();
    m.physical.processes.push(m.physical.processes[0].clone());
    let mut c = m.crops[0].clone();
    c.process = 1;
    m.crops.push(c);
    for period in &mut m.physical.periods {
        period.capacities.push(2);
    }
    for row in &mut p {
        row.activities.push(row.activities[0]);
    }
    m.land[0].available_hectares.fill(1);
    // Correct the shared commodity balance for the additional crop.
    let extra = build(&fixture().0, &[0, 1, 0, 0, 0, 1, 0, 0]);
    for (t, row) in p.iter_mut().enumerate() {
        row.closing[0] += extra[t].closing[0] - 10 + (t as i64 + 1);
    }
    agrees(&m, &p, false);
    m.land[0].available_hectares.fill(2);
    agrees(&m, &p, true);
}
#[test]
fn seed_for_next_season_cannot_be_relabelled_as_consumed_food() {
    let (mut m, mut p) = fixture();
    m.closing_floors[7][0] = 2;
    let stock = p[7].closing[0];
    p[7].deliveries[0] += stock;
    p[7].closing[0] = 0;
    m.physical.periods[7].needs[0] += stock;
    agrees(&m, &p, false);
}
#[test]
fn full_needs_not_only_objective_resource_are_enforced() {
    let (m, mut p) = fixture();
    p[0].deliveries[0] = 0;
    for row in &mut p {
        row.closing[0] += 1;
    }
    agrees(&m, &p, false);
}
#[test]
fn planting_outside_calendar_and_after_horizon_are_rejected() {
    let (m, _) = fixture();
    agrees(&m, &build(&m, &[1, 0, 0, 0, 0, 0, 0, 0]), false);
    agrees(&m, &build(&m, &[0, 0, 0, 0, 0, 0, 0, 1]), false);
}
#[test]
fn malformed_extension_fails_instead_of_omitting_constraints() {
    let (base, p) = fixture();
    for case in 0..7 {
        let mut m = base.clone();
        match case {
            0 => m.crops.push(m.crops[0].clone()),
            1 => m.crops[0].land_pool = 1,
            2 => m.crops[0].sowing_periods.push(7),
            3 => m.crops[0]
                .irrigation_m3_per_activity
                .pop()
                .map(|_| ())
                .unwrap(),
            4 => m.closing_floors[0][0] = -1,
            5 => m.land[0].available_hectares.pop().map(|_| ()).unwrap(),
            _ => m.irrigation_m3[0] = -1,
        };
        assert!(m.compile().is_err());
        assert!(m.replay(&p).is_err());
    }
}
#[test]
fn irrigation_units_efficiency_and_rounding() {
    assert_eq!(irrigation_per_hectare(100, 40, 8000).unwrap(), 750);
    assert_eq!(irrigation_per_hectare(1, 0, 3000).unwrap(), 34);
    assert_eq!(irrigation_per_hectare(10, 30, 8000).unwrap(), 0);
    assert!(irrigation_per_hectare(1, 0, 0).is_err());
    assert_eq!(
        irrigation_per_hectare(u64::MAX, 0, 1),
        Err(MathError::Overflow)
    );
}
#[test]
fn bounded_exhaustive_three_period_oracle() {
    // External scalar equations, 2,048 cases, include harvest timing, seed use,
    // shortages, terminal planting, land, water and exact monthly deliveries.
    for initial in 0..4 {
        for lag in 1..3 {
            for a in 0..4 {
                for b in 0..4 {
                    for c in 0..4 {
                        for limit in 0..4 {
                            let (mut m, _) = fixture();
                            m.physical.initial[0] = initial;
                            m.physical.periods.truncate(3);
                            m.physical.processes[0].lead_periods = lag;
                            m.physical.processes[0].outputs[0] = 3;
                            m.physical.processes[0].inputs[0] = 1;
                            m.land[0].available_hectares = vec![limit; 3];
                            m.irrigation_m3 = vec![2; 3];
                            m.closing_floors = vec![vec![0]; 3];
                            m.crops[0].irrigation_m3_per_activity = vec![1; lag];
                            m.crops[0].sowing_periods = (0..3 - lag).collect();
                            let starts = [a, b, c];
                            let p = build(&m, &starts);
                            let mut expected = true;
                            let mut stock = initial;
                            for t in 0..3 {
                                if t >= lag {
                                    stock += starts[t - lag] * 3;
                                }
                                stock -= 1 + starts[t];
                                let occupied: i64 =
                                    starts[t.saturating_sub(lag - 1)..=t].iter().sum();
                                expected &= (0..=50).contains(&stock)
                                    && starts[t] <= 2
                                    && occupied <= limit
                                    && occupied <= 2
                                    && (starts[t] == 0 || t + lag < 3);
                            }
                            agrees(&m, &p, expected);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn multiple_resource_needs_cannot_be_hidden_behind_one_objective() {
    let (mut m, mut p) = fixture();
    m.physical.resources.push("other_food_kg".into());
    m.physical.initial.push(8);
    m.physical.storage_caps.push(8);
    for process in &mut m.physical.processes {
        process.inputs.push(0);
        process.outputs.push(0);
    }
    for (t, period) in m.physical.periods.iter_mut().enumerate() {
        period.supplies.push(0);
        period.needs.push(1);
        m.closing_floors[t].push(0);
        p[t].deliveries.push(1);
        p[t].closing.push(7 - t as i64);
        p[t].discarded.push(0);
    }
    agrees(&m, &p, true);
    p[0].deliveries[1] = 0;
    for row in &mut p {
        row.closing[1] += 1;
    }
    agrees(&m, &p, false);
}
#[test]
fn extreme_process_products_do_not_wrap_to_feasible_stock() {
    let (mut m, _) = fixture();
    m.physical.periods.truncate(2);
    m.physical.initial[0] = i64::MAX;
    m.physical.storage_caps[0] = i64::MAX;
    m.physical.processes = vec![
        Process {
            inputs: vec![i64::MAX],
            outputs: vec![i64::MAX],
            lead_periods: 1
        };
        3
    ];
    for period in &mut m.physical.periods {
        period.needs[0] = 0;
        period.capacities = vec![i64::MAX; 3];
    }
    m.crops.clear();
    m.land[0].available_hectares.truncate(2);
    m.irrigation_m3.truncate(2);
    m.closing_floors.truncate(2);
    let p = vec![
        PeriodPlan {
            activities: vec![i64::MAX; 3],
            deliveries: vec![0],
            closing: vec![0],
            discarded: vec![0],
        },
        PeriodPlan {
            activities: vec![0; 3],
            deliveries: vec![0],
            closing: vec![0],
            discarded: vec![0],
        },
    ];
    assert!(matches!(m.replay(&p), Err(MathError::Overflow)));
    assert_eq!(
        verify_linear(&m.compile().unwrap(), &m.physical.vector(&p).unwrap(), None),
        Err(MathError::Overflow)
    );
}

#[test]
fn seasonal_shared_work_cannot_be_booked_twice_or_used_before_completion() {
    use qazna::horizon::StockCapacity;
    let (m, p) = fixture();
    // The existing grain stock is used solely as a synthetic capacity proxy here.
    // At month 1 opening stock is 9, not its end-of-month 6.
    for units in 1..=12 {
        let cap = vec![StockCapacity {
            name: "shared".into(),
            resource: 0,
            units_per_stock: 1,
            activity_use: vec![units],
        }];
        let expected = p.iter().enumerate().all(|(t, row)| {
            let opening = if t == 0 {
                m.physical.initial[0]
            } else {
                p[t - 1].closing[0]
            };
            row.activities[0] * units <= opening
        });
        assert_eq!(m.replay_with_capabilities(&p, &cap).is_ok(), expected);
        assert_eq!(
            verify_linear(
                &m.compile_with_capabilities(&cap).unwrap(),
                &m.physical.vector(&p).unwrap(),
                None
            )
            .is_ok(),
            expected
        );
    }
}
#[test]
fn shared_extension_keeps_full_needs_and_rejects_malformed_limits() {
    use qazna::horizon::StockCapacity;
    let (m, mut p) = fixture();
    let valid = StockCapacity {
        name: "work".into(),
        resource: 0,
        units_per_stock: 1,
        activity_use: vec![1],
    };
    assert!(
        m.compile_with_capabilities(&[valid.clone(), valid.clone()])
            .is_err()
    );
    let mut bad = valid.clone();
    bad.activity_use = vec![-1];
    assert!(m.replay_with_capabilities(&p, &[bad]).is_err());
    let mut bad = valid.clone();
    bad.resource = 1;
    assert!(m.compile_with_capabilities(&[bad]).is_err());
    p[7].deliveries[0] = 0;
    p[7].closing[0] += 1;
    assert!(
        m.replay_with_capabilities(&p, std::slice::from_ref(&valid))
            .is_err()
    );
    assert!(
        verify_linear(
            &m.compile_with_capabilities(&[valid]).unwrap(),
            &m.physical.vector(&p).unwrap(),
            None
        )
        .is_err()
    );
}
