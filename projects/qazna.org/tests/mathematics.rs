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

fn scheduled(period: u32, from: usize, to: usize, amount: u64) -> ScheduledMovement {
    ScheduledMovement {
        period,
        movement: Movement { from, to, amount },
        basis: "Synthetic declared purpose".into(),
    }
}

#[test]
fn scheduled_closed_chain_and_payment_delay() {
    let immediate = vec![
        scheduled(0, 0, 1, 10),
        scheduled(0, 1, 2, 10),
        scheduled(0, 2, 0, 10),
        scheduled(1, 0, 1, 10),
        scheduled(1, 1, 2, 10),
        scheduled(1, 2, 0, 10),
    ];
    let bound = schedule_opening(3, &immediate).unwrap();
    assert_eq!(bound.minimum_opening_by_account, vec!["10", "0", "0"]);
    assert!(bound.stationary_repeatable);
    let result = replay_schedule(&[10, 0, 0], &immediate, &[]).unwrap();
    assert_eq!(result.settled_events, 6);
    assert_eq!(result.closing, vec!["10", "0", "0"]);
    let delayed = vec![
        scheduled(0, 0, 1, 10),
        scheduled(0, 1, 2, 10),
        scheduled(1, 0, 1, 10),
        scheduled(1, 1, 2, 10),
        scheduled(1, 2, 0, 10),
        scheduled(2, 2, 0, 10),
    ];
    assert_eq!(
        schedule_opening(3, &delayed)
            .unwrap()
            .minimum_opening_by_account,
        vec!["20", "0", "0"]
    );
    assert_eq!(
        replay_schedule(&[10, 0, 0], &delayed, &[])
            .unwrap()
            .first_blocked_period,
        Some(1)
    );
    assert_eq!(
        replay_schedule(&[20, 0, 0], &delayed, &[])
            .unwrap()
            .settled_events,
        6
    );
}

#[test]
fn blocked_edge_does_not_burn_money_or_invent_credit() {
    let events = vec![
        scheduled(0, 0, 1, 5),
        scheduled(0, 1, 2, 5),
        scheduled(0, 2, 0, 5),
    ];
    let run = replay_schedule(&[5, 0, 0], &events, &[1]).unwrap();
    assert_eq!(run.closing, vec!["0", "5", "0"]);
    assert_eq!(run.unavailable_events, 1);
    assert_eq!(run.insufficient_events, 1);
    assert_eq!(run.settled_volume, "5");
    assert_eq!(run.first_blocked_event, Some(1));
    // A large total in the wrong account cannot pay the first deadline.
    let run = replay_schedule(&[0, 500, 0], &events, &[]).unwrap();
    assert_eq!(run.first_blocked_event, Some(0));
}

#[test]
fn ordered_schedule_bound_matches_exhaustive_initial_states() {
    // Independent exhaustive oracle: enumerate all two-account openings and
    // execute signed changes in a plain small integer reference.
    for mask in 0..256usize {
        let events: Vec<_> = (0..4)
            .map(|k| {
                let code = (mask >> (2 * k)) & 3;
                scheduled(k as u32, code % 2, 1 - code % 2, (1 + code / 2) as u64)
            })
            .collect();
        let mut feasible = Vec::new();
        for a in 0..=8 {
            for b in 0..=8 {
                let mut balances = [a, b];
                let mut valid = true;
                for e in &events {
                    let m = &e.movement;
                    balances[m.from] -= m.amount as i32;
                    balances[m.to] += m.amount as i32;
                    valid &= balances.iter().all(|x| *x >= 0);
                }
                if valid {
                    feasible.push([a, b]);
                }
            }
        }
        let expected: Vec<_> = (0..2)
            .map(|i| feasible.iter().map(|b| b[i]).min().unwrap().to_string())
            .collect();
        assert_eq!(
            schedule_opening(2, &events)
                .unwrap()
                .minimum_opening_by_account,
            expected
        );
        let opening: Vec<_> = expected.iter().map(|s| s.parse().unwrap()).collect();
        assert_eq!(
            replay_schedule(&opening, &events, &[])
                .unwrap()
                .settled_events,
            events.len()
        );
    }
}

