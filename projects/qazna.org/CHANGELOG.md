# Qazna changelog

## 0.9.0 — 3 November 2026

- Reconcile active need-linked production commitments before proposing new cycles, without counting forecasts as receipt.
- Add migration 0009 and planner v2; retain historical proposals and require fresh review of pending v1 plans.
- Separate operator, aggregate observer and AI reviewer credentials; remove operator-token use from the review adapter.
- Add consistent invariant, dimensional needs and coordination queue telemetry with explicit physical evidence limitations.
- Add lightweight synthetic actors, concurrent abuse attempts, production loss and worker outage/recovery exercises, JSONL audit records and a live/offline HTML report.
- Document an organism responsibility audit, including the demonstrated gap between trusted declarations and physical truth.
- Extend required checks to 81 PostgreSQL cases, 11 offline AI tests and two accelerated HTTP exercise seeds.

## 0.8.0 — 2 November 2026

- Add transactional command events, immutable monitor scopes, durable calculation jobs and background/one-shot worker commands.
- Recover expired leases, bound retries and reject late results from stopped or superseded workers.
- Save exact planning results and pending drafts atomically; reuse unchanged proposals without automatic application.
- Add a ChatGPT-authenticated, one-shot Codex adapter for explicit synthetic tests, with no paid-API fallback or Qazna execution tools.
- Validate structured AI evidence against saved quantities and subjects independently in Rust; reject unsupported evidence and approval bypasses.
- Document the organism-inspired coordination layers, implemented security controls and unresolved deployment/model risks.
- Add ten PostgreSQL scenarios, ten offline Python adversarial tests and queued-work/review HTTP checks across restarts.

## 0.7.0 — 2 November 2026

- Persist server-generated planning drafts with immutable requests, proposals and explicit applied/discarded decisions.
- Revalidate reviewed results under ordered locks and apply all selected stock/production holds in one transaction.
- Keep accounting shortages, changed observations and absent actions distinct; failed applications leave the draft and key available for recovery.
- Preserve action links for later receipt, cancellation and production completion without counting forecasts as delivery.
- Add migration 0007 with complete-action and same-transaction attribution constraints; keep released migrations unchanged.
- Add thirteen PostgreSQL scenarios for concurrency, staleness, rollback, attribution, shared resources and upgrade from 0.6.
- Exercise draft persistence and atomic application through the HTTP binary across three restarts.

## 0.6.0 — 2 November 2026

- Add immutable needs norm revisions and atomic application with an auditable source link.
- Select read-only provisioning proposals from uncovered needs, listed stock and process candidates.
- Share simulated input/capacity budgets across needs; report residual quantities, forecast surplus and separate accounting gaps.
- Preserve caller-defined priority order without introducing a hidden allocation policy or treating expected output as stock.
- Add ten PostgreSQL scenarios, an upgrade from 0.5 and HTTP checks for norm persistence and advisory selection.
- Connect the active workspace to local Git history with a project-scoped sparse checkout and document the IDE/commit workflow.
- Add a scoped parent-manifest update script for local commits.

## 0.5.0 — 2 November 2026

- Complete a committed production plan atomically: consume reserved inputs and capacity, record losses and create usable output.
- Record zero-output cycles without creating empty stock; require an explicit basis for output shortfall.
- Trace output lots to immutable completion reports, recipes and input lots.
- Protect completion/cancellation races, repeated confirmations and rollback after partial writes.
- Upgrade active 0.4 plans while preserving previous migration files.
- Add nine PostgreSQL scenarios and extend HTTP verification across two process restarts.
- Translate all maintained Qazna documentation and contributor materials into English; establish English as the project contribution language.

## 0.4.0 — 2 November 2026

- Declare shared capacity per unit and period, with immutable recipes, inputs and expected outputs.
- Assess feasibility using consistent snapshots, exact shortage quantities and whole-batch limits.
- Reserve capacity and all input lots atomically; cancel to release both.
- Share stock between production and resident provisioning, including plans for different dates.
- Persist blocked observations and idempotent responses without money postings or future stock creation.
- Add an upgrade from 0.3, ten PostgreSQL scenarios and persistent production-plan HTTP checks.

## 0.3.0 — 2 November 2026

- Register needs independently of accounting balances and stock.
- Add provisioning unit hierarchies and provider account assignments.
- Reserve against needs while preserving blockage reasons and availability snapshots.
- Separate received, reserved, uncovered and unmet quantities.
- Aggregate unit/subtree coverage without mixing physical dimensions.
- Keep legacy reservations operational without retrospective attribution to needs.
- Extend concurrency, rollback, migration and restart verification.

## 0.2.0 — 2 November 2026

- Add allocations unique by resident, day and category with a required basis.
- Add resource lots with normative accounting measures and atomic stock/funds reservations.
- Confirm receipt or cancel, with repeat-processing and concurrency protection.
- Introduce a global command idempotency registry while preserving 0.1 transfer receipts.
- Upgrade the ledger without rewriting its history.
- Establish needs, provisioning and feedback as organism-inspired architectural principles.
- Prepare Tartarus publication, project licensing and automated checks.

## 0.1.0 — 2 November 2026

- Introduce the Rust core with Axum, SQLx and PostgreSQL.
- Implement double-entry history, exact amounts, atomic transfers and a local operator API.
- Verify concurrency, idempotency and persistence after restart.
