# Completion roadmap and feasibility boundaries

Latest review: 13 November 2026, coordinated unstarted-batch repair and protected service reserves.
The current work order below supersedes earlier milestone-specific next steps.

## Completion scope and remaining work — 12 November 2026

The selected extended integration now joins 31 resources, 44 processes and 60
monthly periods with original food ages, finite capital, packaging recovery and
exact circulation. Reviewed runtime versions, restart/restore and invalid-capacity
cases cover this bounded candidate. This closes package 2's selected integration
gate; it does not establish an efficient or indefinitely self-renewing economy.
Two explicit findings move into package 4: surplus packaging disposal and terminal
concentration of the fixed schedule's working measure in the common treasury.
No reset, hidden issuance or automatic redistribution resolves them. Four packages
remain. Approved research versions still do not change actual inventory or money.

Package 3 now has a bounded Rust registry and monthly age-cohort projection:
source/version/unit/interval checks, three illustrative demographic variants,
unknown need propagation and immutable runtime demand binding. The full upstream
candidate rejects the increased projected demand rather than hiding unmet need.
A separate finite-stock water-component fixture exercises accepted review and
restart/restore. Regional person-day partitions and joint pregnancy/care profiles now bind to
physical needs, with local-shortage rejection. A separate 60-month service graph now connects care and education to shared
workforce, rooms, equipment, training and construction with exact operating inputs.
Lifecycle transitions, sex-specific fertility, full upstream integration, regional
routes, source calibration and engineering inputs remain open;
package 3 is not complete.
Controlled future availability revisions and bounded unstarted-batch repair now
preserve accepted history, ongoing training and human needs. Package 4 has this
first reviewed repair path, including failed recovery. Incident declarations now
persist independently of repair; matching approved service and settled circulation
versions can be linked without claiming observed recovery. Compatible declarations
can now be combined with preserved lineage; correction and withdrawal retain all
history. The extended-absence case exhausts nine single-batch dates and exposes a
future care gap. This closes the bounded incident-combination/correction gate.
Coordinated repair now searches up to four unstarted batches with fixed staff and
equipment reserve floors. A two-region case succeeds only when both courses move;
physical/monetary prefixes remain fixed and reviewed versions survive restoration.
This closes the bounded multi-batch/protected-headroom gate. Next integration gate:
explicit emergency reserve-release proposals tied to incident evidence, with
unchanged human needs and independent approval; quantify residual shortages and
recovery/replenishment obligations without adding stock or issuance. Repeatable
terminal circulation and upstream service replenishment remain open; package 3
calibration and package 4 are not complete. The effort range below remains a planning estimate, not a new deadline.

The target for this estimate is **research demonstrator v1**: a reproducible,
bounded five-year scenario with linked provisioning, physical capabilities and
accounting; selected food chains and demographic profiles; explicit inputs and
uncertainties; a small operator-facing service; and documented failure cases.
The 100-million population is an aggregate demand scenario, not evidence of
100 million concurrent users. This target is not a complete representation of all
national sectors or a production monetary system.

| Remaining package | Acceptance gate | Estimated focused engineering days |
| --- | --- | ---: |
| 1. Persistent physical scenario foundation — implemented | Dated and capital/seasonal inputs, reviewed versions, pinned selected stocks and restart/restore; monetary association continues in package 2 | — |
| 2. Join extended physical and monetary plans — implemented for selected candidate | 60-month runtime fixture, origin-preserving food, upstream production, finite capital/recovery and exact payments; independent balances and rejection of infeasible capacity changes. Efficiency, deficit quantification and repeatable policy continue in package 4 | — |
| 3. Input and population registry — bounded foundation implemented | Exact monthly cohorts, registry metadata, unknown needs and reviewed binding implemented. Regional/life-stage needs, shared service capacity and local-shortage gates implemented. Remaining: source calibration, lifecycle transitions, regional routes and engineering coefficients | 6–10 |
| 4. Close the rolling feedback experiment — bounded service repair implemented | Exact revisions, single/joint-batch repair, protected service reserves, durable incident lineage and matching reviewed circulation implemented; emergency release/replenishment and repeatability remain. Decisions use observed state, retain commitments, replan after declared outages/demand changes, report shortages without inventing goods, and pass sensitivity/adversarial cases; optional solver outputs independently verified | 4–7 |
| 5. Expose the demonstrator's workflows | Small operator/reviewer interface, synthetic participant scope, intelligible reasons, correction/dispute examples, approval and audit history, accessible navigation; no claim of production identity/enrollment | 7–12 |
| 6. Freeze and reproduce a release | Clean deployment, recorded workload envelope, sustained/restart/restore and fault exercises, dependency/privilege review, versioned evidence and known limitations, release guide and runnable demonstration | 4–6 |