#[test]
fn denied_event_combinations_match_independent_prefix_history() {
    let events = vec![
        scheduled(0, 0, 1, 3),
        scheduled(0, 1, 2, 2),
        scheduled(1, 2, 0, 1),
        scheduled(1, 0, 2, 2),
        scheduled(2, 2, 1, 2),
        scheduled(2, 1, 0, 2),
    ];
    for mask in 0..64usize {
        let denied: Vec<_> = (0..6).filter(|i| mask & (1 << i) != 0).collect();
        let mut history = vec![[3i32, 1, 0]];
        let mut settled = 0;
        for (i, e) in events.iter().enumerate() {
            let before = *history.last().unwrap();
            let m = &e.movement;
            let mut after = before;
            if mask & (1 << i) == 0 && before[m.from] >= m.amount as i32 {
                after[m.from] -= m.amount as i32;
                after[m.to] += m.amount as i32;
                settled += 1;
            }
            assert_eq!(after.iter().sum::<i32>(), 4);
            history.push(after);
        }
        let run = replay_schedule(&[3, 1, 0], &events, &denied).unwrap();
        assert_eq!(run.settled_events, settled);
        assert_eq!(run.closing, history.last().unwrap().map(|v| v.to_string()));
        assert_eq!(
            run.minimum_balance,
            (0..3)
                .map(|i| history.iter().map(|b| b[i]).min().unwrap().to_string())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn balanced_total_with_wrong_entitlements_cannot_sustain_each_person() {
    let events = vec![
        scheduled(0, 0, 2, 4),
        scheduled(0, 1, 2, 6),
        scheduled(0, 2, 0, 3),
        scheduled(0, 2, 1, 7),
        scheduled(1, 0, 2, 4),
        scheduled(1, 1, 2, 6),
    ];
    let run = replay_schedule(&[4, 6, 0], &events, &[]).unwrap();
    assert_eq!(run.first_blocked_event, Some(4));
    assert_eq!(run.opening_total, "10");
    assert!(
        !schedule_opening(3, &events[..4])
            .unwrap()
            .stationary_repeatable
    );
}

#[test]
fn independent_regions_and_account_relabelling_preserve_trace() {
    let events = vec![
        scheduled(0, 0, 1, 7),
        scheduled(0, 2, 3, 11),
        scheduled(0, 1, 0, 7),
        scheduled(0, 3, 2, 11),
    ];
    let run = replay_schedule(&[7, 0, 11, 0], &events, &[0]).unwrap();
    assert_eq!(run.closing[2..], ["11", "0"]);
    let permutation = [2, 0, 3, 1];
    let mapped: Vec<_> = events
        .iter()
        .map(|e| {
            scheduled(
                e.period,
                permutation[e.movement.from],
                permutation[e.movement.to],
                e.movement.amount,
            )
        })
        .collect();
    let mut balances = [0; 4];
    for (i, b) in [7, 0, 11, 0].into_iter().enumerate() {
        balances[permutation[i]] = b;
    }
    let mapped_run = replay_schedule(&balances, &mapped, &[0]).unwrap();
    for (i, &j) in permutation.iter().enumerate() {
        assert_eq!(run.closing[i], mapped_run.closing[j]);
    }
    assert_eq!(run.settled_events, mapped_run.settled_events);
}

#[test]
fn schedule_validation_rejects_ambiguous_or_invalid_inputs() {
    let events = vec![scheduled(1, 0, 1, 1), scheduled(0, 1, 0, 1)];
    assert!(schedule_opening(2, &events).is_err());
    assert!(replay_schedule(&[1, 0], &events, &[]).is_err());
    let mut event = scheduled(0, 0, 1, 1);
    event.basis = " ".into();
    assert!(schedule_opening(2, &[event]).is_err());
    let event = scheduled(0, 0, 1, 1);
    assert!(replay_schedule(&[1, 0], std::slice::from_ref(&event), &[0, 0]).is_err());
    assert!(replay_schedule(&[1, 0], &[event], &[1]).is_err());
    assert!(schedule_opening(0, &[]).is_err());
    assert!(schedule_opening(10_001, &[]).is_err());
    assert!(schedule_opening(2, &[scheduled(0, 0, 2, 1)]).is_err());
    assert!(schedule_opening(2, &[scheduled(0, 0, 0, 1)]).is_err());
    assert!(schedule_opening(2, &[scheduled(0, 0, 1, 0)]).is_err());
    let run = replay_schedule(&[u64::MAX, u64::MAX], &[scheduled(0, 0, 1, u64::MAX)], &[]).unwrap();
    assert_eq!(
        run.closing,
        vec!["0".to_string(), (2 * u128::from(u64::MAX)).to_string()]
    );
}

mod capabilities {
    use super::*;
    use qazna::horizon::*;
    fn toy() -> Horizon {
        Horizon {
            resources: vec![
                "material".into(),
                "equipment".into(),
                "qualified_capacity".into(),
                "output".into(),
            ],
            initial: vec![4, 1, 1, 0],
            storage_caps: vec![8, 2, 2, 8],
            processes: vec![Process {
                inputs: vec![1, 0, 0, 0],
                outputs: vec![0, 0, 0, 1],
                lead_periods: 1,
            }],
            periods: (0..3)
                .map(|_| Period {
                    supplies: vec![0; 4],
                    needs: vec![0, 0, 0, 4],
                    capacities: vec![4],
                })
                .collect(),
            objective_resource: 3,
        }
    }
    fn replay(
        m: &Horizon,
        starts: &[Vec<i64>],
        deliver: bool,
        exits: &[Withdrawal],
    ) -> Vec<PeriodPlan> {
        let mut stock = m.initial.clone();
        let mut result = vec![];
        for (t, a) in starts.iter().enumerate() {
            for e in exits.iter().filter(|e| e.period == t) {
                stock[e.resource] -= e.quantity;
            }
            for (j, p) in m.processes.iter().enumerate() {
                if t >= p.lead_periods {
                    for (k, n) in stock.iter_mut().enumerate() {
                        *n += p.outputs[k] * starts[t - p.lead_periods][j];
                    }
                }
            }
            let mut delivered = vec![0; stock.len()];
            if deliver {
                delivered[3] = stock[3];
                stock[3] = 0;
            }
            for (j, p) in m.processes.iter().enumerate() {
                for (k, n) in stock.iter_mut().enumerate() {
                    *n -= p.inputs[k] * a[j];
                }
            }
            result.push(PeriodPlan {
                activities: a.clone(),
                deliveries: delivered,
                closing: stock.clone(),
                discarded: vec![0; stock.len()],
            });
        }
        result
    }
    fn limit(resource: usize, use_by_process: Vec<i64>) -> StockCapacity {
        StockCapacity {
            name: format!("stock_{resource}"),
            resource,
            units_per_stock: 2,
            activity_use: use_by_process,
        }
    }
    #[test]
    fn shared_equipment_and_skills_are_not_consumed_or_double_booked() {
        let m = toy();
        let plan = replay(&m, &[vec![2], vec![2], vec![0]], true, &[]);
        let checked = m
            .compile_capabilities(&[limit(1, vec![1]), limit(2, vec![1])], &[])
            .unwrap();
        verify_linear(&checked, &m.vector(&plan).unwrap(), None).unwrap();
        assert!(plan.iter().all(|p| p.closing[1] == 1 && p.closing[2] == 1));
        let mut m = toy();
        m.initial[0] = 8;
        m.processes.push(m.processes[0].clone());
        for t in &mut m.periods {
            t.capacities.push(4);
        }
        let plan = replay(&m, &[vec![2, 2], vec![0, 0], vec![0, 0]], true, &[]);
        verify_linear(&m.compile().unwrap(), &m.vector(&plan).unwrap(), None).unwrap();
        assert!(
            verify_linear(
                &m.compile_capabilities(&[limit(1, vec![1, 1])], &[])
                    .unwrap(),
                &m.vector(&plan).unwrap(),
                None
            )
            .is_err()
        );
    }
    #[test]
    fn training_completion_cannot_be_used_before_the_next_period() {
        let mut m = toy();
        m.initial[2] = 0;
        // Material in index 1 is now an eligible training place, not equipment.
        m.processes.push(Process {
            inputs: vec![0, 1, 0, 0],
            outputs: vec![0, 0, 1, 0],
            lead_periods: 1,
        });
        m.periods.push(m.periods[0].clone());
        for t in &mut m.periods {
            t.capacities = vec![4, 1];
        }
        let caps = m
            .compile_capabilities(&[limit(2, vec![1, 0])], &[])
            .unwrap();
        let good = replay(
            &m,
            &[vec![0, 1], vec![0, 0], vec![2, 0], vec![0, 0]],
            true,
            &[],
        );
        verify_linear(&caps, &m.vector(&good).unwrap(), None).unwrap();
        let premature = replay(
            &m,
            &[vec![0, 1], vec![2, 0], vec![0, 0], vec![0, 0]],
            true,
            &[],
        );
        verify_linear(&m.compile().unwrap(), &m.vector(&premature).unwrap(), None).unwrap();
        assert!(verify_linear(&caps, &m.vector(&premature).unwrap(), None).is_err());
    }
    #[test]
    fn retirement_removes_capacity_before_any_activity() {
        let m = toy();
        let exits = vec![Withdrawal {
            period: 1,
            resource: 1,
            quantity: 1,
            basis: "Retired equipment".into(),
        }];
        let matrix = m
            .compile_capabilities(&[limit(1, vec![1])], &exits)
            .unwrap();
        let good = replay(&m, &[vec![2], vec![0], vec![0]], true, &exits);
        verify_linear(&matrix, &m.vector(&good).unwrap(), None).unwrap();
        let wrong = replay(&m, &[vec![2], vec![2], vec![0]], true, &exits);
        assert!(verify_linear(&matrix, &m.vector(&wrong).unwrap(), None).is_err());
        let mut ex = exits[0].clone();
        ex.quantity = 2;
        let wrong = replay(&m, &[vec![0], vec![0], vec![0]], true, &[ex.clone()]);
        assert!(
            verify_linear(
                &m.compile_capabilities(&[], &[ex]).unwrap(),
                &m.vector(&wrong).unwrap(),
                None
            )
            .is_err()
        );
    }
    #[test]
    fn capability_validation_and_overflow_are_rejected() {
        let m = toy();
        let ex = Withdrawal {
            period: 0,
            resource: 1,
            quantity: 2,
            basis: "exit".into(),
        };
        assert!(m.compile_capabilities(&[], &[ex]).is_err());
        let ex = Withdrawal {
            period: 1,
            resource: 1,
            quantity: 1,
            basis: "exit".into(),
        };
        assert!(m.compile_capabilities(&[], &[ex.clone(), ex]).is_err());
        let good = limit(1, vec![1]);
        assert!(m.compile_capabilities(&[good.clone(), good], &[]).is_err());
        for bad in [limit(9, vec![1]), limit(1, vec![-1]), limit(1, vec![])] {
            assert!(m.compile_capabilities(&[bad], &[]).is_err());
        }
        let mut m = toy();
        m.initial[1] = 2;
        let mut bad = limit(1, vec![1]);
        bad.units_per_stock = i64::MAX;
        assert_eq!(
            m.compile_capabilities(&[bad], &[]).unwrap_err(),
            MathError::Overflow
        );
    }
    fn links(capital: bool) -> Vec<CashLink> {
        vec![
            CashLink {
                period: 0,
                source: PaymentSource::Input {
                    process: 0,
                    resource: 0,
                },
                quantity: 2,
                amount: "6".into(),
            },
            CashLink {
                period: 1,
                source: PaymentSource::Input {
                    process: 0,
                    resource: 0,
                },
                quantity: 2,
                amount: "6".into(),
            },
            CashLink {
                period: 1,
                source: if capital {
                    PaymentSource::Capital {
                        process: 0,
                        resource: 3,
                    }
                } else {
                    PaymentSource::Provision { resource: 3 }
                },
                quantity: 2,
                amount: "14".into(),
            },
            CashLink {
                period: 2,
                source: if capital {
                    PaymentSource::Capital {
                        process: 0,
                        resource: 3,
                    }
                } else {
                    PaymentSource::Provision { resource: 3 }
                },
                quantity: 2,
                amount: "14".into(),
            },
        ]
    }
    #[test]
    fn goods_payment_link_requires_complete_unique_exact_amounts() {
        let m = toy();
        let p = replay(&m, &[vec![2], vec![2], vec![0]], true, &[]);
        let base = links(false);
        m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &base).unwrap();
        let mut bad = base.clone();
        bad[0].amount = "7".into();
        assert!(m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &bad).is_err());
        let mut bad = base.clone();
        bad[0].quantity = 3;
        assert!(m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &bad).is_err());
        let mut bad = base.clone();
        bad.push(base[0].clone());
        assert!(m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &bad).is_err());
        assert!(
            m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &base[1..])
                .is_err()
        );
        let mut bad = base.clone();
        bad[0].amount = "06".into();
        assert!(m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &bad).is_err());
        let mut bad = base.clone();
        bad[0].source = PaymentSource::Input {
            process: 0,
            resource: 2,
        };
        assert!(m.verify_cash_links(&p, &[3, 0, 0, 7], &[], &bad).is_err());
    }
    #[test]
    fn capital_payment_is_after_completion_and_not_counted_twice() {
        let m = toy();
        let p = replay(&m, &[vec![2], vec![2], vec![0]], false, &[]);
        let base = links(true);
        m.verify_cash_links(&p, &[3, 0, 0, 7], &[3], &base).unwrap();
        let mut bad = base.clone();
        bad[2].period = 0;
        assert!(m.verify_cash_links(&p, &[3, 0, 0, 7], &[3], &bad).is_err());
        let delivered = replay(&m, &[vec![2], vec![2], vec![0]], true, &[]);
        let mut double = links(true);
        double.extend_from_slice(&links(false)[2..]);
        assert!(
            m.verify_cash_links(&delivered, &[3, 0, 0, 7], &[3], &double)
                .is_err()
        );
        assert!(
            m.verify_cash_links(&p, &[3, -1, 0, 7], &[3], &base)
                .is_err()
        );
        assert!(
            m.verify_cash_links(&p, &[3, 0, 0, 7], &[3, 3], &base)
                .is_err()
        );
    }
}

