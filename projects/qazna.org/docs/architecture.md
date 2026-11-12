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


## Persistent reviewed dated scenarios — 11 November 2026

Migration 0013 and `scenarios` / `scenario_service` add an operator-only research
state store. It is separate from real inventory, provision and the monetary ledger.
The immutable definition binds a UUID scope, format version 1, physical unit,
ordered unique resource/location names and an assumption basis. Creation stores an
empty generation zero. It cannot import a claimed opening inventory as fact.

A proposal supplies the expected store generation and 1–64 typed commands:
advance, introduce a declared origin, transfer, withdraw or record movement loss.
Rust reads the current approved canonical bytes from PostgreSQL, validates their
scope/unit/resource count/revision, replays the commands and saves immutable
candidate bytes. The flow revision counts commands; the store generation counts
approved batches. A pending candidate is never the current state.

The operator subsequently resolves a committed draft with an explicit basis.
Application locks its scenario, rejects a changed base generation, replays again
and compares the exact candidate before appending an outcome. A different operator
request cannot silently rebase the draft. A stale candidate can be discarded; a
closed candidate cannot be reopened. Only applied outcomes select current state;
historical reads do not roll it back. Global idempotency records share the same
transaction, including deferred failures. Scenario rows, drafts and outcomes reject
UPDATE/DELETE/TRUNCATE; database constraints also require sequential generations,
matching scope and a draft committed before its decision transaction. The latter
check also consults the local inserting transaction's commit status, with the
32-bit tuple XID mapped to a nearby full XID. A restored historical stamp that
happens to equal the new cluster's current XID does not block a committed draft;
SAVEPOINT creation or release does not bypass the separate-commit gate. PostgreSQL's
[XID status implementation](https://github.com/postgres/postgres/blob/REL_16_STABLE/src/backend/utils/adt/xid8funcs.c)
is the reference for this exceptional check; the integration tests cover both
restored collisions and current nested transactions. Historical stamps alone are
not a portable proof that a row was created in the current database transaction.

Dated-command HTTP requests remain limited to **16 KiB**. There is **no raw snapshot upload**.
Clients submit bounded command batches; internal canonical checkpoints are capped
at **256 KiB** (the offline helper's larger limit is not the service contract).
Large scenarios must be explicitly redesigned/partitioned, not silently truncated.
Checkpoints are returned as JSON strings so their exact canonical bytes survive
transport. Do not parse large integer quantities through a floating-point client
and resubmit them without exact-integer handling.

The trusted store is the application-controlled PostgreSQL history. This is not a
signature system or protection against a database administrator rewriting dumps,
disabling triggers or changing code. Database login credentials are not participant
credentials; the operator token remains a local service credential, not a named
human identity or a multi-party approval system. Introductions are reviewed
scenario declarations, not independently attested harvests. No route changes
real lots, transfers, allocations, normative registries or issuance.

The next section extends persistent history to the physical model. Monetary
scenario association remains the next integration stage.


## Reviewed physical model versions

Migration **0014** adds immutable model definitions, proposed input/plan checkpoints
and approval outcomes. A definition fixes ordered resource names and units, period
unit, absolute time origin, provenance and optional dated-state mappings. Generation
zero is an empty model slot, not a zero-stock physical model. Each proposal supplies
a complete `CapitalCycle`, plan, rolling boundary, material bases and expected dated
generations. Rust compiles exact constraints, checks the candidate vector, independently
replays the physical chain and audits each declared material constituent. The service
validates candidates; it does not yet search for or optimize a new plan.

An approved version retains all seasonal/capital inputs, planned stocks, original
asset cohorts, pending starts, constituent evidence and exact referenced dated
checkpoints. Quantities are not summed across different units. Canonical checkpoints
travel as JSON strings, preserving i128 totals that exceed JavaScript's exact integer
range or u64. Arbitrary authoritative checkpoint uploads are not accepted.

On replanning, the new boundary cannot retreat or exceed the old plan. The service
freezes the old plan through the **new** boundary before testing a replacement: newly
promoted history cannot be rewritten in the same proposal. Resource identity, initial
stock, recipes, asset lifetime rules and material bases remain fixed. Future supplies,
needs, capacities and an extended horizon may change when the existing rolling verifier
permits them. Boundary zero preserves initial stock and cohort ages with no committed
starts. Changing the fixed physical schema requires a distinct scenario, not a hidden
revision. Observation corrections and emergency reconciliation remain future work.

Each optional dated binding fixes a single-unit location-to-resource mapping and a
basis for calendar alignment. The candidate must reference the current approved dated
generation, match its period to `boundary - 1` (unstarted at boundary zero), and match
selected opening-stock quantities and units. Model and dated rows remain locked through
approval; dated locks are taken in sorted UUID order. A changed dated generation makes
a pending proposal stale, even if total stock is unchanged. Historic accepted snapshots
stay readable after dated state advances. This verifies **selected boundary-stock
agreement and pinned history**, not a causal match of every lot movement to a production
process. Unmapped stocks, material identity across transformations, physical attestation
are not supplied by this gate. The separate coupling layer below adds monetary association.

The model proposal route alone allows **1 MiB** request bodies; other routes retain
16 KiB. Generated checkpoints are capped at **2 MiB**. Preflight bounds are 32 resources,
48 processes, 120 periods, 16 land pools, 48 crop entries/stock limits, 32 asset lives/
stores and eight material bases/dated bindings. A conservative `T*T*max(P,1)*R` budget
is capped at 20 million before compilation. Four process-wide blocking calculation
slots prevent an unbounded CPU queue; excess work receives retryable 503 `scenario_busy`.
These are bounded demonstrator limits, not national-scale performance guarantees.

A committed proposal requires a later operator decision. Application repeats validation
against the locked approved head and compares exact candidate bytes before appending the
outcome and idempotency response in one transaction. The same immutable-history,
separate-transaction and sequential-generation rules as dated scenarios apply. The
trusted boundary is the service and PostgreSQL deployment; a privileged database owner
can still rewrite data or code. No model route creates inventory, real deliveries,
ledger postings, participant entitlements or monetary issuance.


## Reviewed physical and monetary coupling

Migration 0015 adds immutable `coupled_scenarios`, `coupled_scenario_drafts` and
`coupled_scenario_outcomes`. A definition pins a physical scenario and one explicit
circulation policy: account roles, resource suppliers, process operators/order,
capital resources, unit norms, common-entitlement norms and opening measure.
These are versioned experimental assumptions about common productive ownership,
not taxes, interest, trading money or a universal economic law. Changing them
requires a distinct scenario. Source attribution is recorded in the policy basis.

The operator names an approved physical generation and optional unavailable event
indices; the server derives the schedule. It does not accept a client-supplied
payment list. Each period orders completed-capital payments, resident provision
and process inputs in the declared process order. Capital is paid at completion;
inputs at release; provision at the planned delivery. Each external valued flow
has exactly one goods payment. A same-unit input remains in the physical/valuation
register but creates no fictitious transfer or common receipt. Zero-norm resources
remain in physical verification and create no monetary event. Resource quantities
are multiplied by fixed integer norms; these are not market prices.

For quantity `q`, unit norm `v` and common-entitlement norm `c`, the goods
movement is `G = q*v` and the dependent common movement is `C = q*c`, with
`0 <= c <= v`. Each settled event updates exactly two balances by `-amount`
and `+amount`; hence `sum(balance[t]) = sum(opening)` at every event boundary.
For fixed full-settlement cumulative net movements `N[a,k]`, the opening bound is
`E[a] = max(0, max_k(-N[a,k]))`. It does not guarantee survival of arbitrary
interrupted schedules or steady-state reproduction; those require their own replay.


`Horizon::verify_cash_links` checks physical completeness and exact quantities.
A separate event reconciler checks amounts, endpoints, uniqueness, missing events
and the dependency of common circulation on the goods payment. Common measure
cannot circulate from a failed receipt. The chronological account replay is checked
against the independent schedule replay. Balances/totals use exact integers and
canonical strings; no floating point or implicit issuance is used.

For the fixed fully settling event order, the existing worst-prefix calculation
returns a componentwise minimum opening vector. The supplied opening measure is
not silently increased to that vector. Differences, unavailable events and
insufficient funds are reported; failed transfers move nothing. Per-period balances,
aggregate conservation, resident end-of-period measure and failed event causes are
retained. A funded schedule can meet this synthetic circulation condition; an
underfunded schedule remains a valid **failed research result**. Invalid physical
plans cannot become approved physical versions. Neither result is actual delivery,
a cancellation of needs, nor proof of sufficient national provision.

Proposal and approval lock the coupled root, physical root and sorted dated roots.
They require the current approved physical head and its still-current dated anchors;
a dated change alone makes pending coupling stale. Physical feasibility, independent
replay and constituent checks run again. A later physical boundary freezes the old
physical and monetary prefix through that boundary, including old unavailable events.
Approved history remains readable. Foreign keys bind drafts to approved physical
generations and their own definition; immutable triggers and separate-transaction
review preserve the same approval boundary as the physical layer.

Requests retain the 16 KiB route limit. Policies allow at most 64 accounts; schedules
at most 20,000 events and 256 unavailable indices; canonical evidence at most 8 MiB.
Calculations share the four bounded blocking slots with physical proposals. Runtime
permissions must be reapplied after migration. No coupled endpoint posts to the real
ledger, issues measure, changes a needs entitlement or certifies physical custody.

The maintained synthetic fixture covers 14 periods, three resource dimensions and
two processes: material, food and finite-life equipment replacement. It produces
35 valued physical links and 84 events, with opening measure 160 and settled volume
198. The latter is repeated circulation, not new money or economic output. This
small fixture remains a regression case. The selected extended integration below
now binds the upstream graph and its declared food routes.


### Extended physical/dated integration

`ModelInput.food_routes` optionally declares tracked resource indices, opening-stock
remaining lives, crop processes and origin-preserving 1:1 processing/transport
routes. `src/food_routes.rs` derives dated events from the submitted physical plan;
it does not accept a client-authored authoritative checkpoint. Every process touching
a tracked resource must have exactly one valid rule. Only a declared seasonal crop
with seed input and larger same-resource output can introduce a new harvest origin.
Other tracked routes preserve origin and expiry. Unmapped routes, external food
supplies and incompatible units are rejected. Each period reconciles dated stocks
and expiry with the physical balances and declared disposal.

Rules are immutable across an existing model history, including adding/removing
them. Approval and coupled calculation repeat the audit. Derived evidence records
seed withdrawals separately from final deliveries, expiry, final transit/stock and
the canonical dated state. Optional fields are omitted when absent, preserving old
format-1 checkpoint bytes. No migration or new execution authority is introduced.

The initial `model_scenarios::preview` and `coupled::preview` helpers expose the
same checked calculation for reproducible local examples. They are pure calculations,
not stored approvals: they do not authenticate a supplied physical snapshot, resolve
live dated anchors, post money or establish custody. Runtime routes still lock and
check current approved generations and recompute before review.

The selected fixture has 31 resources, 44 processes and 60 periods. A conserved
packaging-material basis joins finite primary ore, recovery, packaging and explicit
residue/waste; both recovery stages consume energy and shared factory/work capacity.
Food routes currently support same-resource seed/harvest and 1:1 preservation or
transport. Nutrient loss, arbitrary blends, measured spoilage, recalls and empirical
shelf-life certification need separate assumptions and future mappings. Declared
shelf life is not evidence of food safety. The model still does not establish actual
physical delivery or an indefinitely repeatable monetary circulation policy.


### Population registry and demand binding

`src/population.rs` defines bounded, source-described parameters with rational
lower/central/upper assumptions and an explicit unknown state. Entries carry an ID,
unit, region, reference period, recording date, source locator/version and basis.
Illustrative mode allows synthetic inputs. Source-attributed mode rejects inputs
labelled synthetic; it does not authenticate their provenance or certify calibration.
Unknown required demographic inputs prevent projection. Unknown need coefficients
retain missing IDs and a missing central/upper aggregate rather than becoming zero.

The monthly age array has 1,561 bins (0 through an open-ended 130+ bin), at most
32 complete disjoint age groups, 32 need items, 512 parameters and 120 periods.
Opening counts are common and fixed across variants; births and migration counts
are scenario inputs. Mortality uses bounded rates, exact fractional carry per age
group and largest-remainder apportionment across ages. No person disappears at the
last age bin. Fractions use bounded integer ratios and checked wide products;
serialized resulting totals are strings. An explicit 20-billion-person research
ceiling prevents unsupported scales. Gregorian month lengths, including leap years,
determine exposure. Whole opening-month counts contribute need; exits, ageing,
births and arrivals are booked at the end boundary. Inbound ages refer to that
boundary. This coarse exposure convention is not a continuous individual record.

`ModelInput.population` optionally includes the complete study and item/resource
mappings. Mapped units, calendar origin, period count and central projected needs
must match the physical model exactly. Coverage records distinguish requirement,
planned delivery and unmet quantity; unmapped items stay visible. The pure mapping
audit can expose a prospective shortfall using physical balance constraints, but
full model proposal/approval retains the stricter seasonal all-needs gate. It cannot
approve that shortfall as successful provision. The demonstrated old upstream plan
fails this gate for new demand, despite settled payments under its original needs.

Population inputs and mappings cannot be silently changed or removed within an
approved model history. New evidence currently requires a new explicit scenario;
controlled future revisions are still rolling-feedback work. Canonical checkpoints
store derived population evidence. Approval recomputes it and preserves the existing
operator, idempotency, transaction and stale-generation boundaries. The optional
fields are omitted when absent, so earlier checkpoint bytes remain compatible.
No migration, dependency, live entitlement or monetary authority is introduced.

For input status, demographic timing and remaining calibration, see the shared
[input assumptions](input-assumptions.md). Regional and life-stage binding extends
this foundation below.


### Regional and life-stage binding

`PopulationBinding.profiles` is optional; absent fields preserve previous serialized
bytes. `need_profiles.rs` validates a complete disjoint partition of every monthly
age-group's person-days across named regions and joint profiles. Limits are 16
regions, 16 profiles and 256 exposure cells; the registry and calendar retain their
existing bounds. A profile pairs a life stage with support needs, allowing pregnancy
and assisted care together without double counting. Exposure source metadata and
coefficient sources are stored in the approved input.

Each cell/item rule explicitly inherits the age norm or replaces it with a complete
profile norm. Replacement is not an extra baseline. The complete matrix is required;
missing entries, duplicate cells, extra fields and invalid units fail. Zero exposure
contributes zero, while an unknown coefficient for positive exposure propagates
unknown central/upper need. Equal parameter references are aggregated within a
region before integer ceiling; total national need sums regional rounded need.
Distinct parameters are not silently treated as interchangeable.

For group g and month t, `sum(c in g) exposure[c,t] = opening[g,t] * days[t]`.
For region r and item k, `need[r,k,t] = sum(norm buckets b) ceil(exposure[b,t] * norm[b,k])`.
The new profile totals replace the base age totals for mapped physical needs; both
base evidence and profile evidence remain separately visible in the checkpoint.

`local_delivery` covers exactly each declared region and mapped item for every
period. Its sum must equal physical deliveries. The audit reports local shortages
without netting them against another region's surplus; full proposal/approval rejects
any positive mapped shortage. No unrecorded interregional transfer is inferred.
All profile inputs are pinned by the existing immutable population binding and are
recomputed at approval. No migration, dependency, AI authority or ledger mutation
is added. Runtime and checkpoint size limits remain unchanged.

Classification accuracy, lifecycle transitions and actual local access are separate
from these arithmetic checks; see [input assumptions](input-assumptions.md).

## Non-storable services and shared capacity

`src/services.rs` audits an optional `ModelInput.services` against independently
replayed physical/capital state and derived population profiles. `ModelCheckpoint.services`
stores coverage, capacity and operating-input evidence. Absent fields preserve
previous checkpoint serialization. No migration, dependency or new authority is
introduced.

Each service rule binds a region and need item to a complete monthly provision
schedule, same-region capacity pools and operating inputs. All regions must have
each selected service. Unbound needs remain listed. Asset identity is unique
across pools and cannot also back the older independent capacity/storage limits.
Service use and all active development/training commitments share the same monthly
capacity ledger. Expired cohorts and unfinished future capacity cannot contribute.
Operating needs reconcile exactly to designated physical end-use rows.

A shortage blocks preview/proposal approval; a failed candidate does not consume
the retry key or change accepted history. Services require population evidence.
Changing/removing service assumptions in an existing model history is rejected.
The pending and accepted service input/evidence survive restart and logical
backup/restore. Controlled future revisions remain a separate feedback-stage task.
