<p align="center">
  <img src="assets/brand/qazna.svg" width="176" height="176" alt="Qazna — red pulsating shanyrak, continuous circulation">
</p>
<h1 align="center">Qazna</h1>
<p align="center"><strong>Human needs · Shared capabilities · Continuous circulation</strong></p>
<p align="center">
  <a href="Cargo.toml"><img src="assets/brand/badges/version.svg" alt="Crate version 0.11.0"></a>
  <a href="docs/completion-roadmap.md"><img src="assets/brand/badges/stage.svg" alt="Research prototype"></a>
  <a href="LICENSE"><img src="assets/brand/badges/license.svg" alt="Code license AGPL-3.0-or-later"></a>
</p>
<p align="center">
  <img src="assets/brand/badges/rust.svg" alt="Rust minimum supported version 1.88">
  <img src="assets/brand/badges/edition.svg" alt="Rust edition 2024">
  <img src="assets/brand/badges/tokio.svg" alt="Tokio locked dependency">
  <img src="assets/brand/badges/axum.svg" alt="Axum locked dependency">
</p>
<p align="center">
  <img src="assets/brand/badges/sqlx.svg" alt="SQLx locked dependency">
  <img src="assets/brand/badges/postgresql.svg" alt="PostgreSQL 16 tested">
  <img src="assets/brand/badges/python.svg" alt="Python 3.11 or later for tooling">
</p>
<p align="center"><a href="https://github.com/DauletBai/tartarus/actions/workflows/qazna.yml">CI results</a> · <a href="docs/completion-roadmap.md">Completion roadmap</a> · <a href="assets/brand/README.md">Brand assets</a></p>

Qazna is a research system for meeting human needs, based on the Tenge model described in *Tartar and I*. Its architectural guide is a living organism: needs, provisioning units, resource circulation and feedback. Profit and market pricing are not default objectives.

