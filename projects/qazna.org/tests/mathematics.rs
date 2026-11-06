use qazna::mathematics::*;

fn vector(numerators: &[i64], denominator: i64) -> RationalVector {
    RationalVector {
        numerators: numerators.to_vec(),
        denominator,
    }
}
fn row(terms: &[(usize, i64)], rhs: i64) -> Row {
    Row {
        name: "resource".into(),
        terms: terms.to_vec(),
        sense: Sense::AtMost,
        rhs,
    }
}
#[test]
fn resource_dual_bound_is_an_independent_optimum_certificate() {
    let model = LinearModel {
        objective: vec![1, 1],
        rows: vec![row(&[(0, 2), (1, 1)], 240), row(&[(0, 1), (1, 2)], 240)],
    };
    let result = verify_linear(&model, &vector(&[80, 80], 1), Some(&vector(&[1, 1], 3))).unwrap();
    assert_eq!(result.objective_numerator, "160");
    assert!(result.optimal_for_supplied_linear_model && result.integral);
    let feasible = verify_linear(&model, &vector(&[100, 0], 1), None).unwrap();
    assert!(!feasible.optimal_for_supplied_linear_model);
    assert_eq!(
        verify_linear(&model, &vector(&[100, 0], 1), Some(&vector(&[1, 1], 3))),
        Err(MathError::BadCertificate)
    );
    assert!(matches!(
        verify_linear(&model, &vector(&[81, 80], 1), None),
        Err(MathError::Infeasible(_))
    ));
}
#[test]
fn fractional_lp_is_not_claimed_as_an_integer_solution() {
    let model = LinearModel {
        objective: vec![1],
        rows: vec![row(&[(0, 2)], 3)],
    };
    let r = verify_linear(&model, &vector(&[3], 2), Some(&vector(&[1], 2))).unwrap();
    assert!(r.optimal_for_supplied_linear_model && !r.integral);
    assert_eq!(
        verify_linear(&model, &vector(&[1], 1), Some(&vector(&[1], 2))),
        Err(MathError::BadCertificate)
    );
}
#[test]
fn equality_duals_may_be_negative_but_capacity_duals_may_not() {
    let mut model = LinearModel {
        objective: vec![1],
        rows: vec![row(&[(0, -1)], -3)],
    };
    model.rows[0].sense = Sense::Equal;
    assert!(verify_linear(&model, &vector(&[3], 1), Some(&vector(&[-1], 1))).is_ok());
    model.rows[0].sense = Sense::AtMost;
    assert_eq!(
        verify_linear(&model, &vector(&[3], 1), Some(&vector(&[-1], 1))),
        Err(MathError::BadCertificate)
    );
}
#[test]
fn malformed_inputs_and_overflow_fail_closed() {
    let mut model = LinearModel {
        objective: vec![1],
        rows: vec![row(&[(0, 1)], 3)],
    };
    for p in [
        vector(&[0], 0),
        vector(&[0], -1),
        vector(&[-1], 1),
        vector(&[], 1),
    ] {
        assert!(verify_linear(&model, &p, None).is_err());
    }
    assert!(verify_linear(&model, &vector(&[1], 1), Some(&vector(&[], 1))).is_err());
    model.rows[0].terms = vec![(0, 1), (0, 1)];
    assert!(verify_linear(&model, &vector(&[1], 1), None).is_err());
    model.rows[0].terms = vec![(1, 1)];
    assert!(verify_linear(&model, &vector(&[1], 1), None).is_err());
    let max = i64::MAX;
    let huge = LinearModel {
        objective: vec![1; 3],
        rows: vec![row(&[(0, max), (1, max), (2, max)], max)],
    };
    assert_eq!(
        verify_linear(&huge, &vector(&[max; 3], max), None),
        Err(MathError::Overflow)
    );
}
#[test]
fn valid_duals_agree_with_exhaustive_integer_search() {
    // Independent enumeration, not a replay of the certificate algorithm.
    for grain in 0..=15 {
        for energy in 0..=15 {
            let model = LinearModel {
                objective: vec![1, 1],
                rows: vec![
                    row(&[(0, 2), (1, 1)], grain),
                    row(&[(0, 1), (1, 2)], energy),
                ],
            };
            let feasible: Vec<_> = (0..=15)
                .flat_map(|x| (0..=15).map(move |y| (x, y)))
                .filter(|&(x, y)| 2 * x + y <= grain && x + 2 * y <= energy)
                .collect();
            let best = feasible.iter().map(|(x, y)| x + y).max().unwrap();
            for (x, y) in feasible {
                for dual in [vector(&[1, 1], 3), vector(&[1, 0], 1), vector(&[0, 1], 1)] {
                    if let Ok(r) = verify_linear(&model, &vector(&[x, y], 1), Some(&dual)) {
                        assert!(r.optimal_for_supplied_linear_model);
                        assert_eq!(x + y, best);
                    }
                }
            }
        }
    }
}
#[test]
fn proportional_quota_conserves_stock_respects_caps_and_is_order_independent() {
    for a in 0..=12u64 {
        for b in 0..=12u64 {
            for c in 0..=12u64 {
                for supply in 0..=40u64 {
                    let n = [(11, a), (22, b), (33, c)];
                    let r = proportional_quota(&n, supply).unwrap();
                    let total = a + b + c;
                    assert_eq!(r.iter().sum::<u64>(), supply.min(total));
                    let reversed =
                        proportional_quota(&[(33, c), (22, b), (11, a)], supply).unwrap();
                    assert_eq!(r.iter().copied().rev().collect::<Vec<_>>(), reversed);
                    for (i, &(_, need)) in n.iter().enumerate() {
                        assert!(r[i] <= need);
                        if total > 0 {
                            let error = (i128::from(r[i]) * i128::from(total)
                                - i128::from(supply.min(total)) * i128::from(need))
                            .abs();
                            assert!(error < i128::from(total));
                        }
                    }
                }
            }
        }
    }
    assert!(proportional_quota(&[(1, 1), (1, 2)], 1).is_err());
    assert!(proportional_quota(&[(1, u64::MAX), (2, 1)], 1).is_err());
}
#[test]
fn proportional_rounding_has_a_documented_supply_monotonicity_counterexample() {
    let needs = [(1, 1500), (2, 1500), (3, 900), (4, 500), (5, 500), (6, 200)];
    let low = proportional_quota(&needs, 25).unwrap();
    let high = proportional_quota(&needs, 26).unwrap();
    assert_eq!(low, vec![7, 7, 4, 3, 3, 1]);
    assert_eq!(high, vec![8, 8, 5, 2, 2, 1]);
    assert!(high[3] < low[3] && high[4] < low[4]);
}
fn edge(from: usize, to: usize, amount: u64) -> Movement {
    Movement { from, to, amount }
}
#[test]
fn liquidity_matches_all_prefixes_and_is_componentwise_minimal() {
    // 500 distinct schedules, including zero edges, disconnected accounts and H=1.
    let mut state = 71u64;
    for case in 0..500 {
        let mut movements = Vec::new();
        for _ in 0..(case % 13) {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let from = (state % 4) as usize;
            let to = (from + 1 + ((state >> 12) % 3) as usize) % 4;
            movements.push(edge(from, to, 1 + (state >> 32) % 30));
        }
        let repeats = 1 + (case % 7) as u64;
        let bound = liquidity_bound(4, &movements, repeats).unwrap();
        let opening: Vec<i128> = bound
            .minimum_opening_by_account
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        let mut balances = opening.clone();
        let mut low = balances.clone();
        for _ in 0..repeats {
            for e in &movements {
                balances[e.from] -= i128::from(e.amount);
                balances[e.to] += i128::from(e.amount);
                for i in 0..4 {
                    low[i] = low[i].min(balances[i]);
                    assert!(balances[i] >= 0);
                }
                assert_eq!(balances.iter().sum::<i128>(), opening.iter().sum::<i128>());
            }
        }
        for value in &low {
            assert_eq!(
                *value, 0,
                "reducing a positive minimum by one must make some prefix negative"
            );
        }
    }
}
#[test]
fn finite_treasury_requires_a_circulation_rule_not_just_conservation() {
    let drain = [edge(0, 1, 10), edge(1, 2, 10)];
    let b = liquidity_bound(3, &drain, 1825).unwrap();
    assert_eq!(b.minimum_opening_by_account, vec!["18250", "0", "0"]);
    assert!(!b.stationary_repeatable);
    let circle = [edge(0, 1, 10), edge(1, 2, 10), edge(2, 0, 10)];
    let b = liquidity_bound(3, &circle, 1825).unwrap();
    assert_eq!(b.minimum_opening_by_account, vec!["10", "0", "0"]);
    assert!(b.stationary_repeatable);
    assert!(liquidity_bound(3, &circle, 0).is_err());
    assert!(liquidity_bound(3, &[edge(0, 3, 1)], 1).is_err());
}

