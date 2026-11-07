# Developing and verifying Qazna

The working directory inside Tartarus is `projects/qazna.org/`. Maintain project files in English. Runtime names and descriptions may still contain supported Unicode; this is not an English-only user-data policy.

## Automated checks

Requirements: Rust with rustfmt and Clippy, Python 3.11+ and PostgreSQL 16+ binaries. No Python packages are required for the check runner. Fetch Rust dependencies once:

```sh
cd projects/qazna.org
cargo fetch --locked
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/check.py
```

The script checks formatting, dependency license inventory, Clippy with warnings denied, unit tests, integration tests and the real HTTP binary across restarts. PostgreSQL runs in a temporary cluster with a private Unix socket and no TCP listener. It is stopped and deleted afterward. Existing user databases are never used.

Ordinary `cargo test` does not run the ignored PostgreSQL tests. The full script invokes them with `--ignored --test-threads=4`. A manual run must set `QAZNA_TEST_DATABASE_URL` to a disposable database in which tests may create schemas. Scenarios run four at a time; requests inside concurrency tests remain concurrent.

Verified on 3 November 2026 with Rust 1.97.0 and PostgreSQL 16.10: 3 Rust unit tests, 11 offline Python safety tests and 81 PostgreSQL integration tests. Coverage includes allocation/reservation races, rollback injection, upgrades from 0.1/0.2/0.3/0.4/0.5/0.6/0.7, needs independent of balances, hierarchy scope, durable shortages, shared production capacity, cross-period stock races, exact overflow assessment, production completion, losses, terminal races, provenance chains, immutable norm revisions and proposals with shared resource/capacity budgets, immutable drafts, stale-plan rejection, complete batch rollback and competing draft/direct actions, transactional event delivery, lease fencing, stopped workers and independent AI evidence validation. The HTTP smoke scenario checks persistent holds, cancellation, completion and provenance draft application and queued coordination across three restarts. At that milestone, backup restoration was still pending; see the current verification record below.

## Manual startup

Create a separate `qazna_demo` database and configure its connection URL. Migrations are a separate command; the server does not automatically migrate on every start.

```sh
export DATABASE_URL='postgresql://USER:PASSWORD@127.0.0.1:5432/qazna_demo'
cargo run --locked -- migrate
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f fixtures/demo.sql
export QAZNA_OPERATOR_TOKEN="$(python3 -c 'import secrets; print(secrets.token_hex(32))')"
cargo run --locked -- serve
```

Replace the URL placeholders with credentials for your separate demo database. The default binding is `127.0.0.1:8080`; `QAZNA_BIND` can select another loopback address. The client shell needs the token; never commit it.

The fixture only runs in an empty database whose name ends in `_demo`. It creates technical issuer account `...0001` with balance -1000, treasury `...0002` with 1000 and resident `...0003` with zero. This is synthetic initial state, not an issuance operation based on verified growth.

## Core HTTP routes

| Method and path | Purpose |
| --- | --- |
| `GET /health/live` | Process liveness |
| `GET /health/ready` | Database and current coordination schema availability |
| `POST /v1/accounts` | Create an empty treasury, resident or producer account with an explicit UUID |
| `GET /v1/accounts/{id}` | Read type, balance and holds |
| `POST /v1/transfers` | Transfer with required Idempotency-Key |

All `/v1/*` routes require `Authorization: Bearer ...`. This is local operator access, not user-specific authorization. Treasury and issuer are singletons; the API cannot create or transfer with an issuer account.

```sh
curl --fail-with-body http://127.0.0.1:8080/v1/transfers \
  -H "Authorization: Bearer $QAZNA_OPERATOR_TOKEN" \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: example-payment-1' \
  -d '{"from":"00000000-0000-0000-0000-000000000002","to":"00000000-0000-0000-0000-000000000003","amount":"100"}'
```