mod trimming {
    use qazna::{horizon::*, mathematics::verify_linear, optimization::trim_plan};

    // Independent full-horizon replay, deliberately not an incremental delta
    // implementation. Recomputes every incoming batch from the entire history.
    fn replay(
        m: &Horizon,
        activities: &[Vec<i64>],
        deliveries: &[Vec<i64>],
        exits: &[Withdrawal],
    ) -> Option<Vec<PeriodPlan>> {
        let mut stock = m.initial.clone();
        let mut result = Vec::new();
        for t in 0..m.periods.len() {
            let mut closing = Vec::new();
            let mut discarded = Vec::new();
            for (k, &opening) in stock.iter().enumerate() {
                let exit: i64 = exits
                    .iter()
                    .filter(|e| e.period == t && e.resource == k)
                    .map(|e| e.quantity)
                    .sum();
                if opening < exit {
                    return None;
                }
                let mut n = opening - exit + m.periods[t].supplies[k] - deliveries[t][k];
                for (j, p) in m.processes.iter().enumerate() {
                    n -= p.inputs[k] * activities[t][j];
                    if t >= p.lead_periods {
                        n += p.outputs[k] * activities[t - p.lead_periods][j];
                    }
                }
                if n < 0 {
                    return None;
                }
                closing.push(n.min(m.storage_caps[k]));
                discarded.push((n - m.storage_caps[k]).max(0));
            }
            stock = closing.clone();
            result.push(PeriodPlan {
                activities: activities[t].clone(),
                deliveries: deliveries[t].clone(),
                closing,
                discarded,
            });
        }
        Some(result)
    }
    fn toy() -> Horizon {
        Horizon {
            resources: vec!["fuel".into(), "energy".into()],
            initial: vec![100, 0],
            storage_caps: vec![100, 0],
            processes: vec![Process {
                inputs: vec![1, 0],
                outputs: vec![0, 10],
                lead_periods: 1,
            }],
            periods: (0..6)
                .map(|t| Period {
                    supplies: vec![0, 0],
                    needs: vec![0, if t > 0 { 13 } else { 0 }],
                    capacities: vec![4],
                })
                .collect(),
            objective_resource: 1,
        }
    }
    #[test]
    fn trim_keeps_every_delivery_and_reserve_reduces_avoidable_output() {
        let m = toy();
        let starts: Vec<_> = (0..6).map(|t| vec![if t < 5 { 4 } else { 0 }]).collect();
        let delivered: Vec<_> = m.periods.iter().map(|p| p.needs.clone()).collect();
        let seed = replay(&m, &starts, &delivered, &[]).unwrap();
        let seed_json = serde_json::to_value(&seed).unwrap();
        let (p, e) = trim_plan(&m, &seed, &[], &[], &[80, 0], &[0], 4).unwrap();
        assert_eq!(e.removed_starts_by_process, vec!["10"]);
        assert!(e.coordinate_irreducible);
        assert_eq!(p.iter().map(|p| p.discarded[1]).sum::<i64>(), 35); // indivisible 10-unit batches
        assert_eq!(p.last().unwrap().closing, vec![90, 0]);
        assert_eq!(
            p.iter().map(|p| p.deliveries.clone()).collect::<Vec<_>>(),
            delivered
        );
        assert_eq!(serde_json::to_value(&seed).unwrap(), seed_json);
        assert_eq!(
            serde_json::to_value(
                replay(
                    &m,
                    &p.iter().map(|p| p.activities.clone()).collect::<Vec<_>>(),
                    &delivered,
                    &[]
                )
                .unwrap()
            )
            .unwrap(),
            serde_json::to_value(&p).unwrap()
        );
        let (_, budget) = trim_plan(&m, &seed, &[], &[], &[80, 0], &[0], 1).unwrap();
        assert!(!budget.coordinate_irreducible); // budget exhaustion is not a certificate
        let (again, e) = trim_plan(&m, &p, &[], &[], &[80, 0], &[0], 4).unwrap();
        assert_eq!(e.accepted_changes, 0);
        assert_eq!(
            serde_json::to_value(again).unwrap(),
            serde_json::to_value(p).unwrap()
        );
    }
    #[test]
    fn trim_protects_delayed_capabilities_and_future_retirement() {
        let m = Horizon {
            resources: vec!["raw".into(), "equipment".into(), "food".into()],
            initial: vec![100, 0, 0],
            storage_caps: vec![100, 10, 0],
            processes: vec![
                Process {
                    inputs: vec![1, 0, 0],
                    outputs: vec![0, 1, 0],
                    lead_periods: 2,
                },
                Process {
                    inputs: vec![1, 0, 0],
                    outputs: vec![0, 0, 10],
                    lead_periods: 1,
                },
            ],
            periods: (0..6)
                .map(|t| Period {
                    supplies: vec![0; 3],
                    needs: vec![0, 0, if t == 4 { 20 } else { 0 }],
                    capacities: vec![3, 3],
                })
                .collect(),
            objective_resource: 2,
        };
        let exits = vec![Withdrawal {
            period: 3,
            resource: 1,
            quantity: 1,
            basis: "Retirement before work".into(),
        }];
        let limits = vec![StockCapacity {
            name: "machine".into(),
            resource: 1,
            units_per_stock: 2,
            activity_use: vec![0, 1],
        }];
        let mut a = vec![vec![0, 0]; 6];
        a[0][0] = 3;
        a[3][1] = 2;
        let d = m
            .periods
            .iter()
            .map(|p| p.needs.clone())
            .collect::<Vec<_>>();
        let seed = replay(&m, &a, &d, &exits).unwrap();
        let (p, _) = trim_plan(&m, &seed, &limits, &exits, &[0, 0, 0], &[0, 1], 4).unwrap();
        assert_eq!(p[0].activities[0], 2); // one retired, one needed; cannot trim to one
        verify_linear(
            &m.compile_capabilities(&limits, &exits).unwrap(),
            &m.vector(&p).unwrap(),
            None,
        )
        .unwrap();
        let mut forged = p.clone();
        forged[0].activities[0] = 1;
        assert!(trim_plan(&m, &forged, &limits, &exits, &[0, 0, 0], &[0, 1], 4).is_err());
    }
    #[test]
    fn trimming_work_releases_the_cached_upstream_capacity_requirement() {
        let m = Horizon {
            resources: vec!["raw".into(), "equipment".into(), "food".into()],
            initial: vec![100, 0, 0],
            storage_caps: vec![100, 10, 0],
            processes: vec![
                Process {
                    inputs: vec![1, 0, 0],
                    outputs: vec![0, 1, 0],
                    lead_periods: 2,
                },
                Process {
                    inputs: vec![1, 0, 0],
                    outputs: vec![0, 0, 10],
                    lead_periods: 1,
                },
            ],
            periods: (0..6)
                .map(|t| Period {
                    supplies: vec![0; 3],
                    needs: vec![0, 0, if t == 4 { 10 } else { 0 }],
                    capacities: vec![3, 3],
                })
                .collect(),
            objective_resource: 2,
        };
        let exits = vec![Withdrawal {
            period: 3,
            resource: 1,
            quantity: 1,
            basis: "Retirement".into(),
        }];
        let limits = vec![StockCapacity {
            name: "machine".into(),
            resource: 1,
            units_per_stock: 1,
            activity_use: vec![0, 1],
        }];
        let mut starts = vec![vec![0, 0]; 6];
        starts[0][0] = 3;
        starts[3][1] = 2;
        let d = m
            .periods
            .iter()
            .map(|p| p.needs.clone())
            .collect::<Vec<_>>();
        let seed = replay(&m, &starts, &d, &exits).unwrap();
        let (p, _) = trim_plan(&m, &seed, &limits, &exits, &[0, 0, 0], &[1, 0], 4).unwrap();
        assert_eq!(p[3].activities[1], 1);
        assert_eq!(p[0].activities[0], 2); // refreshed capacity plus one retirement
        let fresh = replay(
            &m,
            &p.iter().map(|p| p.activities.clone()).collect::<Vec<_>>(),
            &d,
            &exits,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(fresh).unwrap(),
            serde_json::to_value(p).unwrap()
        );
    }

