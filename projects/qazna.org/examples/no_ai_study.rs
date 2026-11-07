//! Offline research calculations only: no DB, model, network or economic authority.
//! Floating point is used for explicitly approximate USD cost estimates, never postings.
use serde_json::{Value, json};
use std::{env, fs};

fn number(v: &Value, key: &str) -> f64 {
    let n = v[key]
        .as_f64()
        .unwrap_or_else(|| panic!("missing number {key}"));
    assert!(n.is_finite() && n >= 0.0, "invalid {key}");
    n
}
fn usd(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn api_year(
    events_day: f64,
    fraction: f64,
    attempts: f64,
    input: f64,
    output: f64,
    m: &Value,
) -> f64 {
    events_day
        * 365.0
        * fraction
        * attempts
        * (input * number(m, "input_usd_per_million")
            + output * number(m, "output_usd_per_million"))
        / 1_000_000.0
}

// Exact bounded integer optimization: maximize x+y, then minimize energy x+2y.
// For each x, the largest feasible y dominates all smaller y at that x.
// This is an auditable small reference, not a scalable general MIP solver.
fn solve(grain: u64, energy: u64, cap_x: u64, cap_y: u64, demand: u64) -> (u64, u64) {
    let mut best = (0, 0);
    for x in 0..=cap_x.min(grain / 2).min(energy).min(demand) {
        let y = cap_y
            .min(grain - 2 * x)
            .min((energy - x) / 2)
            .min(demand - x);
        if x + y > best.0 + best.1 || (x + y == best.0 + best.1 && x + 2 * y < best.0 + 2 * best.1)
        {
            best = (x, y);
        }
    }
    best
}

fn calculate(c: &Value) -> Value {
    let h = &c["infrastructure"];
    for name in [
        "target_utilization",
        "planning_utilization",
        "self_hosted_target_utilization",
    ] {
        assert!(number(h, name) > 0.0 && number(h, name) <= 1.0);
    }
    assert!(number(h, "people_per_planning_domain") > 0.0);
    assert!(number(h, "self_hosted_effective_requests_second_per_gpu") > 0.0);
    assert!(number(h, "productive_hours_per_reviewer_year") > 0.0);
    assert!(number(h, "ai_case_review_fraction") <= 1.0);
    assert!(number(c, "attempt_multiplier") >= 1.0);
    let mut rows = Vec::new();
    for population in c["populations"].as_array().expect("populations") {
        let p = population.as_u64().expect("positive integer population");
        assert!(p > 0);
        let daily = p as f64 * number(c, "commands_per_person_day");
        let avg = daily / 86400.0;
        let cpu = (avg * number(h, "cpu_ms_per_command") / 1000.0 * number(h, "peak_factor")
            / number(h, "target_utilization")
            * number(h, "replica_capacity_factor"))
        .ceil()
        .max(number(h, "minimum_vcpu"));
        let domains = (p as f64 / number(h, "people_per_planning_domain")).ceil();
        let planning_cpu = (domains
            * number(h, "planning_runs_per_domain_day")
            * number(h, "cpu_seconds_per_plan")
            / 86400.0
            / number(h, "planning_utilization"))
        .ceil()
        .max(1.0);
        let compute = (cpu + planning_cpu) * 8760.0 * number(h, "usd_per_vcpu_hour");
        let stored_gb = daily
            * 365.0
            * number(h, "stored_bytes_per_command")
            * number(h, "storage_copy_factor")
            / 1e9;
        // Linear growth from empty at t=0, one-year rolling retention thereafter.
        let storage_steady = stored_gb * number(h, "usd_per_decimal_gb_month") * 12.0;
        let storage_first = storage_steady / 2.0;
        let network_first = (compute + storage_first) * number(h, "network_observability_fraction");
        let fte = number(h, "base_fte")
            + (p as f64 / 1e6 * number(h, "additional_fte_per_million_people")).ceil();
        let labour = fte * number(h, "loaded_usd_per_fte_year");
        let assurance = number(h, "fixed_external_review_usd_year")
            + p as f64 * number(h, "review_usd_person_year");
        let contingency = 1.0 + number(h, "contingency_fraction");
        let first = (compute + storage_first + network_first + labour + assurance) * contingency;
        let steady = ((compute + storage_steady)
            * (1.0 + number(h, "network_observability_fraction"))
            + labour
            + assurance)
            * contingency;
        let build = (number(h, "initial_build_person_years")
            * number(h, "loaded_usd_per_fte_year")
            + number(h, "initial_external_assurance_usd"))
            * contingency;
        let ai_fixed = number(h, "optional_ai_extra_fte") * number(h, "loaded_usd_per_fte_year")
            + number(h, "optional_ai_evaluation_usd_year");
        let mut models = Vec::new();
        for m in c["models"].as_array().expect("models") {
            for fraction in c["escalation_fractions"].as_array().expect("fractions") {
                let f = fraction.as_f64().expect("fraction");
                assert!((0.0..=1.0).contains(&f));
                let api = api_year(
                    daily,
                    f,
                    number(c, "attempt_multiplier"),
                    number(c, "input_tokens"),
                    number(c, "billed_output_tokens"),
                    m,
                );
                // Review cases, not repeated API attempts. This is additional AI review only.
                let review_fte = daily
                    * 365.0
                    * f
                    * number(h, "ai_case_review_fraction")
                    * number(h, "minutes_per_ai_case_review")
                    / (60.0 * number(h, "productive_hours_per_reviewer_year"));
                let review_cost = review_fte * number(h, "loaded_usd_per_fte_year");
                let extra = if f == 0.0 {
                    0.0
                } else {
                    (api + ai_fixed + review_cost) * contingency
                };
                let gpu = if f == 0.0 {
                    0.0
                } else {
                    (avg * f * number(c, "attempt_multiplier") * number(h, "peak_factor")
                        / number(h, "self_hosted_effective_requests_second_per_gpu")
                        / number(h, "self_hosted_target_utilization"))
                    .ceil()
                    .max(number(h, "self_hosted_gpu_count_minimum"))
                };
                let local_extra = if f == 0.0 {
                    0.0
                } else {
                    (gpu * 8760.0 * number(h, "self_hosted_gpu_usd_hour") + ai_fixed + review_cost)
                        * contingency
                };
                models.push(json!({"model":m["name"], "fraction":f,
                    "additional_case_review_fte":review_fte,
                    "additional_case_review_year_usd":usd(review_cost),
                    "attempts_day":daily*f*number(c,"attempt_multiplier"),
                    "api_only_year_usd":usd(api), "hybrid_first_year_recurring_usd":usd(first+extra),
                    "hybrid_five_year_plus_build_usd":usd(build+first+4.0*steady+5.0*extra),
                    "self_hosted_gpu_assumption":gpu,
                    "self_hosted_first_year_recurring_usd":usd(first+local_extra)}));
            }
        }
        rows.push(json!({"population":p,"commands_day":daily,"average_commands_second":avg,
            "peak_commands_second":avg*number(h,"peak_factor"),"ledger_vcpu_assumption":cpu,
            "planning_vcpu_assumption":planning_cpu,"planning_domains":domains,
            "stored_end_year_decimal_tb":stored_gb/1000.0,
            "compute_year_usd":usd(compute),"storage_first_year_usd":usd(storage_first),
            "network_observability_first_year_usd":usd(network_first),"fte_assumption":fte,
            "labour_year_usd":usd(labour),"external_assurance_year_usd":usd(assurance),
            "no_ai_first_year_recurring_usd":usd(first),"no_ai_steady_year_recurring_usd":usd(steady),
            "initial_build_usd":usd(build),"no_ai_five_year_plus_build_usd":usd(build+first+4.0*steady),
            "five_year_no_deletion_storage_extra_usd":usd(8.0*storage_steady*(1.0+number(h,"network_observability_fraction"))*contingency),
            "ai_variants":models}));
    }
    let cases = [
        ("balanced", 240, 240, 100, 100, 200),
        ("energy_shortage", 240, 150, 100, 100, 200),
        ("grain_shortage", 120, 240, 100, 100, 200),
        ("plant_outage", 240, 240, 40, 100, 200),
    ];
    let optimization: Vec<_> = cases
        .iter()
        .map(|&(name, g, e, cx, cy, d)| {
            let (x, y) = solve(g, e, cx, cy, d);
            let gx = cx.min(g / 2).min(e).min(d);
            let gy = cy.min(g - 2 * gx).min((e - gx) / 2).min(d - gx);
            json!({"case":name,"grain":g,"energy":e,"cap_x":cx,"cap_y":cy,"demand":d,
            "optimal_x":x,"optimal_y":y,"delivered":x+y,"unmet":d-x-y,
            "grain_used":2*x+y,"energy_used":x+2*y,"greedy_delivered":gx+gy})
        })
        .collect();
    json!({"as_of":c["as_of"],"scope":c["scope"],"inputs":c,
        "costs":rows,"integer_optimization":optimization,
        "limitations":"Synthetic budget assumptions and bounded exact optimization; not a production benchmark or proven economic effectiveness. No AI called."})
}

fn study(c: &Value) -> Value {
    let mut result = calculate(c);
    let mut sensitivity = Vec::new();
    // Isolate technology uncertainty; keep staffing, prices and demand fixed.
    for (name, cpu_ms, bytes, copies, plan_seconds) in [
        ("lighter", 2.0, 500.0, 2.0, 10.0),
        ("base", 10.0, 2000.0, 4.0, 60.0),
        ("heavier", 50.0, 8000.0, 6.0, 600.0),
    ] {
        let mut changed = c.clone();
        changed["populations"] = json!([100_000_000]);
        for (key, value) in [
            ("cpu_ms_per_command", cpu_ms),
            ("stored_bytes_per_command", bytes),
            ("storage_copy_factor", copies),
            ("cpu_seconds_per_plan", plan_seconds),
        ] {
            changed["infrastructure"][key] = json!(value);
        }
        let calc = calculate(&changed);
        let row = &calc["costs"][0];
        sensitivity.push(json!({"case":name,"cpu_ms_per_command":cpu_ms,
            "bytes_per_command":bytes,"copies":copies,"cpu_seconds_per_plan":plan_seconds,
            "no_ai_first_year_recurring_usd":row["no_ai_first_year_recurring_usd"],
            "no_ai_five_year_plus_build_usd":row["no_ai_five_year_plus_build_usd"]}));
    }
    result["technology_sensitivity_100m"] = json!(sensitivity);
    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "research/computation-cost-inputs.json".into());
    let input: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    println!("{}", serde_json::to_string_pretty(&study(&input))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_results_match_current_inputs_and_calculation() {
        let c: Value =
            serde_json::from_str(include_str!("../research/computation-cost-inputs.json")).unwrap();
        // Check published serialization without an additional float-parsing round.
        let saved = include_str!("../research/computation-cost-results.json");
        let generated = serde_json::to_string_pretty(&study(&c)).unwrap();
        assert!(
            generated == saved.trim(),
            "Regenerate computation-cost-results.json"
        );
    }

    #[test]
    fn exact_optimizer_matches_exhaustive_small_space_and_dual_bound() {
        for g in 0..9 {
            for e in 0..9 {
                for cx in 0..4 {
                    for cy in 0..4 {
                        for d in 0..9 {
                            let (x, y) = solve(g, e, cx, cy, d);
                            let feasible = |a, b| 2 * a + b <= g && a + 2 * b <= e && a + b <= d;
                            assert!(x <= cx && y <= cy && feasible(x, y));
                            for a in 0..=cx {
                                for b in 0..=cy {
                                    if feasible(a, b) {
                                        assert!(x + y >= a + b);
                                        if x + y == a + b {
                                            assert!(x + 2 * y <= a + 2 * b);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let (x, y) = solve(240, 240, 100, 100, 200);
        assert_eq!((x, y), (80, 80));
        assert_eq!(3 * (x + y), 240 + 240); // Sum of both resource constraints proves optimality.
    }

    #[test]
    fn pricing_units_and_no_ai_identity() {
        let c: Value =
            serde_json::from_str(include_str!("../research/computation-cost-inputs.json")).unwrap();
        let model = json!({"input_usd_per_million":2,"output_usd_per_million":10});
        assert!((api_year(1e9, 0.001, 1.1, 2000.0, 500.0, &model) - 3_613_500.0).abs() < 0.001);
        assert_eq!(api_year(1e9, 0.0, 1.1, 2000.0, 500.0, &model), 0.0);
        let result = calculate(&c);
        for row in result["costs"].as_array().unwrap() {
            for variant in row["ai_variants"].as_array().unwrap() {
                if variant["fraction"] == 0.0 {
                    assert_eq!(
                        variant["hybrid_first_year_recurring_usd"],
                        row["no_ai_first_year_recurring_usd"]
                    );
                    assert_eq!(
                        variant["hybrid_five_year_plus_build_usd"],
                        row["no_ai_five_year_plus_build_usd"]
                    );
                }
            }
        }
    }

    #[test]
    fn storage_retention_and_token_sensitivity() {
        let c: Value =
            serde_json::from_str(include_str!("../research/computation-cost-inputs.json")).unwrap();
        let a = calculate(&c);
        let mut changed = c.clone();
        changed["input_tokens"] = json!(4000);
        changed["billed_output_tokens"] = json!(1000);
        let b = calculate(&changed);
        let r = &a["costs"][2];
        assert_eq!(r["stored_end_year_decimal_tb"], 2920.0);
        let review_fte = r["ai_variants"][12]["additional_case_review_fte"]
            .as_f64()
            .unwrap();
        assert!((review_fte - 76.0416666667).abs() < 1e-8);
        // Five annual linear-growth storage areas total 12.5 full-year units;
        // one-year rolling retention totals 4.5. Their difference is eight.
        let expected_extra = 8.0 * 2_920_000.0 * 0.08 * 12.0 * 1.3 * 1.25;
        assert_eq!(r["five_year_no_deletion_storage_extra_usd"], expected_extra);
        let v = &r["ai_variants"][12];
        assert_eq!(
            b["costs"][2]["ai_variants"][12]["api_only_year_usd"]
                .as_f64()
                .unwrap(),
            2.0 * v["api_only_year_usd"].as_f64().unwrap()
        );
        assert_eq!(
            a["costs"][2]["no_ai_first_year_recurring_usd"],
            b["costs"][2]["no_ai_first_year_recurring_usd"]
        );
    }
}