**Total planning range: 21–35 focused engineering days**, approximately **5–7
working weeks** for one continuously available full-time implementer at five days
per week. The calculation-focused packages 3–4 total **10–17 engineering days**;
packages 5 and 6 add **11–18 days** for the service and release. This is a rough
human-equivalent effort estimate, not a promise of autonomous agent work or a
calendar completion date. Decisions, missing source data, failed acceptance tests,
review findings and interruptions can extend elapsed time. Re-estimate after each
merged package instead of adding indefinitely to the scope.

Assumptions: existing Rust/PostgreSQL architecture retained; no model training,
new distributed platform, nationwide integration, production participant identity
or full sector census in v1. Independent external reviews, empirical food/industrial
measurements and institutional approval are outside these effort totals.

For an **all-sector national model**, there is no defensible completion date yet:
health, education, housing, regional transport and other production chains still
need agreed scope, usable data, validated coefficients and independent review.
A **voluntary shadow pilot** additionally needs an operator, participants,
measurement procedures, privacy/security safeguards and operational support.
Its calendar cannot be derived from the code alone. Deployment to a state or to
billions of users is not the next six-week deliverable.

Evidence used in this estimate: `src/api.rs` and current 16 KiB body limit;
`src/draft_service.rs` and existing atomic operator application;
`src/rolling.rs` (offline fixed-prefix models); `src/dated.rs` and
`src/scenario_service.rs` (persistent dated-state command replay);
`migrations/0013_dated_scenarios.sql`, `0014_model_scenarios.sql` and
`0015_coupled_scenarios.sql`; `src/coupled.rs` and `src/coupled_service.rs`;
`docs/development.md` (actual tests); and the absence of a participant product UI.
Existing backup, privilege and draft tests cover the new dated-state boundary;
the selected extended coupled integration is tested; empirical-evidence gates remain open.

## Earlier implementation order and bounded results

