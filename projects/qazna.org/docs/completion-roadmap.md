# Completion roadmap and feasibility boundaries

Assessment date: 4 November 2026. Reviewed baseline: `3f4a028` (v0.10.0).
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

## Blocking work and acceptance evidence

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
- How measure returns from providers to the treasury, how capacity growth and depreciation are verified, and whether any issuance/redemption is permitted.
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
