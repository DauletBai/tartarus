# Mathematical operation without AI: design and cost study

As of 6 November 2026. This is a reproducible engineering study, not a procurement quotation, national deployment plan or evidence that the proposed economy has been validated. All infrastructure capacities, staffing and workloads are editable assumptions. Only the explicitly sourced API tariffs are observed prices. Amounts are approximate external IT costs in USD, not a conversion of Qazna's normative Tenge measure.

## Decision and implemented boundary

Qazna should remain operational without generative AI. Exact postings, resource balances, eligibility rules, capacity checks and plan validation need deterministic software. Optimization does not require a language model. AI may assist with explanations, document extraction, forecasting features and incident investigation, provided its contribution is independently checked and worth its full cost.

The current Rust coordinator already calls `propose_provisioning` directly. The separate opt-in `scripts/ai_review.py` adds commentary. No model review is required to calculate and apply a valid draft. The PostgreSQL test `automatic_recalculation_keeps_operator_authority_and_deduplicates_drafts` now explicitly checks that the review table remains empty through recalculation and operator application. This verifies one complete path, not national-scale unattended operation.

The current `ordered-planner-v2` is bounded, sequential and caller-ordered; it is not a global LP/MIP optimizer. It is limited to 32 needs and 64 candidates per proposal. A small exact optimizer has been added as a **research example**, not integrated into economic authority. AI-independent operation and optimal planning are separate capabilities.

| Function | Non-AI implementation | Optional AI contribution |
| --- | --- | --- |
| Accounting circulation | Integer postings, double entry, atomic transactions, idempotency | Explain a saved, reconciled result |
| Supply and production | Resource balance, min-cost flow, linear/mixed-integer optimization | Propose a formulation for independent validation |
| Daily/long-term planning | Rolling horizons, scenario trees, robust constraints, classical forecasting | Improve forecasts if out-of-sample evaluation justifies it |
| Individual needs | Versioned human-approved norms, verified events, preferences, choice and appeal | Assist registration; no inferred entitlement from personality or employment |
| Shortage detection | Constraint residuals, inventory/queue thresholds, statistical change detection | Triage unfamiliar combinations and explain evidence |
| Security | Authentication, scoped authorization, signatures, rate limits, audit, isolation and deterministic incident rules | Advisory investigation of sanitized incident bundles |

Anomaly detection is not inherently dependent on AI either. Start with rules and statistical baselines such as EWMA/CUSUM. Calling a language model for every transaction is unnecessary. Automatic isolation should use preapproved, bounded rules with recovery procedures; a model must not independently confiscate balances, reject essential provision or change monetary norms.

## Economic formulation

Indices: region `r`, resource `k`, production process `j`, person/group `i`, time `t`. Quantities carry physical units. Let `s` be stocks, `x` process activity, `B` usable output coefficients, `A` consumed input coefficients, `y` confirmed provision, `f` shipments with arrival delay, and `w` spoilage. A time-expanded physical balance is:

```text
s[r,k,t+1] = s[r,k,t] + sum_j (B[k,j]-A[k,j])*x[r,j,t]
            + arrivals[r,k,t] - dispatches[r,k,t]
            - sum_i y[i,r,k,t] - w[r,k,t]
s >= 0; x >= 0; y >= 0
0 <= y[i,r,k,t] <= validated_need[i,r,k,t]
C*x <= available_capacity
L*x <= available_skilled_hours
E*x <= available_energy
shipments <= route_capacity
```

A production lead time moves output to its actual completion period. Likewise, dispatch is not same-period arrival. Include maintenance, transport, construction, education and health as explicit activities consuming inputs and capacity; they cannot be covered by a financial balance alone. Changing needs, births, equipment failures and preferences require revised forecasts and feedback. No optimizer has perfect knowledge of future choices.

Use a human-approved **lexicographic** objective: first protect specified essential-service constraints, then improve worst-group coverage, then reduce remaining unmet needs and waste, then unnecessary changes to an accepted schedule. Units and priorities must be explicit; do not add litres, meals and care-hours. For a positive frozen need `n`, a coverage constraint `y >= z*n`, `0 <= z <= 1`, can maximize the common minimum fraction `z`. Higher-priority clinical constraints need separate treatment. This defines a research comparison, not an enacted distribution policy.