The project lives in `projects/qazna.org/` in [Tartarus](https://github.com/DauletBai/tartarus), alongside the book and research archive. The MVP and subsequent production development share **Rust + Tokio + Axum + SQLx + PostgreSQL**. Rust owns planning inputs and exact validation; an isolated mathematical solver is proposed for larger optimization problems. React/TypeScript is planned for the interface. Project documentation, code comments, examples and contributor materials are maintained in English for an international community.

The service now preserves reviewed physical model versions: seasonal production,
capital replacement, stocks, pending work and exact constituent balances. Replanning
retains committed history and original equipment ages; selected dated stocks can be
pinned and checked. A separately reviewed circulation scenario now derives exact
payments from an approved physical generation and immutable accounting norms.
Missing funds and interrupted payments remain visible; approvals do not change
real money or verified inventory. See [architecture](docs/architecture.md#reviewed-physical-and-monetary-coupling).

## Project information

| Item | Reference |
| --- | --- |
| Stage | Research prototype; [scope and remaining work](docs/completion-roadmap.md) |
| Architecture | [Rust service and exact calculation boundaries](docs/architecture.md) |
| Reproducible evidence | [Physical/circulation studies](docs/physical-circulation.md) · [test record](docs/development.md) |
| Code and documentation | [AGPL-3.0-or-later](LICENSE) · [CC-BY-SA-4.0 documentation](LICENSE-DOCS) |
| Repository activity | [GitHub Actions](https://github.com/DauletBai/tartarus/actions/workflows/qazna.yml) · [pull requests](https://github.com/DauletBai/tartarus/pulls) |
| Parent repository | [Tartarus overview and About](https://github.com/DauletBai/tartarus) |

## Book and implementation

*Tartar and I* by Daulet Baimurza is the author's Russian research essay on language, historical memory and the Tenge economic proposal. Chapter 26 introduces the organism-inspired provisioning idea; its appendices distinguish model assumptions from calculations. Qazna turns selected provisioning mechanisms into executable, testable software. Passing software tests does not establish the book's historical hypotheses or prove the full economic model.

- [Read the Russian HTML edition](https://dauletbai.github.io/tartarus/), with individual sections suitable for browser translation.
- [Download the print PDF](https://media.githubusercontent.com/media/DauletBai/tartarus/codex/qazna-rust-provisioning/book/Tartar_i_Ya_print_6x9.pdf).
- [Explore the book and research archive](https://github.com/DauletBai/tartarus).

The active project is this directory in Tartarus. The separate [DauletBai/qazna.org](https://github.com/DauletBai/qazna.org) repository is archived earlier work. It is not the current runtime. The book's copyright is separate from the software and project documentation licenses.

The [food and standby lifecycle study](docs/physical-circulation.md#dated-food-transport-loss-and-standby-maintenance)
adds dated batches, FEFO, transport damage and finite-resource maintenance. Its 32
five-year submodel runs preserve expiry, failures and unmet needs explicitly;
the full graph now includes a separate integrated comparison with exact cash
replay. The selected [preserved-food reserve design](docs/physical-circulation.md#preserved-food-reserves-the-target-architecture)
adds constructed grain/packaged storage, preservation inputs and restart-aware
reserves. Eleven target scenarios retain early and overlong-failure deficits;
fresh-food traces are historical comparators. Live service integration remains open.

The [automation and seasonal supply annex](docs/physical-circulation.md#automation-seasonal-output-and-dated-distribution--11-november-2026)
now checks five-year equipment/qualification deployment, shared processing and
transport capacity, dated stocks, resource tradeoffs and manual fallback. All
adoption scenarios retain identical human provision; scenario coefficients remain
synthetic and computation requires no AI.

The [upstream capital annex](docs/physical-circulation.md#upstream-production-and-finite-life-capital--11-november-2026)
connects that graph to finite deposits, generation, material production,
construction, retirement and replacement. Its demand-matched schedule preserves
food provision with lower synthetic fuel extraction; physical calibration and
live rolling-plan integration remain open.

The [rolling commitments and material annex](docs/physical-circulation.md#rolling-commitments-and-conserved-material--11-november-2026) preserves pending work and asset ages across replanning and checks explicit residue recovery against constituent conservation. It remains an offline, synthetic experiment.

The [dated checkpoint annex](docs/physical-circulation.md#dated-food-checkpoints--11-november-2026)
retains original lot identity, expiry and in-flight output through all 60 monthly
boundaries. The [completion estimate](docs/completion-roadmap.md#completion-scope-and-remaining-work--11-november-2026)
sets five remaining packages after the physical persistence foundation, separately from
national modelling and pilot readiness.

## Implemented capabilities

- Observe consistent invariant/queue snapshots with a limited observer credential.
- Exercise synthetic residents, production losses, abuse and worker outages with timestamped logs and live/offline reports.
- Avoid duplicate planning against already committed, need-linked production.

- Recalculate monitored needs after committed changes through a durable worker; keep application under operator control.
- Attach evidence-checked AI commentary through an opt-in ChatGPT-authenticated test adapter.
- Apply explicit, immutable revisions of needs norms without rewriting existing needs.
- Select advisory provisioning proposals using shared stock/capacity budgets and caller-defined priorities.
- Save reviewable planning drafts and apply all selected stock/production holds atomically after revalidation.
- Register needs independently of purchasing power; distinguish requested, reserved, received and unmet quantities.
- Organize provisioning units into responsibility hierarchies and report coverage by period and physical unit.
- Preserve shortage observations and their input snapshots.
- Maintain an append-only double-entry ledger with exact amounts, atomic transfers and allocations.
- Reserve physical stock and accounting funds together; confirm receipt or cancel.
- Declare daily capacity and immutable process recipes; assess plans against selected input lots.
- Reserve production capacity and all inputs atomically, sharing physical stock with resident provisioning.
- Complete a production cycle, record input losses and output shortfall, and trace new output to its plan and input lots.
- Preserve idempotency, transaction history and earlier database versions through additive migrations.
- Exercise the local operator HTTP API against real PostgreSQL, including process restarts.

Version 0.10.0 added an opt-in, durable attestation gate for resource use and delivery, with separate inspector/custodian/recipient credentials. The Rust planner calculates exact proposals; model commentary never applies them. It does not independently verify reports, authorize issuance or automatically count production as a resident's receipt. Accounting norms and capacity declarations remain explicit experimental inputs. Global optimization, independent physical attestations, approved monetary policy, resident identity/ownership and the product web interface are still pending. This is a local research MVP, not a production-ready national monetary system.

Version 0.11.0 adds live principal revocation, scoped evidence reads, a restricted runtime database group and a logical backup/restore rehearsal. The shadow food-pilot charter chooses finite treasury-funded shortfall allocations for modelling; broader policy and participant safeguards remain open.

## Exact mathematical research

The offline Rust modules `mathematics` and `horizon` verify rational linear plans,
optimality and infeasibility certificates, delayed production/resource balances,
integer distribution rules and the minimum opening measure for a specified
circulation schedule. They are not wired to ledger authority or enacted policy.
[Mathematical results and proof boundaries](docs/computation-and-ai-costs.md#exact-mathematical-verification-and-five-year-resource-study)
and [continuous strict-evidence HTTP trials](docs/exercises.md#multi-period-circulation-exercise)
include reproducible counterexamples, not only successful scenarios. External
outreach is deferred while this independent computational work proceeds.

The [closed-circulation candidate](docs/computation-and-ai-costs.md#closed-circulation-candidate--7-november-2026)
adds one-treasury daily provision, explicit common-production entitlements,
settlement-delay reserves and zero personal closing balances. It is a bounded
experimental rule; physical completeness and broader policy remain open.

The [coupled physical and monetary study](docs/physical-circulation.md) connects
finite deposits, energy, equipment, transport, construction and training in one
five-year matrix. Shared capabilities and exact goods/payment links are checked
separately from liquidity. Coefficients remain synthetic research inputs.
Its exact integer improvement stage reduces excess starts while preserving each
delivery and terminal reserves; all payments are recalculated. The baseline uses
44.08% fewer power batches than the fixed-output seed. This is a synthetic
comparison, not a calibrated savings forecast or a global optimum.
A causal reserve-route experiment then reduces unmet meals during a 90-day primary
transport interruption by 72.24%, and eliminates them in the 30-day scenario under
the declared stocks/capacities. Future restoration dates are not supplied to the
daily decisions; exact physical and cash checks remain separate.
A further experiment uses one identical opening balance vector across six disruption
schedules: 2,030,483,170,650 synthetic normative units (1.253609% above normal-only
funding). Every scheduled payment settles; physical shortages under severe stress
remain explicitly reported. This is a fixed-family certificate, not a national
budget, global optimum or guarantee against unknown failures.

An eight-policy, six-scenario joint-planning comparison now eliminates the
90-period transport-only meal deficit through additional **produced** equipment,
with unchanged initial assets. Equipment plus food buffers reduces worst unmet
meals to 2.76 billion synthetic units. The minimax service-shortfall fraction reaches
a certified 1.569116% lower bound in the previous configuration without backup
generation or energy storage; the remaining physical interruption is not hidden by cash balances.
See [objectives, exact certificate and limits](docs/physical-circulation.md#joint-reserve-replacement-and-training-policies).

The next [reserve lifecycle experiment](docs/physical-circulation.md#constructed-reserve-and-recovery-experiment)
constructs standby generators and fuel storage from existing resources, accounts
for daily loss, protects restart stock and refills after recovery. A 30-period
reserve covers the prepared repeated-interruption family; 60 periods covers the
60-period supply failure. Thirty-five exact traces retain early/overlong failures
and a failed overfilling policy. This is a synthetic resilience result, not a
national stockpile recommendation or a guarantee against unknown emergencies.

## Run the checks

Requirements: Rust (tested with 1.97.0), Python 3.11+ and PostgreSQL 16+. From the repository root:

```sh
cd projects/qazna.org
cargo fetch --locked
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/check.py
```

On Linux, set `PG_BIN` to the installed PostgreSQL directory, for example `/usr/lib/postgresql/16/bin`. Checks create a private, socket-only temporary cluster and stop and delete it afterward. Existing databases are not used.

## Documentation

- [Physical chains, shared capabilities and goods/payment verification](docs/physical-circulation.md)
- [Operation without AI: mathematical model, exact example, budgets and optional AI costs](docs/computation-and-ai-costs.md)

- [AI economic research: company fit, Kazakhstan partnership routes, proposed experiments and pilot budget](docs/ai-economic-research.md)
- [Architecture, test quality and international readiness review](docs/architecture-review.md)
- [Shadow food-pilot charter](docs/pilot-charter.md)
- [Revocation, runtime privileges and recovery](docs/recovery-and-access.md)

- [Completion roadmap and pilot acceptance gates](docs/completion-roadmap.md)

- [Organism audit and remaining gaps](docs/organism-audit.md)
- [Independent resource and delivery attestations](docs/evidence.md)
- [Live synthetic exercises](docs/exercises.md)
- [Version 0.9 verification evidence](docs/verification-0.9.md)
- [Organism-inspired principles](docs/organism.md)
- [Architecture and readiness boundaries](docs/architecture.md)
- [Allocations and provisioning](docs/provisioning.md)
- [Needs, units and feedback](docs/needs.md)
- [Production capacity and planning](docs/production.md)
- [Production completion, losses and provenance](docs/completion.md)
- [Needs norms and provisioning proposals](docs/norms-and-planning.md)
- [Reviewed drafts and atomic application](docs/drafts.md)
- [Coordination and ChatGPT test workflow](docs/coordination.md)
- [AI safety boundaries and verification](docs/ai-safety.md)
- [Local Git and IDE workflow](docs/git-workflow.md)
- [Development, API and verification](docs/development.md)
- [Initial analysis and historical plan](docs/analysis-2026-11-02.md)
- [Changelog](CHANGELOG.md), [contributing](CONTRIBUTING.md), [security](SECURITY.md)

## Licensing

Code is **AGPL-3.0-or-later**, see [LICENSE](LICENSE). Documentation in this project is **CC BY-SA 4.0**, see [LICENSE-DOCS](LICENSE-DOCS). [NOTICE](NOTICE) defines the scope. These licenses do not automatically apply to the parent book, research archive or external materials.

The [dependency inventory](THIRD_PARTY_LICENSES.json) records locked Rust package versions and upstream license declarations. Dependency source and compiled distributions are not shipped here; distributing them requires the applicable upstream notices.


Earlier bounded extension: [daily maintenance and specialist absence](docs/physical-circulation.md#daily-maintenance-work-and-unavailable-technical-teams).
Two separately constrained workdays prevent repairs from completing without a
technical team; preserved-food and housing outcomes are reported independently
of successful monetary settlement. Coefficients remain synthetic.

### Nutrition and household choice

The offline [nutrition/order annex](docs/physical-circulation.md#nutrient-needs-agricultural-requirements-and-household-orders) checks nutrient constraints before aggregating food requirements. Household cooking, prepared-food orders and delivery are separate choices. Pinned composition data, exact arithmetic and negative tests expose missing evidence and capacity gaps; the sample remains an uncalibrated research model, not a certified national diet.

The [seasonal food annex](docs/physical-circulation.md#seasonal-cereal-and-feed-balance--11-november-2026)
checks 60 monthly crop/seed/land/water balances and reserve replenishment for the
cereal/feed portion of the order snapshot. Exact matrices and independent replay
retain failed scenarios; synthetic land/yield assumptions and missing dated-stock
integration remain visible.


### Reviewed dated scenario state

The service now persists bounded dated research scenarios in PostgreSQL, retaining
original lot identity, age, pending outputs and movements across approved versions.
Only an operator can apply a previously saved proposal; conflicting or stale
approvals fail. This does not update real inventory or money. See the
[HTTP workflow and limits](docs/development.md#persistent-scenario-workflow) and
[architecture](docs/architecture.md#persistent-reviewed-dated-scenarios--11-november-2026).