Already implemented: exact accounting and physical/payment checks; a 1,825-period
coupled synthetic model with 22 dimensions and 21 processes; fixed-delivery plan
trimming; causal reserve-route feedback with delayed commitments; and independent
checks of idempotency, feasibility and accounting conservation; and a common
opening balance certificate across six disruption schedules; and constructed fuel
reserves with losses, restart protection and refill across seven scenarios; and
an isolated dated-food/transport/maintenance submodel with 32 five-year runs; and
seven complete lifecycle/circulation scenarios with a fixed-buffer counterexample;
and a target preserved-food design with constructed storage and restart allowance;
and two-phase daily maintenance with independent paused-work checks;
and an offline nutrient/order annex with pinned food-composition data, per-profile
constraints, bounded menu search and mode-specific service requirements.
Seasonal crops, automation, upstream production, finite-life capital, rolling
commitments and constituent conservation have since been implemented as bounded
offline annexes (merged PRs #5–#8). The current dated-state stage preserves original
food origins, expiry, pending harvest and movements across checkpoint restoration.
Current validation is recorded in [development](development.md); published CI
results are distinct from local checks. The runtime API now exposes reviewed
dated, capital/seasonal and derived circulation
versions; the selected extended integration now has a reproducible runtime fixture.
These results cover the declared synthetic model, not national deployment.

The author's current priority is independent calculation and testing before
external approaches. No partner, investment or paid model call is a prerequisite
for the following sequence. The table distinguishes completed bounded experiments from remaining work.

| Order | Work package | Why it is still needed | Completion evidence |
| --- | --- | --- | --- |
| 1 — bounded experiment implemented | Common opening measure across six disruption schedules | Same initial money/assets now tested; physical reserves are fixed rather than optimized; uncertainty beyond the declared family remains open | Exact worst-prefix witnesses, independent replay of every scenario, one-unit-less necessity checks and equal-observation physical prefixes; see the common-funding results |
| 2 — bounded experiment implemented | Joint planning of reserves, routes, replacement and training | Eight feedback policies × six paths now compared with equal initial assets and exact terminal/cash checks; general design optimization remains open | Zero transport-only food deficit with additional produced equipment; minimax shortage reaches a verified energy-outage lower bound; see joint-planning evidence |
| 3 — integrated lifecycle experiment implemented | Energy continuity, expiry, transport loss, maintenance and changing demand | Constructed reserves and integrated dated-food/transit/service traces with monetary replay are tested; preserved-food source profiles, nutrient/last-mile calibration, specialist capacity calibration and runtime integration remain open | Explicit dated lots/loss events; no expired or destroyed output delivered; partial failures, recalls, disrupted routes and revised needs with no rewriting of past provision |
| 4 — nutrition/order submodel implemented | Cohorts, additional services and evidence-backed coefficients | Two synthetic adult profiles and explicit preparation/delivery choice now work offline; seasonal cereal/feed and a bounded automation/distribution graph now work; retained nutrients, national demography, regional calibration, health and education remain incomplete | Versioned input registry with units, region, year, source and uncertainty; age/cohort transitions, voluntary qualification pathways and service capacity; distinguish synthetic and calibrated runs |
| 5 | Reviewed integration into the live Rust service | Reviewed runtime versions and the selected extended fixture now exist; broader calibration remains | Versioned scenario snapshots and drafts; stale-state rejection, atomic application, restart/retry and independent replay; no solver or AI authority to post or issue measure |
| 6 | Participant-facing demonstrator and operational completion | Machine credentials are not resident identity; disputes, corrections and UI are incomplete | Scoped resident/organization access, evidence custody and dispute resolution, compensating events, accessible role-specific UI, overload and disaster-recovery tests; independent review before any consequential pilot |

### Implemented bounded calculation: common opening measure

For a declared finite scenario set Ω and fixed scenario schedules, let
`C[a,ω,t]` be cumulative net inflow at account a through event t. A common opening
vector sufficient for all those schedules is:

```text
E[a] = max(0, max over ω in Ω and event prefixes t of -C[a,ω,t])
```

This is the componentwise minimum for those fixed schedules with no extra
rebalancing or later issuance. It is not a global monetary optimum or a guarantee
for events outside Ω. Verify it against an independent replay and small exhaustive
cases. Changing routing or payment order changes the schedules and requires new
bounds. Causal decisions must remain equal while observations are equal; choosing
an action from a scenario's hidden future is forbidden.

The maintained report now compares service coverage, opening measure, fixed reserve
quantities and disposal using the **same** initial funding and asset basis. It
includes simultaneous power/transport/capacity failures and delayed recovery.
See [common-funding evidence](physical-circulation.md#common-opening-measure-across-bounded-disruption-scenarios). If full provision is physically impossible,
report the deficit and the governing constraints. A subsequent optimizer may then
trade among declared service/reserve objectives without changing human rights or
inventing inputs. Calibration and the runtime safety gates remain necessary.

## Earlier assessment and milestone record

Initial assessment: 4 November 2026. Baseline: `3f4a028` (v0.10.0).
This is a source/documentation review, not an independent security audit or a new economic validation. The baseline passed 86 PostgreSQL, 3 Rust and 11 Python tests, HTTP restart checks, five strict-evidence cases and two accelerated actor exercises. Those results cover specified synthetic cases only.

## What completion means

There is no credible single percentage for a completed national monetary system. Use separate deliverables with explicit acceptance gates:

1. A reproducible research demonstrator: implemented mechanisms, assumptions, failure cases and limitations can be inspected and repeated.
2. A bounded voluntary pilot: a named operator, participants, resource category, governance rules, safeguards, support and exit procedure are in place. Prefer shadow accounting before any authority over real provision.
3. An operated service: measured reliability, recovery, security, accessibility, maintenance and accountable institutional ownership.
4. A wider economic proposal: independently evaluated rules, transition costs, institutions and evidence from pilots. Software readiness cannot establish public consent or predict adoption.

The present implementation is a research core. It is not yet a complete self-service product or an approved monetary regime. Ongoing maintenance and incident response remain necessary after any release.

## Implemented foundation

Rust/Tokio/Axum and PostgreSQL/SQLx implement exact accounting, atomic reservations, needs, recipes and capacity, completion and losses, provenance, reviewed drafts, durable coordination and aggregate observation. The operator retains execution authority; model commentary cannot apply actions. Strict evidence mode separates inspector, custodian and recipient credentials and blocks unattested or disputed resources. The book and code now cross-reference each other; the Russian HTML reading edition is public.

## Initial blocking work and acceptance evidence

This baseline checklist is retained for traceability. The v0.11 and research
updates below record partial closures; the current work order above governs
implementation.

| Priority | Work | Current gap | Acceptance evidence |
| --- | --- | --- | --- |
| P0 | Pilot charter and participant rights | Scope, governance and accountability are not an operational agreement | Written scope, voluntary entry/exit, support, appeal, stop criteria and named decision owners; no deprivation for refusing a model recommendation |
| P0 | Basic provision and accounting policy | Need registration is independent of money, but `reserve_in` in `src/provisioning_service.rs` requires sufficient available measure | Approved, versioned rules for allocations, treasury replenishment/circulation and shortages; test a person with zero balance and available food, a depleted treasury, and insufficient physical food without silently issuing money |
| P0 | Fair allocation and norm governance | `src/planning_service.rs` follows caller-listed order; no adopted social priority rule | Explicit urgency, minimum coverage, waiting-time and exception policies; explain every denial; test order manipulation and chronic exclusion under scarcity |
| P0 | Identity, ownership and key lifecycle | Broad operator token, locally configured attestors, no real-world independence proof | Resident/organization scope checks, appropriate authentication, enrollment review, revocation and rotation; cross-account denial, compromised key and colluding enrollment scenarios |
| P0 | Evidence custody and disputes | Digests do not authenticate documents; disputes block but have no resolution path | Retained source evidence, measurement/provenance checks, independent review, deadlines and auditable resolution; test false output, coercion, duplicate claims, lost evidence and malicious dispute blocking |
| P0 | Corrections and recovery of business mistakes | Append-only records lack a complete compensating workflow | Linked correction events for wrong needs, receipts, assignments and amounts; prevent duplicate compensation, preserve originals, and distinguish accounting reversal from physically returning goods |
| P0 | Runtime privileges and privacy | DB owner can bypass constraints; participant access/retention rules incomplete | Separate migrator/runtime/audit privileges, data minimization, protected transport and secrets, restricted evidence access; demonstrate runtime cannot alter schema/history and logs do not expose secrets or private needs |
| P0 | Backup and outage continuity | Process restart is tested; disaster recovery is not | Restore into an isolated environment, reconcile histories and physical holds, exercise database loss and network outage; choose and measure RPO/RTO, plus an offline continuity/reconciliation procedure |
| P0 | Small usable interface | API and exercise report are not resident/operator workflows | Resident needs/receipt/dispute, operator draft review, inspector evidence and auditor views; accessible language-aware flows, understandable reasons and assisted access without a smartphone |
| P1 | Physical resource lifecycle | No complete transport, expiry, quality, recall or return model | Lot location/custody and timing, partial deliveries, expiry/quarantine, quality and units; exercises for spoiled food, delayed transport, rejected receipt and recalls |
| P1 | Production realism | Immutable recipes and daily capacity do not model all real constraints | Maintenance, downtime, staffing, energy/waste, replacement inputs and multi-stage dependencies; conservation and feasibility checks on observed pilot processes |
| P1 | Planning quality | Bounded greedy proposals, no recursive/global optimizer | Versioned deterministic inputs and constraints; compare baseline with optional HiGHS solver; independently validate feasibility, explain infeasibility, bound runtime and preserve safe manual operation |
| P1 | Operational limits and observability | No demonstrated external workload envelope or complete incident process | Per-scope quotas, admission limits, queue budgets/coalescing, external alerts, runbooks and support ownership; overload, slow DB, storage exhaustion and sustained concurrency tests against declared targets |
| P1 | AI evaluation and service model | A few synthetic live reviews are not a measured reliability claim | Versioned labelled evaluations for unsupported claims, missed shortages, injection and latency; model/config provenance, cost limits, privacy review and deterministic fallback; retain operator approval |
| P1 | Release and independent review | CI and license inventory exist; release/maintenance process incomplete | Reproducible deployment instructions, migration rehearsal, dependency/security review, release provenance, incident channel, independent security/economic review and closure of critical findings |
| P2 | Multi-site scale and federation | No measured nationwide workload or cross-region consistency | Adopt only after pilot evidence; test partition behavior, cross-domain reservations and duplicate prevention, regional recovery and realistic operating costs |

P0 means required before assigning the system consequential authority over real participants or assets. P1 work is scoped to the pilot: perishable goods need expiry controls before that pilot, not afterwards. P2 is not a prerequisite for a small shadow experiment.

## Decisions that must not be silently encoded

- What the accounting unit represents and why a given resource requires a given measure.
- Who approves norms, changes them, audits that authority and hears appeals.
- Whether and how basic provision is funded when the resident or treasury lacks measure. Current demo issuance is a fixture, not policy.
- How measure circulates from providers through the treasury, how capacity growth and depreciation are verified, and whether any issuance/redemption is permitted.
- How scarce physical resources are prioritized without inventing resources through accounting.
- Who can stop a participant, unit or system; how essential provision continues during a dispute or outage.
- What is necessary to know about a person, what must remain private, and how corrections coexist with retained transaction history.

These require accountable human decisions and appropriate independent review. An optimizer cannot choose a definition of justice from physics, and a language model must not infer authority from its confidence.

## Suggested implementation sequence

1. Agree on a narrow shadow-pilot charter and resolve the basic-provision/treasury gap in executable examples. Define acceptance and stop rules before adding functionality.
2. Implement identity and scoped access, independent enrollment, revocation, evidence custody and dispute/correction events as one coherent trust boundary.
3. Separate database privileges; rehearse backup restoration and offline reconciliation; expose the flows through a small accessible interface.
4. Model the selected resource's actual lifecycle and run mixed cooperative/adversarial exercises. Include honest mistakes, inaccessible recipients, staff absence, collusion, operator abuse and shortages, not only malicious outsiders.
5. Run the voluntary shadow pilot with independently checked measurements. Compare delivered coverage, waiting time, waste, administrative effort, exclusion/errors, appeals, recovery and cost against an agreed baseline. Specify thresholds, observation period and review method before evaluating results.
6. Decide whether to continue, revise or stop. Add optimization, broader categories or multiple sites only where measured constraints justify them.

Keep the existing modular Rust application and PostgreSQL. Identity and interfaces do not require replacing the core. Do not add Kafka, Temporal, extra databases or autonomous AI agents solely to match an organism metaphor; introduce infrastructure only against a measured requirement. The book's infrastructure table describes candidates, not the current deployment.

The eOtinish review date of 16 November 2026 is an administrative milestone, not evidence of acceptance or a production-readiness deadline. A credible demonstrator, limitations register and pilot protocol are better deliverables than an unsupported claim of completion. No calendar estimate is reliable until pilot scope, staffing and decision owners are agreed.

## Human behavior and the book

The author's latest reflection questions near-term social adoption and frames the work as a contribution to future generations. This is a motivation and governance concern, not an empirical forecast that adoption is impossible for a century. Technical feasibility, institutional acceptance and personal meaning should be discussed separately.

A viable pilot must not require every person to become morally perfect. Design for mixed motives, errors, disagreement and abuse, including misconduct by administrators and inspectors. Safeguards should constrain actions rather than score a person's moral worth. Basic provision must not depend on agreeing with the author, accepting a spiritual account, or being designated enlightened.

The organism analogy is useful for feedback, redundancy and repair, but social participants retain autonomy, consent and equal standing. Neither a single operator nor an AI brain should become an unaccountable source of truth. Refusal to join a pilot is not evidence of immorality.

A meaningful outcome can be a reproducible example that helps a willing community and records where it fails. This does not promise universal adoption and does not require declaring other people incapable of change. The book can preserve the author's personal convictions while clearly separating them from historical evidence and testable economic claims.

## Review anchors

- [Architecture](architecture.md), [organism principles](organism.md), [organism audit](organism-audit.md).
- [Norms and planning](norms-and-planning.md), [evidence](evidence.md), [AI safety](ai-safety.md), [security reporting](../SECURITY.md).
- `src/provisioning_service.rs`: resource availability and accounting gate.
- `src/api.rs` / `src/main.rs`: machine credentials, local service boundary and worker operation.
- `src/coordination_service.rs`: lease/retry behavior; `scripts/exercise.py`: synthetic recovery checks.

This roadmap does not change the book, economic policy, execution permissions or runtime behavior.

## Implementation update: v0.11

[Recovery and access](recovery-and-access.md) implements principal revocation, scoped evidence reads, runtime privilege separation and a logical restore rehearsal. [The shadow charter](pilot-charter.md) records the author-delegated scope and modelling choice. These are partial closures of the corresponding rows, not completion of OIDC, disputes, corrections, production backup operations or a real pilot. The remaining roadmap stays open.

## Research update: 6 November 2026

The [architecture/test review](architecture-review.md) updates implementation
evidence and test gaps. The [AI economic research brief](ai-economic-research.md)
maps four organizations' research to Qazna work packages E1–E6: reproducible
scenarios, independent multi-period circulation, policy comparisons, replaceable
AI advisers, minimized outcome measurement and participant-safe operation.
These are research/engineering requirements, not implemented features or grants.
Prioritize E1/E2 and the participant safeguards before claiming economic effects;
use E3/E4 to separate policy improvements from model assistance. All adopted
legacy runtime mechanisms must be reimplemented in Rust. Corporate research
funding is an optional route, not a dependency for the deterministic core.

## Independent computational work — 6 November 2026

Proceed without external applications, investment or a model provider. Completion
of this stage means an executable, reproducible specification with checked proofs,
negative cases and explicit scope; it does not mean that mathematical testing can
establish real-world social adoption or erase missing inputs.

Implemented in this block: exact primal/dual and infeasibility certificates,
time-expanded resource matrices, delayed production, fixed-cycle liquidity
bounds, two integer distribution references and continuous strict-evidence HTTP
trials. See [mathematical verification](computation-and-ai-costs.md#exact-mathematical-verification-and-five-year-resource-study).
The experimental algorithms are not connected to ledger authority.

Remaining computational acceptance gates:

1. Specify a complete, repeatable circulation schedule with economically justified
   edges and funding continuity. Verify every account prefix, growth/change in
   capacity and disconnected-region scenario; do not add a disguised fee or tax.
2. Extend the model beyond fixed-population synthetic boundary supplies: cohort
   transitions, independently sourced assumptions, staffing/education pipelines,
   upstream inputs, transport times, infrastructure replacement and expiry.
3. Add a multi-period optimizer with independently checked feasibility/certificates,
   or a feasible bound and honest gap where exact optimality cannot be certified.
   Full basic provision, choice and worst-group outcomes need separate objectives.
4. Compare scarcity policies including temporal exclusion and indivisible resources;
   do not adopt either rounding algorithm merely because it has one good property.
5. Close existing software P0 gaps (identity, corrections/disputes and interfaces)
   before consequential use. Test them against independent oracles and adversarial
   combinations, then measure resource limits under increasing actual load.
6. Produce a repeatable completion dossier that distinguishes theorem, code test,
   synthetic model and empirical input. External model review can then challenge
   the results without being necessary for their reproduction.

The current 1,825-period example has 100-million-person-sized aggregate numbers,
not 100 million simulated actors. Its baseline certificate concerns total meals
under declared constraints; it is not a proof of the whole proposed economy.

## Independent computational work — 7 November 2026

Gate 1 now has an explicit **candidate** for a complete toy circulation: common
productive units account for intermediate obligations and the population's
unconditional provision entitlement. One treasury disburses daily needs; residents
spend the day's provision and end each modelled period at zero. Public timing
reserves, not personal saving balances, cover delayed receipts. This assumption is
not a tax added to private income and is not enacted as an autonomous API policy.

Implemented exact finite-schedule prefix bounds, rejection without changing funds,
1,825-period same-day/delayed/growing cases, an intraday rescheduling comparison,
disconnected complete regions and
upstream-link failure traces. Tests include wrong individual entitlements despite
aggregate balance. See [the model and its proofs](computation-and-ai-costs.md#closed-circulation-candidate--7-november-2026).
The real API exercise uses existing operator-authorized transfer and provisioning
endpoints; it checks restart and retry behavior with synthetic evidence.

Gate 1 remains partial: the toy equipment boundary omits real replacement inputs;
common entitlements, additional contribution rewards, changing choice and external
trade need explicit complete constraints. A missed transfer trace is not a recovery
planner. Continue by coupling the circulation identity to the physical time-expanded
matrix and cohort/staffing/construction pipelines (gate 2), then optimizing and
checking multi-objective plans (gate 3). Do not confuse an accounting identity with
calibrated welfare, legitimacy or a complete five-year national plan.


## Coupled physical and monetary phase — 7 November 2026

Gate 2 now has a bounded, reproducible coupled example: finite deposits, energy,
metal/equipment, two transport legs, food, construction and training. Shared stock
capacity rows prevent equipment/qualified-hour double booking and premature use.
Explicit withdrawals remove capacity before activities. Complete, unique payment
links bind accounting to inputs, delivered goods and completed public capital.

Five 1,825-period scenarios pass exact physical and financial checks. Baseline
provision is complete for its two declared final services; outage and delayed
replacement/training cases retain unmet needs. A transport-window certificate
establishes a strictly positive minimum shortage. See [full scope and results](physical-circulation.md).

This partially closes the physical-coupling gate; it does not supply calibrated
national sector data, demographic cohorts, full upstream closure, spoilage,
individual choice or a global optimizer. Next work should add data/units provenance
and rolling multi-objective optimization of waste, service gaps and working funds.
No new runtime monetary authority or ownership policy is enabled.

## Exact plan improvement — 7 November 2026

Gate 3 now has an integer coordinate-improvement procedure, independent whole-matrix
verification and an exhaustive small-case oracle. Five five-year paired scenarios
preserve every seed delivery and terminal stock floor, with fewer power/fuel starts
and newly calculated monetary traces. A separate counterexample proves that local
irreducibility must not be advertised as global optimality.

This partially advances gate 3. It does not close the calibration or causal-planning
gates: trimmed traces use full-horizon hindsight; seed shortages are unchanged.
Next implement coupled/block moves or an independently verified solver candidate,
then causal rolling windows with committed work, expiry and safety-reserve policies.
Real sector coefficients need the input evidence listed in the
[calibration gate](physical-circulation.md#units-and-calibration-gate).


### Causal recovery milestone — 7 November 2026

Implemented an offline committed-output pipeline and a daily reserve-route policy.
The synthetic 30-period primary-transport interruption has zero missed meals with
declared slower reserve capacity; the 90-period stress retains a measured shortage.
Same-observation trace prefixes are equal despite different future recovery dates.
All traces retain exact physical/cash verification. This completes a bounded causal
recovery experiment, not a national planner. Next: joint reserve/route scheduling,
expiry and in-transit losses, uncertain demand, explicit priorities and a single
opening cash envelope valid across the declared disruption set.


## Joint planning milestone — 7 November 2026

The [joint-planning calculation](physical-circulation.md#joint-reserve-replacement-and-training-policies)
adds 48 exact five-year physical/cash traces, 40 same-observation prefix checks,
common terminal reserve floors and a sparse dual energy-outage certificate with
negative premise checks. Equipment plus food buffers reduces worst missed meals
from 5,089,395,840 to 2,760,000,000. Equipment policies eliminate the 90-period
transport-only food shortage. Four policies reach the fixed model's minimax
coverage lower bound; physical energy shortages remain. Earlier training alone has
no measured benefit in this bounded family. This advances gates 2/3 without claiming
complete national optimization or runtime integration.

Next: model the resource costs, delays and losses of energy continuity, then dated
inventory, transport failures and maintenance. Preserve service-level deficits,
causality, independent physical checks and recalculated monetary schedules.


## Constructed reserve milestone — 7 November 2026

The [reserve/recovery experiment](physical-circulation.md#constructed-reserve-and-recovery-experiment)
adds 35 exact five-year traces, 25 causal prefix checks and a retained failed refill
policy. A storage-aware 30-period reserve covers the prepared 30-period/repeated
failure family; a 60-period reserve also covers a 60-period supply interruption.
Early and overlong interruptions still cause explicit shortages. Six new mathematical
tests cover constructed storage timing, exhaustive allocation, loss conservation,
malformed constraints and minimal reserve sizing. Next implement dated food lots,
expiry, in-transit loss and maintenance/correlated failure; then broaden/calibrate
needs and integrate reviewed drafts. No runtime economic authority was added.

## Food and standby lifecycle milestone — 7 November 2026

The [lifecycle submodel](physical-circulation.md#dated-food-transport-loss-and-standby-maintenance)
checks 32 five-year paths, expiry in storage/transit, FEFO receipts, shared
maintenance labour, finite spares/fuel and correlated failures. Seven new tests
include independent individual-unit and periodic-service oracles. No-spare,
late-arrival and repeated-outage counterexamples remain published. The next gate
is to integrate these transitions into the full coupled matrix and monetary
replay, then compare adaptive age-aware production with the fixed-supply case.
Live lot expiry, quality/recall and reviewed runtime application remain pending.

## Integrated lifecycle gate — 7 November 2026

The full graph now includes dated food, transport damage and constructed-generator
maintenance. Metal/energy/service-capacity use and exact payments belong to the
same calculation; the static food-target failure is retained. See the
[integrated comparison](physical-circulation.md#integrating-lifecycle-constraints-with-the-complete-graph).
The common funding envelope is conditional on these scenarios. The next gate is
independently validated age-aware optimisation, per-day maintenance labour and
calibration; live authority remains unchanged.


### Preserved-food reserve gate — 7 November 2026

The target has no fresh perishable stock. Schema 9 contains nine raw-reserve
comparators, eleven preserved-food scenarios and the counterexample that sizes
reserve without restart. Source-qualified storage evidence and mandatory product
profile fields are included. The tested four-day restart allowance closes food
shortage in the prepared 30-day correlated outage, without hiding housing shortage.
Longer/early failures and missing storage remain counterexamples. Next add typed
nutrient and rehydration/access constraints with sourced coefficients, storage
operating/quality costs, then reviewed persistence in the runtime. Do not promote
an abstract staple or 730-day test parameter to a complete national food standard.


### Daily maintenance-work gate — 7 November 2026

Schema 10 represents two separate workdays in the full graph; unfinished service
pauses under unavailable second-phase technical capacity. Five five-year target
paths and three old-model counterfactuals retain physical shortages while all
payments for actual goods settle. The new 729-pattern individual-unit oracle and
queue replay prevent elapsed time from standing in for performed work. See
[daily maintenance](physical-circulation.md#daily-maintenance-work-and-unavailable-technical-teams).
Next: typed food/nutrient, rehydration and access constraints with a versioned
coefficient registry, then cohorts and health/education services. Real trade
specializations and general multi-day construction/training work remain open.


Acceptance criteria set before the nutrient/order block (partial closure below):

1. Version every food/profile coefficient with its physical unit, source, date,
   uncertainty and synthetic/calibrated status. Missing nutrient data must not be
   silently replaced by zero or treated as evidence of a complete diet.
2. Model potable water, preparation energy, handling and last-mile capacity in
   the same dated resource balance as food; a warehouse receipt alone is not
   confirmed personal provision.
3. Separate cohort needs and voluntary food choices. Test equal food masses with
   different nutrient content, inaccessible stores, water/energy interruption,
   and an adequate total that conceals an underserved group.
4. Use independent feasibility checks and keep residual deficits visible. Do not
   introduce clinical advice, automatic entitlement restrictions or new monetary
   issuance through the offline calculation.


### Nutrient/order gate — 7 November 2026

Schema 11 adds exact per-profile checks, complete-data LP compilation, a pinned
USDA composition extract, explicit preparation/delivery orders and agricultural
requirements. Twenty-four candidate menus are checked independently; four-service
mode scenarios expose kitchen/delivery gaps without duplicating food. Unknown
retention/composition prevents a full-diet certificate. Existing monetary/lifecycle
traces retain their original units and results.

Next priorities are validated retained-nutrient recipes and preserved-stock
profiles; children, pregnancy, lactation and other demographic needs; seasonal
feed/crop/land and water capacity; then mapping these requirements into the dated
runtime resource graph. Current order snapshots are offline and synthetic, not
an operational resident-order system or an adopted national food standard.

### Seasonal cereal/feed gate — 11 November 2026

Schema 12 adds a 60-month crop/seed/land/irrigation subchain derived from the
100-million order snapshot, with mandatory full needs and next-cycle reserve
floors. A separate event replay checks the exact matrix. Six scenarios distinguish
current provision, depleted reserve, feasible replenishment and capacity failures.
A pinned USDA rice-preparation extract and sequential retention helper keep
unmeasured storage unknown. The specific cereal/feed and preparation arithmetic
gate is closed; a complete national food model is **not** closed.

Next: regional/agronomic calibration and wider demographic profiles; integrate
preserved dated lots, nutrients and processing/transport/energy/work requirements
with the seasonal matrix; extend other food sectors, animal rations and resource
cycles. The current synthetic crop area is not measured available land, and
retention estimates are not batch food-safety evidence. Reviewed live draft
integration follows those evidence gates; no automatic authority is introduced.


### Automation and downstream seasonal supply — 11 November 2026

Implemented a 60-month, 17-resource, 29-process graph joining the existing
crop/seed/land/water model to processing, delivery, robot-cell installation,
qualification and shared work. Eleven fixed-candidate comparisons distinguish
feasible adoption/fallback paths from resource failures. Three independent checks
cover matrix feasibility, chronological capability use and unchanged-expiry FEFO.
Human needs do not depend on retained employment. Source-reviewed automation
context is separate from synthetic scenario coefficients.

Next: calibrate typed crop/product preservation and conversion losses; connect
external energy/material/robot-kit/training-place boundaries to upstream capacity,
constructed storage and age-qualified replacement. Then persist reviewed scenario
snapshots/drafts in the Rust service with stale-state and operator-approval gates.
Do not jump from synthetic labour savings to a national fiscal or employment claim.

### Upstream production and finite-capital gate — 11 November 2026

Schema 14 implements one 27-resource/42-process graph linking the previous rapid
food-automation path to extraction, generation, refining, packaging/upkeep,
robot-kit manufacture, training intake and infrastructure construction. Assets
retire by cohort; shared storage expires with its buildings. Ten candidates retain
two feasible schedules and eight failures. Demand-matched generation can satisfy
the same needs using a smaller fuel deposit that fails the fixed schedule.

Next: typed physical coefficients, material conversion losses/residues, resource
recycling and a rolling horizon with committed work carried across boundaries.
Add upstream transport and measured energy/warehouse constraints; then persistent
scenario snapshots and operator-reviewed draft integration. Financial schedules
for the expanded graph require a separate exact circulation mapping. No national
calibration, complete circular economy or live authority is implied by this gate.

### Rolling commitments and material gate — 11 November 2026

Schema 15 retains absolute history across replanning: eight checkpoints of the
60-month upstream graph preserve pending construction, crop-stage commitments,
stocks and cohort expiry. Small-fixture horizon extension/repeated checkpoints
are verified; retroactive edits and lifetime resets are rejected. A separate typed
material chain accounts for primary losses, residue recovery and remaining waste,
including the energy/capacity costs and exact mass in flight.

Next: preserve original dated food lots in compact rolling state; version and
persist reviewed scenario evidence with stale-state protection. Add measured
multi-constituent recipes, retired-asset disassembly, waste handling and upstream
logistics. The current full-prefix method requires unchanged physical schemas and
is a bounded offline verifier, not an operational national planning controller.

### Dated checkpoint gate — 11 November 2026

Schema 16 adds identity-preserving food-state checkpoints. All 60 end-period
boundaries of the existing upstream food graph restore and continue to the same
final state. Original expiry, pending harvest, movements, consumed seeds, actual
withdrawals, damage and expiry counters remain distinct. Origin tombstones prevent
ID reuse; revisions and a separately trusted reference reject stale/altered data.

The trusted reference is an offline caller precondition, not a newly implemented
security service. Package 1 has since implemented persistent versioned scenario
state and
reviewed application; package 2 remains partially complete. Unit-level purity, food
safety, original observation truth,
recipe migration, quarantine/recall and institutional approval remain open.
