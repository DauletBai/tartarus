# Qazna architecture

Decision of 2 November 2026: the MVP and industrial development use the same Rust core. A temporary Go server followed by a Rust rewrite is outside the plan. The earlier Qazna repository provides requirements and test scenarios; its implementations are not automatically included in this runtime.

## Technology boundaries

| Component | Technology | Status |
| --- | --- | --- |
| Accounting, provisioning and HTTP API | Rust, Tokio, Axum | Implemented core modules |
| Persistent storage | PostgreSQL 16+, SQLx, SQL migrations | Implemented; tested against real PostgreSQL |
| Economic optimization | Python, NumPy, SciPy HiGHS | Research code exists; integration pending |
| Advisory AI tests | ChatGPT-authenticated Codex CLI, isolated Python adapter | Opt-in synthetic tests; no model execution authority |
| Web interface | React, TypeScript, Vite | Planned |
| User identity | OIDC, roles and account ownership checks | Planned; currently a local operator API |
| Background work | PostgreSQL jobs and transactional outbox | Implemented for ordered Rust planning; Python integration pending |

The [organism analogy](organism.md) guides the domain. One Rust application currently contains domain, ledger, provisioning, needs, production, completion, planning, draft application, coordination and API modules. A future Python worker will receive versioned inputs and return a plan; it will not have write access to ledger postings. Neither a solver nor a language model determines authoritative balances.