The treasury then holds 900 and the resident 100. Identical retries return the same transaction_id without a second debit, including after restart. Amount `"101"` with the same key returns 409. Insufficient available funds also return 409 (`insufficient_funds`); unavailable storage returns 503. Command keys are global to the ledger.

Local commits and IDE tracking are described in the [Git workflow](git-workflow.md).

Further contracts: [provisioning](provisioning.md), [needs](needs.md), [production planning](production.md), [completion and provenance](completion.md), [norms and advisory planning](norms-and-planning.md), [reviewed drafts and application](drafts.md), [coordination and AI tests](coordination.md). A lightweight live/offline exercise report is available; a product web UI, public issuance API and optimizer integration remain future work. See [architecture](architecture.md) for deployment boundaries.

## Optional model tests and local disk use

`scripts/test_ai_review.py` is offline and runs in CI. Live subscription tests are separate opt-in invocations of `scripts/ai_review.py`; credentials and outputs stay outside version control. The HTTP smoke scenario exports a synthetic calculation to ignored `target/last-smoke-calculation.json` for such a test.

When disk space is constrained, remove only regenerable project build caches and run checks with `CARGO_INCREMENTAL=0`. The growing PostgreSQL suite creates isolated schemas in its disposable cluster; it needs sufficient free disk space and reports storage errors instead of silently skipping scenarios.

## Operational exercises

The required check runner also executes two accelerated synthetic actor scenarios (seeds 7 and 19). For a wall-clock run, dashboard and persistent local report, follow [synthetic exercises](exercises.md). The observer/reviewer credentials and aggregate status contract are described in [coordination](coordination.md).

The final exercise drain allows 90 seconds: a terminated worker may leave a valid 30-second lease, followed by recalculation and queued work. A timeout reports the last queue snapshot. This does not shorten production leases or relax the zero-pending/queued/running/failed completion requirements.

Version 0.10 adds the [attestation protocol](evidence.md) and a real HTTP strict-evidence exercise to the required check runner. The v0.9 counts above describe the prior milestone.

## Verification record — 7 November 2026, physical coupling

The full disposable-PostgreSQL check passed: 3 Rust library tests, 28 mathematical
tests, 4 no-AI example tests, 11 Python advisory-safety tests and 91 PostgreSQL
integration tests. The runner also regenerated the horizon, circulation and coupled
reports byte-for-byte; exercised HTTP restart/provenance, strict attestations,
restricted runtime privileges and backup restoration of 36 tables; and completed
the 60-period shortage and 35-period closed-circulation trials plus actor seeds
7 and 19. Known physical-evidence gaps in actor trials remain documented.

A final reporting-only addition exposes discarded quantities; the coupled report
was regenerated and compared again, and Clippy passed for all targets. These
synthetic checks establish the stated program invariants, not observed national
productivity, long-term economic stability or readiness for public deployment.

## Verification record — 7 November 2026, exact plan improvement

The mathematical target now has **34 passing tests**, including six trimming
checks: fixed deliveries and indivisible batches; delayed shared capabilities and
retirement; refreshed cached capacity after downstream work reduction; terminal
reserves and malformed requests; 32 exhaustive-oracle scenarios with recycling and
overflow; and a counterexample separating coordinate irreducibility from global
optimality. The added cache-regression test and all-target Clippy were rerun after
the full check had begun; no production code changed during that addition.

The full disposable-PostgreSQL runner passed. It regenerated all three maintained
research reports byte-for-byte, including the five paired 1,825-period comparisons;
passed 91 PostgreSQL tests, HTTP restart/provenance, both strict-evidence checks,
runtime privileges and backup restoration of 36 tables. Its 60-period shortage
trial made 4,548 HTTP requests with one restart; the 35-period closed trial made
3,901 with two restarts. Both passed; actor seeds 7/19 completed with their existing
physical-evidence limitation still reported. No model calls or real participants
were used. The original seed scenario values were also checked unchanged after
refactoring payment generation, and the documentation's 20 physical table values
were compared to the generated report.
