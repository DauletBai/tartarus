# Qazna

Qazna is a research system for meeting human needs, based on the Tenge model described in *Tartar and I*. Its architectural guide is a living organism: needs, provisioning units, resource circulation and feedback. Profit and market pricing are not default objectives.

The project lives in `projects/qazna.org/` in [Tartarus](https://github.com/DauletBai/tartarus), alongside the book and research archive. The MVP and subsequent production development share **Rust + Tokio + Axum + SQLx + PostgreSQL**. Python/HiGHS is planned for optimization; React/TypeScript is planned for the interface. Project documentation, code comments, examples and contributor materials are maintained in English for an international community.

## Implemented capabilities

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

Version 0.5.0 records operator-confirmed production. It does not independently verify reports, authorize issuance or automatically count production as a resident's receipt. Accounting norms and capacity declarations remain explicit experimental inputs. Automatic plan selection, approved monetary policy, user roles and the web interface are still pending. This is a local research MVP, not a production-ready national monetary system.

## Run the checks

Requirements: Rust (tested with 1.97.0), Python 3.11+ and PostgreSQL 16+. From the repository root:

```sh
cd projects/qazna.org
cargo fetch --locked
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/check.py
```

On Linux, set `PG_BIN` to the installed PostgreSQL directory, for example `/usr/lib/postgresql/16/bin`. Checks create a private, socket-only temporary cluster and stop and delete it afterward. Existing databases are not used.

## Documentation

- [Organism-inspired principles](docs/organism.md)
- [Architecture and readiness boundaries](docs/architecture.md)
- [Allocations and provisioning](docs/provisioning.md)
- [Needs, units and feedback](docs/needs.md)
- [Production capacity and planning](docs/production.md)
- [Production completion, losses and provenance](docs/completion.md)
- [Development, API and verification](docs/development.md)
- [Initial analysis and historical plan](docs/analysis-2026-11-02.md)
- [Changelog](CHANGELOG.md), [contributing](CONTRIBUTING.md), [security](SECURITY.md)

## Licensing

Code is **AGPL-3.0-or-later**, see [LICENSE](LICENSE). Documentation in this project is **CC BY-SA 4.0**, see [LICENSE-DOCS](LICENSE-DOCS). [NOTICE](NOTICE) defines the scope. These licenses do not automatically apply to the parent book, research archive or external materials.

The [dependency inventory](THIRD_PARTY_LICENSES.json) records locked Rust package versions and upstream license declarations. Dependency source and compiled distributions are not shipped here; distributing them requires the applicable upstream notices.
