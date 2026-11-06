//! Reproducible five-year stress study; synthetic assumptions, not country forecasts.
use qazna::{horizon::*, mathematics::*};
use serde_json::json;
use std::{env, fs};

fn study(name: &str) -> serde_json::Value {
    let population = 100_000_000i64;
    let p = population;
    let days = 1825;
    let mut model = Horizon {
        resources: vec![
            "grain_kg",
            "water_litre",
            "energy_kwh",
            "meal",
            "care_hour",
            "learning_hour",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        initial: vec![2 * p, 2 * p, 2 * p, 3 * p, 0, 0],
        storage_caps: vec![3 * p, 6 * p, 3 * p, 3 * p, 0, 0],
        processes: vec![Process {
            inputs: vec![2, 1, 1, 0, 0, 0],
            outputs: vec![0, 0, 0, 3, 0, 0],
            lead_periods: 1,
        }],
        periods: Vec::new(),
        objective_resource: 3,
    };
    let mut plan: Vec<PeriodPlan> = Vec::new();
    let mut previous = model.initial.clone();
    let mut first_unmet = None;
    let mut unmet = [0i128; 6];
    let mut discarded = [0i128; 6];
    let mut requested = [0i128; 6];
    let mut yearly = Vec::new();
    let mut minimum_food_coverage_ppm = 1_000_000i128;
    for day in 0..days {
        let mut data = Period {
            supplies: vec![p, 3 * p, 2 * p, 0, p / 10, p / 5],
            needs: vec![0, 2 * p, p, p, p / 10, p / 5],
            capacities: vec![p / 2],
        };
        if name == "production_outage" && (365..405).contains(&day) {
            data.capacities[0] = 0;
        }
        if name == "choice_and_need_surge" && (600..630).contains(&day) {
            data.needs[3] = 2 * p;
        }
        if name == "input_disruption" && (900..930).contains(&day) {
            data.supplies[2] = 0;
        }
        let mut available: Vec<i64> = previous
            .iter()
            .zip(&data.supplies)
            .map(|(a, b)| a + b)
            .collect();
        if day > 0 {
            available[3] += plan[day - 1].activities[0] * 3;
        }
        // Causal rule: only today's observed demand is extrapolated to tomorrow.
        // There is no access to the next period's shock while choosing starts.
        let deliveries: Vec<_> = available
            .iter()
            .zip(&data.needs)
            .map(|(a, b)| (*a).min(*b))
            .collect();
        for k in 0..6 {
            available[k] -= deliveries[k];
            let missing = data.needs[k] - deliveries[k];
            unmet[k] += i128::from(missing);
            requested[k] += i128::from(data.needs[k]);
            if missing > 0 && first_unmet.is_none() {
                first_unmet = Some(day);
            }
        }
        minimum_food_coverage_ppm = minimum_food_coverage_ppm
            .min(i128::from(deliveries[3]) * 1_000_000 / i128::from(data.needs[3]));
        let desired = (3 * data.needs[3] - available[3]).max(0).div_euclid(3)
            + i64::from((3 * data.needs[3] - available[3]).max(0) % 3 != 0);
        let activity = if day + 1 == days {
            0
        } else {
            desired
                .min(data.capacities[0])
                .min(available[0] / 2)
                .min(available[1])
                .min(available[2])
        };
        available[0] -= 2 * activity;
        available[1] -= activity;
        available[2] -= activity;
        let waste: Vec<_> = available
            .iter()
            .zip(&model.storage_caps)
            .map(|(a, b)| (a - b).max(0))
            .collect();
        let closing: Vec<_> = available.iter().zip(&waste).map(|(a, b)| a - b).collect();
        for k in 0..6 {
            discarded[k] += i128::from(waste[k]);
        }
        previous = closing.clone();
        plan.push(PeriodPlan {
            activities: vec![activity],
            deliveries,
            closing,
            discarded: waste,
        });
        model.periods.push(data);
        if (day + 1) % 365 == 0 {
            yearly.push(json!({"year":(day+1)/365,"cumulative_unmet":unmet.iter().map(ToString::to_string).collect::<Vec<_>>(),"closing_stock":previous}));
        }
    }
    let matrix = model.compile().unwrap();
    let primal = model.vector(&plan).unwrap();
    let feasible = verify_linear(&matrix, &primal, None).unwrap();
    let dual = model.full_coverage_dual(&matrix);
    let certificate = verify_linear(&matrix, &primal, Some(&dual));
    let mut full_provision_impossible = false;
    if name == "production_outage" {
        // A 40-day window cannot meet all meals even if it starts with the
        // maximum allowed stock and maximum already-started incoming output.
        let mut required = matrix.clone();
        let mut weights = vec![0; required.rows.len()];
        let width = 3 * model.resources.len() + model.processes.len();
        for (i, row) in required.rows.iter().enumerate() {
            let parts: Vec<_> = row.name.split(':').collect();
            let t: usize = parts[1].parse().unwrap();
            let k: usize = parts[2].parse().unwrap();
            if parts[0] == "balance" && k == 3 && (365..405).contains(&t) {
                weights[i] = 1;
            }
            if parts[0] == "storage" && k == 3 && t == 364 {
                weights[i] = 1;
            }
            if parts[0] == "capacity" && (364..404).contains(&t) {
                weights[i] = 3;
            }
        }
        for t in 365..405 {
            required.rows.push(Row {
                name: format!("mandatory_meals:{t}"),
                terms: vec![(t * width + 3, -1)],
                sense: Sense::AtMost,
                rhs: -model.periods[t].needs[3],
            });
            weights.push(1);
        }
        verify_infeasible(
            &required,
            &RationalVector {
                numerators: weights,
                denominator: 1,
            },
        )
        .unwrap();
        full_provision_impossible = true;
    }
    let mut tampered = primal.clone();
    tampered.numerators[3] += 1; // forged additional delivery: must not be accepted
    assert!(verify_linear(&matrix, &tampered, None).is_err());
    json!({"scenario":name,"population_scale":population,"periods":days,"resources":model.resources,
        "assumptions":"Fixed synthetic population; quantities are test inputs, not validated needs norms. Exogenous supplies are not a complete production economy. Rolling one-day forecast; no model calls.",
        "initial":model.initial,"storage_caps":model.storage_caps,"processes":model.processes,
        "matrix":{"variables":matrix.objective.len(),"rows":matrix.rows.len(),"nonzero_coefficients":matrix.rows.iter().map(|r|r.terms.len()).sum::<usize>()},
        "verification":feasible,"food_objective_optimality_proven":certificate.is_ok(),
        "optimality_scope":"Total meals only, under supplied constraints. No stress-case optimum asserted when this certificate fails.",
        "full_food_provision_infeasibility_proven":full_provision_impossible,"first_unmet_day_zero_based":first_unmet,"minimum_daily_food_coverage_ppm":minimum_food_coverage_ppm.to_string(),
        "requested":requested.iter().map(ToString::to_string).collect::<Vec<_>>(),"unmet":unmet.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "discarded":discarded.iter().map(ToString::to_string).collect::<Vec<_>>(),"years":yearly,"forged_delivery_rejected":true})
}
fn main() {
    let scenarios: Vec<_> = [
        "baseline",
        "production_outage",
        "choice_and_need_surge",
        "input_disruption",
    ]
    .into_iter()
    .map(study)
    .collect();
    assert!(
        scenarios[0]["food_objective_optimality_proven"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenarios
            .iter()
            .skip(1)
            .all(|r| !r["first_unmet_day_zero_based"].is_null())
    );
    let chain = vec![
        Movement {
            from: 0,
            to: 1,
            amount: 100,
        },
        Movement {
            from: 1,
            to: 2,
            amount: 100,
        },
    ];
    let mut circle = chain.clone();
    circle.push(Movement {
        from: 2,
        to: 0,
        amount: 100,
    });
    let needs = [(1, 1500), (2, 1500), (3, 900), (4, 500), (5, 500), (6, 200)];
    let report = json!({"version":1,"kind":"synthetic_exact_mathematical_study","model_calls":0,"scenarios":scenarios,
        "liquidity_without_closing_edge":liquidity_bound(3,&chain,1825).unwrap(),
        "liquidity_with_hypothetical_closing_edge":liquidity_bound(3,&circle,1825).unwrap(),
        "circulation_scope":"The final edge is a mathematical comparison, not an adopted tax, fee or monetary rule.",
        "quota_counterexample":{"needs":needs,"supply_25":proportional_quota(&needs,25).unwrap(),"supply_26":proportional_quota(&needs,26).unwrap(),"divisor_supply_25":divisor_allocation(&needs,25).unwrap(),"divisor_supply_26":divisor_allocation(&needs,26).unwrap()}});
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "research/horizon-results.json".into());
    fs::write(
        &output,
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!(
        "Four 1825-period exact-feasibility scenarios written to {output}; not a national economic validation"
    );
}