    #[test]
    fn trim_does_not_trade_a_terminal_reserve_for_less_production() {
        let mut m = toy();
        m.storage_caps[1] = 100;
        let a: Vec<_> = (0..6).map(|t| vec![if t < 5 { 4 } else { 0 }]).collect();
        let d = m
            .periods
            .iter()
            .map(|p| p.needs.clone())
            .collect::<Vec<_>>();
        let seed = replay(&m, &a, &d, &[]).unwrap();
        let floor = seed.last().unwrap().closing.clone();
        let (p, e) = trim_plan(&m, &seed, &[], &[], &floor, &[0], 4).unwrap();
        assert!(p.last().unwrap().closing[1] >= floor[1]);
        assert!(e.removed_starts_by_process[0].parse::<i64>().unwrap() <= 3);
        for (floors, order, budget) in [
            (vec![-1, 0], vec![0], 4),
            (vec![0], vec![0], 4),
            (vec![101, 0], vec![0], 4),
            (vec![0, 0], vec![0, 0], 4),
            (vec![0, 0], vec![1], 4),
            (vec![0, 0], vec![], 4),
            (vec![0, 0], vec![0], 0),
        ] {
            assert!(trim_plan(&m, &seed, &[], &[], &floors, &order, budget).is_err());
        }
    }
    #[test]
    fn coordinate_certificate_does_not_claim_global_optimum() {
        // A useless round trip cannot be removed one start at a time while
        // preserving terminal A. Removing BOTH starts is feasible and better.
        let m = Horizon {
            resources: vec!["A".into(), "B".into()],
            initial: vec![2, 0],
            storage_caps: vec![2, 2],
            processes: vec![
                Process {
                    inputs: vec![2, 0],
                    outputs: vec![0, 2],
                    lead_periods: 1,
                },
                Process {
                    inputs: vec![0, 2],
                    outputs: vec![2, 0],
                    lead_periods: 1,
                },
            ],
            periods: vec![
                Period {
                    supplies: vec![0; 2],
                    needs: vec![0; 2],
                    capacities: vec![1; 2]
                };
                3
            ],
            objective_resource: 0,
        };
        let d = vec![vec![0, 0]; 3];
        let seed = replay(&m, &[vec![1, 0], vec![0, 1], vec![0, 0]], &d, &[]).unwrap();
        let (local, e) = trim_plan(&m, &seed, &[], &[], &[2, 0], &[0, 1], 4).unwrap();
        assert!(e.coordinate_irreducible);
        assert_eq!(e.accepted_changes, 0);
        let global = replay(&m, &vec![vec![0, 0]; 3], &d, &[]).unwrap();
        verify_linear(&m.compile().unwrap(), &m.vector(&global).unwrap(), None).unwrap();
        assert_eq!(
            global.last().unwrap().closing,
            local.last().unwrap().closing
        );
        assert!(
            global.iter().flat_map(|p| &p.activities).sum::<i64>()
                < local.iter().flat_map(|p| &p.activities).sum::<i64>()
        );
    }