Linear programming handles divisible activities. Mixed-integer programming handles indivisible machines, shifts and open/closed facilities. Scheduling may need constraint programming; quadratic penalties may model schedule stability. Multi-period uncertainty needs scenario/robust optimization rather than pretending a single equation predicts everything. If a problem times out, preserve a validated feasible incumbent and its reported optimality gap. An infeasible problem must report the bottleneck; it must not silently invent stock or deprive someone by changing the input need.

The simple input-output equation `(I-A)x=d` is useful only for a compatible square technology model: a nonnegative productive matrix, suitable spectral-radius conditions, and no omitted capacity/time limits. Solving it alone is not a complete state plan.

For accounting, let `D` be the account-edge incidence matrix (each transfer column sums to zero), `q` the exact integer transfer vector, and `b` balances:

```text
b_next = b + D*q + explicitly_authorized_net_issuance
sum(b_next) = sum(b) + sum(explicitly_authorized_net_issuance)
```

Ordinary circulation has zero issuance. Reservations must prevent conflicting spend before settlement. The presence of a balance does not prove physical availability, and a constant accounting unit does not make resources unlimited. There is no interest, tax, deposit return or speculative price mechanism introduced by these equations. Governance of norms and issuance remains an unresolved, separately authorized policy problem.

## Executable result without a model

The Rust example solves a bounded integer problem exactly. Two processes produce the same provision unit. Process X consumes two grain units and one energy unit; Y consumes one grain and two energy units. Their individual capacities are 100. Demand is 200. Maximize `x+y`, then minimize energy among tied solutions:

```text
2x+y <= G; x+2y <= E; x <= cap_X; y <= cap_Y; x+y <= 200
x,y are nonnegative integers
```

| Scenario | Grain / energy | X / Y activity | Provided | Unmet | X-first greedy provision |
| --- | ---: | ---: | ---: | ---: | ---: |
| Balanced | 240 / 240 | 80 / 80 | 160 | 40 | 140 |
| Energy shortage | 240 / 150 | 100 / 25 | 125 | 75 | 125 |
| Grain shortage | 120 / 240 | 10 / 100 | 110 | 90 | 60 |
| X capacity falls to 40 | 240 / 240 | 40 / 100 | 140 | 60 | 140 |

For the balanced case, adding the two resource inequalities proves `3(x+y)<=480`, hence no solution can exceed 160; `(80,80)` attains that bound. The test also compares the optimizer against exhaustive independent enumeration over small resource/capacity/demand combinations. Another test reproduces the published JSON from the current inputs, preventing stale results after a calculation change. This is a genuine minimal result: optimization can outperform ordering without AI. It neither proves a nationwide benefit of 14.3% nor establishes the complete economic proposal.