fn delayed_model() -> (qazna::horizon::Horizon, Vec<qazna::horizon::PeriodPlan>) {
    use qazna::horizon::*;
    (
        Horizon {
            resources: vec!["grain_kg".into(), "meal".into()],
            initial: vec![4, 0],
            storage_caps: vec![10, 10],
            processes: vec![Process {
                inputs: vec![2, 0],
                outputs: vec![0, 3],
                lead_periods: 1,
            }],
            periods: vec![
                Period {
                    supplies: vec![0, 0],
                    needs: vec![0, 0],
                    capacities: vec![2],
                },
                Period {
                    supplies: vec![0, 0],
                    needs: vec![0, 6],
                    capacities: vec![2],
                },
            ],
            objective_resource: 1,
        },
        vec![
            PeriodPlan {
                activities: vec![2],
                deliveries: vec![0, 0],
                closing: vec![0, 0],
                discarded: vec![0, 0],
            },
            PeriodPlan {
                activities: vec![0],
                deliveries: vec![0, 6],
                closing: vec![0, 0],
                discarded: vec![0, 0],
            },
        ],
    )
}
#[test]
fn time_expanded_matrix_enforces_delays_inputs_storage_and_horizon_boundary() {
    let (mut h, p) = delayed_model();
    let m = h.compile().unwrap();
    let x = h.vector(&p).unwrap();
    assert!(
        verify_linear(&m, &x, Some(&h.full_coverage_dual(&m)))
            .unwrap()
            .optimal_for_supplied_linear_model
    );
    // Signed/double-counted delivery cannot be hidden behind an end balance.
    let mut early = p.clone();
    early[0].deliveries[1] = 6;
    early[1].deliveries[1] = 0;
    assert!(verify_linear(&m, &h.vector(&early).unwrap(), None).is_err());
    h.periods[0].capacities[0] = 1;
    assert!(verify_linear(&h.compile().unwrap(), &x, None).is_err());
    h.periods[0].capacities[0] = 2;
    h.processes[0].lead_periods = 2;
    assert!(verify_linear(&h.compile().unwrap(), &x, None).is_err());
    h.processes[0].lead_periods = 0;
    assert!(h.compile().is_err());
    h.processes[0].lead_periods = 1;
    let mut terminal = p.clone();
    terminal[1].activities[0] = 1;
    assert!(verify_linear(&h.compile().unwrap(), &h.vector(&terminal).unwrap(), None).is_err());
}
#[test]
fn sparse_matrix_matches_an_independent_stock_replay() {
    // Enumerate starts, deliveries and waste; replay physical quantities before
    // asking the separate sparse matrix to accept or reject the proposed plan.
    let (h, _) = delayed_model();
    let m = h.compile().unwrap();
    for starts in 0..=3 {
        for delivered in 0..=9 {
            for waste in 0..=9 {
                let grain = 4 - 2 * starts;
                let meal = 3 * starts - delivered - waste;
                let p = vec![
                    qazna::horizon::PeriodPlan {
                        activities: vec![starts],
                        deliveries: vec![0, 0],
                        closing: vec![grain.max(0), 0],
                        discarded: vec![0, 0],
                    },
                    qazna::horizon::PeriodPlan {
                        activities: vec![0],
                        deliveries: vec![0, delivered],
                        closing: vec![grain.max(0), meal.max(0)],
                        discarded: vec![0, waste],
                    },
                ];
                let reference =
                    starts <= 2 && grain >= 0 && delivered <= 6 && (0..=10).contains(&meal);
                assert_eq!(
                    verify_linear(&m, &h.vector(&p).unwrap(), None).is_ok(),
                    reference
                );
            }
        }
    }
}
#[test]
fn cyclic_recipes_cannot_create_initial_inputs_from_future_output() {
    use qazna::horizon::*;
    let h = Horizon {
        resources: vec!["a".into(), "b".into()],
        initial: vec![0, 0],
        storage_caps: vec![10, 10],
        processes: vec![
            Process {
                inputs: vec![1, 0],
                outputs: vec![0, 1],
                lead_periods: 1,
            },
            Process {
                inputs: vec![0, 1],
                outputs: vec![1, 0],
                lead_periods: 1,
            },
        ],
        periods: vec![
            Period {
                supplies: vec![0, 0],
                needs: vec![0, 0],
                capacities: vec![1, 1]
            };
            2
        ],
        objective_resource: 0,
    };
    let p = vec![
        PeriodPlan {
            activities: vec![1, 1],
            deliveries: vec![0, 0],
            closing: vec![0, 0],
            discarded: vec![0, 0]
        };
        2
    ];
    assert!(verify_linear(&h.compile().unwrap(), &h.vector(&p).unwrap(), None).is_err());
}