    #[test]
    fn trim_matches_exhaustive_coordinate_oracle_with_recycling_and_overflow() {
        // Seeded variants include a resource on both sides of a recipe. Exhaust
        // every possible integer reduction; do not assume feasibility monotonicity.
        for case in 0..32 {
            let m = Horizon {
                resources: vec!["raw".into(), "intermediate".into(), "service".into()],
                initial: vec![200, 30, 0],
                storage_caps: vec![200, 10 + case % 7, 0],
                processes: vec![
                    Process {
                        inputs: vec![2, 0, 0],
                        outputs: vec![case % 3, 3, 0],
                        lead_periods: 1 + case as usize % 2,
                    },
                    Process {
                        inputs: vec![0, 1, 0],
                        outputs: vec![0, 0, 2],
                        lead_periods: 1,
                    },
                ],
                periods: (0..8)
                    .map(|t| Period {
                        supplies: vec![0, 4, 0],
                        needs: vec![0, 0, if t > 0 { 2 } else { 0 }],
                        capacities: vec![4, 4],
                    })
                    .collect(),
                objective_resource: 2,
            };
            let a: Vec<_> = (0..8)
                .map(|t| {
                    vec![
                        if t + m.processes[0].lead_periods < 8 {
                            3
                        } else {
                            0
                        },
                        if t < 7 { 2 } else { 0 },
                    ]
                })
                .collect();
            let d = m
                .periods
                .iter()
                .map(|p| p.needs.clone())
                .collect::<Vec<_>>();
            let seed = replay(&m, &a, &d, &[]).unwrap();
            let floor = seed.last().unwrap().closing.clone();
            let matrix = m.compile().unwrap();
            let mut oracle = seed.clone();
            for day in (0..8).rev() {
                for process in [1, 0] {
                    let mut best = oracle.clone();
                    for q in 1..=oracle[day].activities[process] {
                        let mut starts = oracle
                            .iter()
                            .map(|p| p.activities.clone())
                            .collect::<Vec<_>>();
                        starts[day][process] -= q;
                        if let Some(candidate) = replay(&m, &starts, &d, &[])
                            && candidate
                                .last()
                                .unwrap()
                                .closing
                                .iter()
                                .zip(&floor)
                                .all(|(a, b)| a >= b)
                            && verify_linear(&matrix, &m.vector(&candidate).unwrap(), None).is_ok()
                        {
                            best = candidate;
                        }
                    }
                    oracle = best;
                }
            }
            let (actual, _) = trim_plan(&m, &seed, &[], &[], &floor, &[1, 0], 1).unwrap();
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(oracle).unwrap(),
                "case {case}"
            );
        }
    }
}