Cargo.lock pins dependencies. The declared minimum Rust version is 1.88; the verified toolchain is 1.97.0. SQLx is 0.8.6. Library upgrades must pass tests without requiring a language or database migration. References: [Axum](https://docs.rs/axum/0.8.9/axum/), [SQLx](https://docs.rs/sqlx/0.8.6/sqlx/).

## Exact accounting

The core uses integer minor units; their economic denomination and relation to the provisioning basket remain undecided. One ledger has one accounting unit and no currency conversion. Transfers range from 1 through 9223372036854775807; ordinary balances range from zero through that maximum. JSON amounts are strings. Fractions, exponents, leading zeros and JSON numeric amounts are rejected.

`accounts` defines account types, `journal_transactions` records commands, and `postings` records signed changes. Balances are derived from postings rather than stored in an independently mutable cache. PostgreSQL numeric aggregates prevent overflow during summation.

Commands lock the global idempotency key with a PostgreSQL transaction advisory lock, compare the normalized request, then lock accounts in UUID order before writing. READ COMMITTED is explicit. Repeating a key and request returns the original receipt; changed content conflicts. Technical failures roll back the command and leave its key reusable. A lost response is retried with the original key.

Deferred constraints enforce balanced postings, at least two entries and valid balances. Entries can only be appended in the transaction that created their journal operation. UPDATE, DELETE and TRUNCATE are rejected for history tables. A database owner or superuser can override these protections; they do not replace access control, backups or independent audit.

There is no application issuance endpoint. API account creation and transfers exclude the issuer. Initial money is introduced only by a synthetic fixture in an empty demo database. The fixture establishes no monetary policy.

## Provisioning and needs

Migration 0002 adds allocations, physical lots, reservations and terminal events while preserving the original ledger. Resident settlement couples physical consumption to matching accounting postings; cancellation releases both holds. Old transfer keys keep their original receipts. See [provisioning](provisioning.md).

Migration 0003 adds immutable units, provider assignments, needs and provisioning attempts. Need registration never checks money or stock. A successful attempt creates its reservation and need link atomically; a blocked attempt stores reasons and observed availability. Blocked business observations consume their idempotency key, unlike technical failures. Coverage separates promised and received quantities. See [needs](needs.md).

The normal order is command key, need when applicable, reservation for a terminal operation, accounts in UUID order, then the resource lot. Unit trees require existing parents and cannot be silently reorganized. Provisioning is restricted to the assigned unit and descendants. Reports group physical units separately.

## Production planning and completion

Migration 0004 adds daily capacity pools, immutable recipes and atomic capacity/input reservations. Residents and production share one physical inventory. An assessment is a consistent, read-only REPEATABLE READ snapshot; a plan rechecks under locks. Expected output is not stock. See [planning](production.md).

Migration 0005 adds terminal production completion, input losses and output provenance. Completion consumes every reserved input and the planned capacity. Usable output is created in the same transaction and must match the recipe and executing unit. Zero-output completion creates no empty lot. Capacity projections distinguish reserved and consumed capacity; completing work does not make used daily capacity available again. See [completion](completion.md).

Production planning locks its command key, capacity and input lots in UUID order. Cancellation locks the plan first. Completion locks the plan, capacity, output producer account, then input lots. It never acquires an existing account lock after an input lot lock. Database constraints supplement application checks and serialize competing terminal actions. Historical migrations remain byte-identical; updates are additive.

## Readiness boundary

The service binds only to loopback and requires a local operator token. The token permits all operator operations and is not an end-user authorization model. Version 0.11 added a restricted runtime database group, attestor revocation and a logical restore rehearsal. Before an external pilot, implement OIDC, roles and account ownership, protected transport, load limits, production monitoring, operational backup/PITR and threat review. The [6 November review](architecture-review.md) records current source findings, test limits, legacy mechanisms worth reimplementing in Rust and international collaboration boundaries.

The same technology stack does not imply immediate industrial readiness. Economic hypotheses and infrastructure guarantees are verified separately. Operator declarations and production confirmations are recorded evidence claims, not independent verification or automatic permission to issue money.

## Next work

The [current roadmap](completion-roadmap.md#current-implementation-order) governs
sequencing. A common opening funding envelope is now implemented for six declared disruption
schedules with fixed physical reserves. Eight joint feedback policies now compare
reserve targets, produced equipment and training timing across six paths, with
terminal floors and an exact energy-outage lower bound. This is offline research;
no planner gains ledger authority. Next extend reserve continuity to perishable goods, in-transit loss
and expiry, and cohort/input calibration. Connect independently verified candidates
to versioned Rust reviewable drafts only after these contracts are explicit.
Resident identity, corrections/disputes, accessible interfaces, operational load
limits and disaster recovery remain prerequisites for consequential external use.
The offline experiments do not automatically close these runtime gaps.

## Norms and proposal selection

Migration 0006 records immutable norm revision chains and atomically links applied norms to new needs. Existing needs are never rewritten by a revised norm. Read-only proposal selection uses a consistent snapshot and shared budgets for stock and capacity, with explicit caller order and separate accounting gaps. Expected output remains a forecast. See [norms and planning](norms-and-planning.md) for policy, limits and application boundaries. The [local Git workflow](git-workflow.md) provides commits and IDE tracking while preserving the workspace's separate manuscript material.

## Reviewed draft application

Migration 0007 persists server-generated proposal snapshots and their explicit applied/discarded decisions. Application locks all participating needs, capacities, accounts and resource lots in that class order, then reuses transaction-scoped reservation and production services. It requires unchanged proposal observations (excluding the snapshot timestamp), sufficient aggregate accounting measure and at least one action. Selected holds, terminal event, action links and the idempotent response commit together. No production output or resident receipt is inferred.

Database constraints verify complete action attribution and prohibit retrospective links. Consistent reads provide the immutable proposal and current state; creation replay retains the original response. See [drafts](drafts.md) for contracts, recovery and outstanding forecast limitations.

## Coordination and advisory AI

Migration 0008 adds a transactional command outbox, immutable monitor scopes, recoverable calculation jobs and saved results. The background worker produces draft proposals using `ordered-planner-v1`, without applying them. Leases fence stale workers; changed calculations generate drafts and identical results reuse their previous draft. This is bounded coordination, not a global optimizer or a trained economic model.

An optional one-shot Python adapter runs Codex with the author's ChatGPT authentication for synthetic live tests. It minimizes input and accepts only completed, tool-free structured commentary. Rust independently checks each cited quantity/subject before appending a report. Model prose remains untrusted and does not alter economic state. See [coordination](coordination.md) and [AI safety](ai-safety.md).

## Operational feedback (v0.9)

Migration 0009 exposes active need-linked commitments and permits versioned planner v2 results without changing historical migrations. Current calculations use v2; historical v1 proposals remain immutable and require a fresh review before application. Scoped machine tokens separate aggregate observation and review from operator action. Consistent status queries and [synthetic exercises](exercises.md) make failures and known evidence gaps visible. The comprehensive [organism audit](organism-audit.md) separates implemented responsibilities from missing infrastructure and governance.

## Coupled research matrix (7 November 2026)

The offline `horizon` module now compiles retained equipment, housing and qualified
capacity into shared-use rows, with explicit withdrawals and delayed availability
of new assets. `verify_cash_links` independently matches every priced input,
resident provision and completed public-capital obligation to a physical plan.
These verifiers do not write ledger entries or replace the operator API.
`examples/coupled_study.rs` combines these constraints with exact circulation
prefix bounds and an independent physical replay. See [equations, scenarios and
limits](physical-circulation.md). The live system still needs an explicit, reviewed
integration of this time-expanded planning model.

Brand source and generated favicons live only in `assets/brand/`. README uses
relative references; a future web build should serve or copy those same assets
at build time. Local technology badges derive from Cargo metadata and declared
requirements, not an unverified claim that a release is the latest available.

## Offline plan improvement

`optimization::trim_plan` accepts only an exactly verified seed. It reduces integer
process starts with an incremental stock replay, cached next-period capability
floors and terminal reserve constraints. Every accepted pass is independently
checked against the original matrix. Process order and the pass budget are explicit;
a no-change pass establishes coordinate irreducibility only. The coupled example
reconstructs goods/payment links and exact opening funds after improvement. This
module has no database access, HTTP endpoint, solver dependency or AI authority.
The [comparison and counterexample](physical-circulation.md#reducing-excess-production-while-preserving-provision)
explain why it is a research improvement procedure rather than a global optimizer.


### Causal commitment and recovery experiment

`replanning::ProductionPipeline` records accepted process starts and pending outputs
with exact delayed release, sequential periods and idempotent command keys. It
rejects changed retries and checks all arithmetic before mutation. It is an offline
research component, not a persistent runtime queue. The coupled example adds slower
reserve transport routes and daily feedback from current capacity/inventory, with
prefix comparisons between scenarios that share observations but differ in future
outage duration. Independent full physical and payment replay verifies each trace.
The physical policy is causal; its separately calculated prefunding bound is still
retrospective. Production loss/expiry, a fixed cash envelope and global rolling
optimization remain open. See [results and equations](physical-circulation.md#causal-recovery-through-reserve-routes).


### Common scenario funding

`mathematics::common_schedule_opening` computes exact accountwise worst-prefix
requirements over bounded named schedules and returns attaining witnesses. The
coupled study replays six causal physical scenarios with the same opening money,
checks their identical initial asset/norm/recipe basis and reduces each positive
opening coordinate by one to exercise necessity. This is an offline fixed-family
certificate; it does not optimize reserves, authorize issuance or cover disabled
payment edges. The cash-only normal-funding counterfactual does not re-solve
physical outcomes after blocked payments.


## Offline reserve lifecycle

`Horizon::compile_reserves` adds closing-stock constraints tied to completed storage
assets on top of existing physical/capability/withdrawal rows. `reserve_loss` and
`required_reserve` provide exact rounded losses and a minimal component sizing
recurrence. The coupled research example now has 18 dimensions/17 processes and
retains an independent stock/loss replay and full goods/payment reconciliation.
Storage-aware refill respects built space and prioritizes current provision; standby
conversion protects restart fuel. A defective fill policy remains a negative case.
These are offline research controls, not new runtime endpoints or permissions.
Food expiry, transport losses and equipment maintenance are still separate work.

## Offline physical lifecycle boundary

`lifecycle` provides exact dated-lot transitions and finite-resource standby
maintenance. Lot IDs and expiry survive transfers; FEFO consumption, partial
transit damage, late-arrival expiry and contiguous periods preserve physical
conservation. Maintenance pauses when labour is missing and has no capacity during
service or overdue periods. `coupled_study` invokes a separate five-year lifecycle
submodel and publishes it alongside the existing coupled calculations. It does not
change the database lot schema, enforce expiry on live reservations, modify
payments or grant AI execution rights. See [lifecycle results](physical-circulation.md#dated-food-transport-loss-and-standby-maintenance).

The full coupled experiment also uses `DatedStock` and an explicit generator-cohort
history. Dated eligibility and service availability constrain actual starts;
expiry/damage enter the aggregate matrix as discard, maintenance consumes
endogenously produced metal/energy and shared capacities, and input/provision
cash links are rebuilt. The first integrated 19-resource, 18-process stage had no automatic
execution route; aggregate LP feasibility is checked alongside lifecycle guards.


The schema-9 target food scenario uses 21 dimensions and 20 processes, including
constructed grain and packaged-food storage. Its processing recipe consumes
packaging metal and additional energy; it creates a dated preserved product.
Transport retains the source deadline. Future stock positions exclude expired or
late-arriving lots. A tested restart allowance covers the power/processing/route
lead chain; the nominal-only failure remains recorded. The live API is unchanged.
Storage method, package, climate, food safety and nutrient profiles are specification
gates rather than inferred properties of a long numerical expiry date.


The schema-10 offline extension adds first-phase repair stock and a separately
capacity-constrained second workday (22 dimensions, 21 processes). Qualified
work is charged on each day; paused repairs cannot create completion certificates.
The matrix and an independent queue replay verify the same traces. See
[daily work constraints](physical-circulation.md#daily-maintenance-work-and-unavailable-technical-teams).
This adds no live service endpoint or execution privilege.

## Offline nutrient and order requirements

`nutrition` now models typed nutrient definitions/units, reference profiles,
uncertain composition/retention, per-profile constraints and explicit daily
household-order snapshots. It compiles complete nutrient rows into the existing
exact matrix verifier; incomplete evidence cannot certify a basket. Production
requirements use edible yields and losses, with named upstream input dimensions.
Cooking and delivery are independent order properties. This is a pure Rust
calculation annex: no new API, migration, automatic substitution, ledger mutation
or food-safety approval. The live provisioning graph still uses its existing
resource definitions. See [nutrition scope and results](physical-circulation.md#nutrient-needs-agricultural-requirements-and-household-orders).

## Offline seasonal production constraints

`seasonal` extends `Horizon` with explicit crop calendars, shared occupied land,
stage irrigation budgets, mandatory delivery of every declared need and closing
stock floors. Matrix compilation and independent chronological replay check the
same proposed plan by separate paths; arithmetic is integer/checked. Growth-term
limits bound extension size. A 60-month cereal/feed annex derives demand from
existing orders and tests harvest loss, restoration, irrigation and unavailable
feed capacity. Its model assumes preserved stock without deterioration; dated
lot/retention integration and other food sectors remain open. No endpoint,
migration, autonomous crop allocation or monetary action was added.
`nutrition::sequential_retention` propagates unknown stages and outward-rounded
bounds; sourced preparation estimates retain raw-mass basis and uncertainty.
See [seasonal formulas and limits](physical-circulation.md#seasonal-cereal-and-feed-balance--11-november-2026).

## Shared seasonal capabilities and automation scenarios

`Seasonal::compile_with_capabilities` composes mandatory seasonal provisioning
with `Horizon`'s shared prior-period stock capacities. Its paired replay checks
use independently; completion cannot supply current-period machine/crew capacity.
No withdrawal API is added to this composition, preventing inconsistent stock
balance semantics between compilers. Existing seasonal results are unchanged.

`examples/support/automation_study.rs` connects the seasonal cereal/feed model
to identity gross-equivalent preservation/distribution routes, installed cells,
training capacity and resource consumption. Fixed schedules have exact matrix,
chronological and FEFO checks; shortages remain visible. This is an offline
research annex, not a general automation optimizer or a runtime plan. Existing
APIs, migrations, permissions and operator-confirmed application are unchanged.
See [formulas, assumptions and limits](physical-circulation.md#automation-seasonal-output-and-dated-distribution--11-november-2026).

## Upstream capital cohorts

`capital::CapitalCycle` wraps `Seasonal` with `AssetLife`, shared capabilities and
`StoragePool`. Compilation constrains asset discard to scheduled retirement,
removes start-of-period expiries from usable capacity and checks shared storage.
The independent replay schedules cohort expiries from actual proposed starts and
checks the same stock/capacity/storage facts without evaluating matrix rows.
Assets cannot also be consumed early or injected as untracked boundary supplies.
One storage pool per asset resource prevents double allocation of warehouses.

The upstream example replaces recurring industrial boundary supplies in the
rapid-automation scenario with explicit finite-deposit production and replacement
construction. A bounded demand-matching candidate demonstrates the difference
between failure of a fixed schedule and feasibility of another. Calculation is
still offline and aggregate; no authoritative service or accounting behavior is
changed. See [formulas and limits](physical-circulation.md#upstream-production-and-finite-life-capital--11-november-2026).

### Offline continuation and constituent validation

`rolling::RollingCapital` freezes a verified prefix of a `CapitalCycle` on its
original timeline. Future scenario inputs may change; historical facts and recipe,
capital-life and storage schemas may not. Starts before the boundary retain their
completion/expiry and crop-stage commitments. Snapshots expose opening stocks,
work in progress and cohort dates. They are diagnostic views, not importable or
authenticated execution snapshots. Full-prefix matrices remain bounded by Horizon
limits; compact state and reviewed runtime persistence are future work.

`materials::MaterialBasis` checks exact per-recipe constituent conservation and
per-boundary stock/delivery/disposal/work-in-progress identities. It requires
explicit unit/source metadata and rejects unrecorded losses or invented recovery
mass. The example's kg subchain is separate from the synthetic upstream graph;
it does not certify chemistry, safety, or monetary issuance. See the maintained
[rolling/material experiment](physical-circulation.md#rolling-commitments-and-conserved-material--11-november-2026).

### Offline identity-preserving food state

`dated::DatedFlow` tracks a single declared physical unit across resource locations,
immutable origins, future births, movements and withdrawal/loss counters. Its
versioned canonical JSON can be restored only against independently trusted bytes,
matching scope/revision and structural/conservation validation. That reference is
an explicit caller precondition, not implemented storage/authentication. Updates
are transactional on an in-memory copy; ID history survives expiry and restoration.

The scenario adapter verifies every boundary of the existing 60-month food path.
This prepares a state contract for a future reviewed runtime integration; it does
not add a database snapshot table or bypass the 16 KiB HTTP limit. Original food
safety, observation authenticity and identity-route assumptions remain explicit.