#[test]
fn fast_divisor_rule_matches_sequential_independent_reference_and_is_monotone() {
    for a in 0..=9u64 {
        for b in 0..=9u64 {
            for c in 0..=9u64 {
                let needs = [(11, a), (22, b), (33, c)];
                let mut reference = [0u64; 3];
                for supply in 0..=a + b + c {
                    let result = divisor_allocation(&needs, supply).unwrap();
                    assert_eq!(result, reference);
                    let reverse = divisor_allocation(&[(33, c), (22, b), (11, a)], supply).unwrap();
                    assert_eq!(result.iter().copied().rev().collect::<Vec<_>>(), reverse);
                    // Enumerate the next priority by exact cross-products; start at zero.
                    let mut winner: Option<usize> = None;
                    for i in 0..3 {
                        if reference[i] == needs[i].1 {
                            continue;
                        }
                        if let Some(j) = winner {
                            if needs[i].1 * (reference[j] + 1) > needs[j].1 * (reference[i] + 1) {
                                winner = Some(i);
                            }
                        } else {
                            winner = Some(i);
                        }
                    }
                    if let Some(i) = winner {
                        reference[i] += 1;
                    }
                }
            }
        }
    }
    assert_eq!(
        divisor_allocation(&[(1, u64::MAX)], u64::MAX).unwrap(),
        vec![u64::MAX]
    );
    assert!(divisor_allocation(&[(1, u64::MAX), (2, 1)], 1).is_err());
}
#[test]
fn divisor_rule_solves_supply_paradox_but_does_not_guarantee_upper_quota() {
    let needs = [(1, 1500), (2, 1500), (3, 900), (4, 500), (5, 500), (6, 200)];
    let lo = divisor_allocation(&needs, 25).unwrap();
    let hi = divisor_allocation(&needs, 26).unwrap();
    assert!(lo.iter().zip(&hi).all(|(a, b)| a <= b));
    let skewed = [(1, 100), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)];
    let r = divisor_allocation(&skewed, 50).unwrap();
    assert_eq!(r, vec![50, 0, 0, 0, 0, 0]);
    assert!(r[0] > 48); // ceil(50*100/105)=48; explicitly record the trade-off.
}

#[test]
fn farkas_certificate_distinguishes_impossibility_from_bad_candidate() {
    let impossible = LinearModel {
        objective: vec![1],
        rows: vec![row(&[(0, 1)], 5), row(&[(0, -1)], -10)],
    };
    assert!(verify_infeasible(&impossible, &vector(&[1, 1], 1)).is_ok());
    let possible = LinearModel {
        objective: vec![1],
        rows: vec![row(&[(0, 1)], 10), row(&[(0, -1)], -5)],
    };
    assert_eq!(
        verify_infeasible(&possible, &vector(&[1, 1], 1)),
        Err(MathError::BadCertificate)
    );
    assert!(verify_infeasible(&impossible, &vector(&[0, 1], 1)).is_err());
    assert!(verify_infeasible(&impossible, &vector(&[-1, -1], 1)).is_err());
}
