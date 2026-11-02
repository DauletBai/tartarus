# Qazna changelog

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