For a larger solver, [HiGHS](https://highs.dev/) provides MIT-licensed LP/MIP/QP software. Its implementation is C++, not Rust. A possible architecture is a restricted external solver process with Rust-owned input construction and independent exact feasibility validation. No solver is integrated yet. Keeping authoritative runtime logic in Rust does not require claiming an external numerical library was written in Rust; adopting one requires explicit dependency/license and execution-boundary review.

Use regional/time/commodity decomposition and sparse matrices. Individual settlements remain separate from aggregated production planning. A billion residents do not require one dense billion-variable solve every second. Cross-region balances must be reconciled; decomposition does not automatically retain the global optimum. Current all-monitor event fan-out and history-query hot spots must be fixed before extrapolating throughput.

## Reproducible budget assumptions

Run from the project directory:

```sh
cargo test --locked --example no_ai_study
cargo run --locked --quiet --example no_ai_study > research/computation-cost-results.json
```

Inputs: [editable JSON](../research/computation-cost-inputs.json). Results: [generated JSON](../research/computation-cost-results.json). Code: [Rust example](../examples/no_ai_study.rs). No credentials, paid model request or participant data are used. Floating point is only for approximate research costs; authoritative postings remain exact integers.

Base workload is 10 commands/person/day, peak 10 times average, 10 CPU-ms/command, 50% utilization, twice ledger compute capacity for redundancy, minimum eight vCPUs. Planning domains contain 100,000 people, with 96 runs/day, 60 CPU-seconds/run and 50% utilization. These are **unmeasured planning assumptions**, not claims about PostgreSQL throughput, solver time or service availability.

Storage is 2,000 bytes/command including assumed index/history overhead, four copies, $0.08/decimal-GB/month. Compute is $0.04/vCPU-hour. Network/observability adds 30% of compute/storage cost. Engineering/operations uses `6+ceil(population/1m)` FTE at $100,000 fully loaded/year. Independent assurance is `$50,000+$0.01/person/year`. A 25% contingency is included. Initial study/build allowance is ten person-years plus $200,000 assurance, or $1.5m with contingency. **This fixed initial allowance is research development, not the cost of deploying a country.**

Memory, IOPS, inter-region bandwidth, compliance, identity rollout, devices, physical delivery staff, supplier integration and organization-specific overhead need separate sizing/quotes; a percentage allowance cannot guarantee coverage. The model does not include food, hospitals, schools, housing or other real national provision budgets. Existing operator application/attestation labour also needs measured case throughput; engineering FTE is not a substitute for it. National readiness cannot be purchased for the numbers below.

```text
commands_day = population * commands_per_person_day
ledger_vCPU = ceil(avg_commands_second * CPU_seconds * peak / utilization * redundancy)
planner_vCPU = ceil(domains * runs_day * seconds_run / 86400 / utilization)
storage_end_year_GB = commands_day * 365 * bytes_command * copies / 1e9
```

The first operating year starts with empty storage and linear accumulation, so average storage is half its year-end value. The baseline five-year calculation assumes a one-year rolling stored dataset: build + first year + four steady years. Prices, activity and staffing are held fixed in 2026 USD across five years; no inflation, discounting or growth forecast is assumed. This is a cost scenario, **not permission to delete immutable financial history**. Retaining all five years at the same storage tier adds the separately reported amount. Real hot/cold archival tiers need new prices and restore testing.

| Population | Average / peak commands per second | Year-end storage, all copies | First-year recurring, no AI | Five years + study/build, rolling storage | Extra to retain all five years |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100,000 | 11.6 / 116 | 2.92 TB | $0.946m | $6.240m | $0.036m |
| 1 million | 116 / 1,157 | 29.2 TB | $1.001m | $6.594m | $0.364m |
| 100 million | 11,574 / 115,741 | 2.92 PB | $19.553m | $108.374m | $36.442m |
| 1 billion | 115,741 / 1,157,407 | 29.2 PB | $188.210m | $1,033.652m | $364.416m |

Decimal PB means 10^15 bytes. Five years of full retention for the 100m scenario gives **$144.816m**, including the $1.5m research/build allowance. Hardware counts are capacity arithmetic, not a measured cluster design. The billion-person row is a stress extrapolation; geographic/legal/failure-domain complexity is not linear.

For 100m residents, changing CPU cost/command to 2–50 ms, bytes/command to 500–8,000, copy factor to 2–6, and planning CPU time to 10–600 seconds gives **$15.388m–$42.169m** first-year recurring cost, with staffing and prices held fixed. These are selected scenarios, not confidence bounds. Longer solver runtimes, higher human workload or stronger resilience can exceed the range.

## Optional AI cost

Standard text API tariff snapshots, USD per million input/output tokens, checked 6 November 2026:

| Model | Input | Output | Official source |
| --- | ---: | ---: | --- |
| GPT-6 Luna | 0.10 | 0.50 | [OpenAI pricing](https://developers.openai.com/api/docs/pricing) |
| GPT-6.1 Terra | 1.00 | 5.00 | Same source |
| GPT-6.1 Sol | 2.00 | 10.00 | Same source |
| GPT-6 Astra | 10.00 | 50.00 | Same source |
| Claude Haiku 5.5, short-context tier | 0.10 | 0.50 | [Anthropic pricing](https://platform.claude.com/docs/en/about-claude/pricing) |
| Claude Sonnet 5.5 | 2.00 | 10.00 | Same source |

Prices are not a quality ranking. Haiku's listed tier applies up to 100k input context; larger contexts differ. Tokenizers differ, so identical token counts do not guarantee equivalent documents. Google/Azure commercial quotes and pricing were not established for this calculation. No bulk discounts, caches, subscription entitlements or free research credits are assumed. Regional premiums, tools, retrieval, long context and reasoning output can add costs. Recheck tariffs before a purchasing decision.

Each reviewed case assumes 2,000 input and **500 billed output** tokens including reasoning if charged, with a 1.1 attempt multiplier. Review a fraction `f` of commands after deterministic filtering and aggregation; grouping incidents can make actual case count much smaller. Adversarial repeated alerts can make it larger.

```text
annual_API = commands_day * 365 * f * attempts
             * (input_tokens*input_rate + billed_output_tokens*output_rate) / 1e6
additional_AI_case_review_FTE = commands_day * 365 * f
             * review_fraction * minutes_per_review / (60 * productive_hours_year)
```

The hybrid estimate additionally assumes two integration/operations FTE and $25,000/year evaluation, plus human inspection of 1% of AI cases at two minutes/case and 1,600 productive hours/reviewer/year. This is additional AI-output review, not all human authority/physical-delivery labour. It may be inadequate for complex cases; critical decisions cannot use a 1% sample as their authorization policy. All increments receive the same 25% contingency.

| 100m residents: commands sent for AI assistance | API/year, Luna | API/year, Sol | Extra case-review FTE | First-year recurring total, Luna | First-year recurring total, Sol |
| --- | ---: | ---: | ---: | ---: | ---: |
| None | $0 | $0 | 0 | $19.553m | $19.553m |
| 0.01% | $18,068 | $361,350 | 7.6 | $20.807m | $21.236m |
| 0.1% | $180,675 | $3,613,500 | 76.0 | $29.565m | $33.856m |
| 1% | $1,806,750 | $36,135,000 | 760.4 | $117.144m | $160.055m |
| Every command | $180,675,000 | $3,613,500,000 | 76,041.7 | $9,750.886m | $14,041.917m |

The last row exposes the cost of the assumed workflow; it is not a recommendation to review every transfer. At 0.1%, five-year hybrid totals with rolling storage are $158.436m / $179.891m; add $36.442m for full five-year storage retention. If human inspection is unnecessary for a bounded, noncritical explanatory task, set its review fraction explicitly to zero: the 0.1% first-year totals become $20.060m / $24.351m. Do not silently omit human cost from an oversight-dependent design.

For a self-hosted **different open-weight model**, the example assumes $2/GPU-hour, one effective 2,000+500-token request/second/GPU, 50% utilization, peak factor ten and at least two GPUs. At 0.1% that implies 255 GPUs, $4.468m/year GPU rental and $34.924m total first-year recurring cost with the same review assumptions. This is a sensitivity reference, not a claim that any listed proprietary model can run locally or that a given GPU achieves this rate/quality. Benchmark model size, memory, batching, peak concurrency and latency before considering a break-even comparison. Queued nonurgent inference may require far less peak capacity.

## Implementation sequence and acceptance

1. Preserve the deterministic service and an explicit no-AI deployment profile. No provider SDK/credential belongs in posting authorization. Existing offline tests require no paid provider; add provider-unavailability and credential-absence checks when production adapters exist.
2. Measure command CPU, DB I/O/index growth, operator/attestation time and recovery under realistic concurrent workloads. Replace assumed rates in the JSON, including document storage and data locality. Resolve monitor fan-out before extrapolation.
3. Build a Rust-owned multi-period scenario contract and restricted solver interface. Compare current greedy, exact small oracle and candidate LP/MIP on identical inputs. Check independent physical/accounting conservation, stale snapshots, integer rounding, timeouts, scarcity, priorities and delay. Performance gains must not hide new unmet needs.
4. Add dependency-aware jobs, debounce/coalescing, bounded queues and regional planning. Never combine mutually dependent resources without shared reservation/reconciliation.
5. Run deterministic incident rules first. Introduce optional AI only for a measured use case, with minimized data, no signing keys, bounded retries/context, daily monetary cap and automatic fallback. Those provider-budget controls are proposed, not implemented today. Compare false negatives, false positives, response time, harmful actions, human effort and total cost against the non-AI baseline.
6. Before live provision, close identity, appeal, privacy, lifecycle and independent assurance gaps from the [architecture review](architecture-review.md). Economic welfare requires a voluntary evaluated pilot, as described in [AI economic research](ai-economic-research.md), not merely successful code tests.

The recommended default is deterministic planning/settlement plus statistical monitoring. Optional AI must demonstrate an incremental benefit; losing the AI service or exceeding its budget must not stop ordinary provision or circulation.
