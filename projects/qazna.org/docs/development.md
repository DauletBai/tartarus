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

Verified on 2 November 2026 with Rust 1.97.0 and PostgreSQL 16.10: 3 unit tests and 68 integration tests. Coverage includes allocation/reservation races, rollback injection, upgrades from 0.1/0.2/0.3/0.4/0.5/0.6, needs independent of balances, hierarchy scope, durable shortages, shared production capacity, cross-period stock races, exact overflow assessment, production completion, losses, terminal races, provenance chains, immutable norm revisions and proposals with shared resource/capacity budgets, immutable drafts, stale-plan rejection, complete batch rollback and competing draft/direct actions. The HTTP smoke scenario checks persistent holds, cancellation, completion and provenance and draft application across three restarts. Backup restoration remains a separate future check.

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
| `GET /health/ready` | Database and current planning-draft schema availability |
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

Further contracts: [provisioning](provisioning.md), [needs](needs.md), [production planning](production.md), [completion and provenance](completion.md), [norms and advisory planning](norms-and-planning.md), [reviewed drafts and application](drafts.md). There is no web UI, public issuance API or Python integration yet. See [architecture](architecture.md) for deployment boundaries.
