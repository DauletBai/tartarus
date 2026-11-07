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

## Verification record — 7 November 2026, causal reserve routing

The mathematical target has **38 passing tests**. Four new pipeline checks cover
merged delayed arrivals and exactly-once release; changed and historical retries;
multiplication/addition/date overflow with unchanged state; and 24 independently
replayed commitment-list scenarios, each running 90 periods with changing route
starts and lead times. All-target Clippy passed. The 10 previous physical/cash
outcome sets remain unchanged: deliveries, shortages, disposal, annual stocks,
original process starts and opening measure. Matrix dimensions and zero activity
slots expand for the two reserve routes.

The maintained schema-3 report includes four recovery comparisons and executable
prefix-equality assertions before different future conditions are observed. The
full stock/capacity replay and exact payment verifier cover each new trace. A
fixed-route Farkas bound is now explicitly labelled with the no-reserve-dispatch
condition; it is not misapplied to the expanded routing policy.

The renamed two-tone SVG and generated favicons were visually inspected in Chrome
on light/dark backgrounds, with animated flow and 16/32/64-pixel icon previews.

The full disposable-PostgreSQL runner passed: 38 mathematical, 91 PostgreSQL,
3 library, 4 no-AI example and 11 advisory-safety tests; exact regeneration of all
maintained research reports; HTTP restart/provenance; strict evidence at both
privilege levels; and backup/restore of 36 tables. The 60-period shortfall exercise
passed 4,548 HTTP requests with one restart; the 35-period closed exercise passed
3,901 with two restarts. Actor seeds 7 and 19 completed, with their existing
physical-evidence gap still explicit. No real participants or model calls were used.


## Verification record — 7 November 2026, common scenario opening

The mathematical target has **42 passing tests**. Four added checks cover a
counterexample to taking only the largest scenario total; exhaustive opening-vector
enumeration for 16 small three-scenario families with an independent transfer
simulator, prefix witnesses and scenario-order invariance; malformed families and
large exact integer results; and failure of the funding guarantee when a scheduled
payment edge is unavailable. All-target Clippy passed.

The schema-4 study supplies one opening vector to six scenario replays with an
identical initial physical/recipe/norm basis. Each positive opening coordinate is
reduced by one to exercise its attaining witness. Three added power/compound traces
pass the independent physical and payment checks; future-observation physical
prefix comparisons extend to the compound recovery dates. Every previously
published case, improved case, comparison and recovery result is unchanged.

The author's updated red values (`#ff0815`, `#cd0011`) were retained exactly;
PNG/ICO/static SVG assets were regenerated and the woven 512-pixel result visually
inspected. The base remains `#ff000d` at 30% opacity. SVG palette matching and PNG
chunk checks passed; no animation geometry or timing was changed.

The required full disposable-PostgreSQL runner passed: 42 mathematical, 91 database,
3 library, 4 no-AI example and 11 advisory-safety tests; byte-for-byte regeneration
of all maintained calculation reports; HTTP restart/provenance, strict evidence
under both privilege configurations, and backup/restore of 36 tables. The 60-period
shortfall trial made 4,548 requests with one restart (55.255 seconds), and the
35-period closed trial made 3,901 with two restarts (46.151 seconds). Both passed.
Actor seeds 7/19 completed with their existing physical-evidence limitation still
reported. No real participants, real money or model calls were used.

## Joint policy verification — 7 November 2026

The schema-5 coupled report evaluates eight precommitted feedback policies over
six 1,825-period paths. Each of the 48 cases undergoes exact matrix feasibility,
independent stock/capability replay, complete physical-payment-link verification,
common-funding cash replay and terminal-floor checks. All 2,596,456 movements settle
with no later issuance; every resident daily provision flow retains its existing
zero-closing-balance rule. Forty equal-observation prefix comparisons pass.

For the combined 30-period power outage, the sparse dual certificate proves zero
housing-service delivery in periods 392–421. Deleting a required multiplier or
relaxing the energy-storage premise causes the exact-zero certificate check to
fail. The minimax coverage result attains this physical lower bound; monetary and
food-only optimality are not claimed. All previous numerical cases, trimmed cases,
comparisons, recovery and common-funding results were checked unchanged, apart
from the added certificate field.

The coupled regeneration step now uses `cargo run --locked --release --example
coupled_study`. This avoids the high debug-mode cost of the expanded exact report;
Rust assertions and certificate checks remain active. CI compares the entire
regenerated report byte for byte. No dependency, database migration, runtime API,
monetary authority or brand asset was changed by this stage.

The complete required runner passed: 42 mathematics, 91 PostgreSQL, 3 library,
4 no-AI example and 11 advisory-safety tests; Clippy/format/license checks; all
three maintained reports regenerated byte-identically; HTTP restart/provenance,
two strict-evidence privilege configurations and 36-table backup/restore. The
60-period shortfall trial passed 4,548 requests and one restart in 56.068 seconds;
the 35-period closed trial passed 3,901 requests and two restarts in 48.685 seconds.
Actor seeds 7/19 completed, retaining the documented physical-evidence limitation.
The disposable PostgreSQL cluster was stopped and removed by the runner. No real
participants, real money or AI-model calls were involved. The preceding GitHub
commit `60c05fa` also passed CI; this entry does not assume a result for a future run.


## Constructed reserve verification — 7 November 2026

Six new mathematical tests cover exhaustive physical storage allocations, delayed
storage completion, malformed/self-supporting/duplicate constraints, exact loss
bounds including maximum integers, rejected delivery of lost stock, and minimum
reserve sizing against exhaustive forward simulation. There are now 48 mathematical
tests. The state-dependent fuel-loss recurrence is also replayed independently of
the compiled matrix in every coupled trace.

Schema 6 adds 35 five-year reserve/recovery traces and 25 equal-observation prefix
checks. Acceptance assertions require ordinary provision, zero fuel overflow and
terminal floors for the four storage-aware designs; successful prepared 30-period/
repeated scenarios and the 60-period design's 60-period interruption; and visible
failure before construction or beyond endurance. The deliberately uncapped refill
variant must still fail normal provision and exhaust the finite fuel deposit.
All 1,862,357 scheduled movements in the new traces settle using five common opening
vectors, with no later issuance. This financial result includes scenarios retaining
physical shortages and is not a claim of universal provision.

Seventy-one historical report records were compared with the preceding commit:
original unmet quantities, waste, activity totals, terminal balances, provision/
capital measure, first-shortage periods and circulation outcomes are unchanged
when inactive extension columns are removed. Matrix dimensions necessarily grow
to include the constructed assets and their constraints. Existing reserve-free
optimality certificates also include the disabled backup-generator capacity rows.
The historical report files are updated in place; no parallel publication tree or
new dependency is introduced. The preceding commit `07ac64b` passed GitHub CI.

The final required full runner passed: 48 mathematics, 91 PostgreSQL, 3 library,
4 no-AI example and 11 advisory-safety tests; format/Clippy/license checks;
byte-identical regeneration of all three maintained reports; HTTP restart/
provenance; both strict-evidence privilege configurations; and 36-table backup/
restore. The 60-period shortfall exercise passed 4,548 requests and one restart in
55.766 seconds; the 35-period closed exercise passed 3,901 requests and two restarts
in 46.767 seconds. Actor seeds 7/19 completed with the existing physical-evidence
limitation still reported. The runner stopped and removed its disposable cluster;
no real participants, real money, existing databases or model calls were used.

## Food/standby lifecycle verification — 7 November 2026

`lifecycle` adds seven mathematical tests: split/late shipments; FEFO against an
independent individual-unit oracle (324 input combinations); atomic malformed/
overflow commands and identity reuse; exhaustive partial damage; paused service
with insufficient labour/spares; 60 periodic maintenance designs checked against
closed-form capacity/work counts over 500 periods each; and malformed/overflow
maintenance input. It is offline only and introduces no migration or runtime route.

The maintained coupled report is schema 7. Its new isolated lifecycle section
covers 32 five-year paths (58,400 periods), with physical conservation every day
and 28 equal-observation prefix comparisons. All previous report sections were
compared structurally with commit `71deac0` and are unchanged except the schema
number and clarification of scope. Initial pipeline deficits are preserved in
all 32 total shortage counts. Food expiry, finite spares, correlated failure and
fuel exhaustion remain visible counterexamples. The new receipt valuation is not
an additional monetary-schedule proof.

The complete required runner passed: 55 mathematics, 91 PostgreSQL, 3 library,
4 no-AI example and 11 advisory-safety tests; formatting/Clippy/license checks;
byte-identical regeneration of the three maintained reports; HTTP restart and
provenance checks; both strict-evidence privilege configurations; and 36-table
backup/restore. The 60-period shortfall trial made 4,548 requests with one restart
in 56.496 seconds. The 35-period closed trial made 3,901 requests with two restarts
in 46.798 seconds. Actor seeds 7/19 completed while retaining the known physical
evidence gap. The disposable PostgreSQL cluster was stopped and removed. No
existing database, real participants, real money or model calls were used. The
preceding commit `71deac0` passed GitHub CI; this record does not presume the result
of a later CI run.

## Integrated lifecycle and circulation — 7 November 2026

The schema-8 coupled report adds seven 1,825-period scenarios and a fixed-buffer
counterexample. The full graph now has 19 resource dimensions and 18 processes.
Dated food follows transport, expiry and damage; generator construction and service
share material, energy and work constraints with other production. All 436,726
monetary movements across the seven candidate scenarios settle using one opening
vector without later issuance. This includes two scenarios with physical shortages.
Ordinary operation, supply interruption and transport damage have no modelled food
shortfall under the three-day buffer candidate; food expiry remains substantial.
Maintenance work is front-loaded at service start, and no global age-aware
optimality or national calibration is claimed.

Two additional mathematical tests check 7,168 age-qualified FEFO combinations
against an individual-unit oracle and atomic overflow/ineligible selection. Six
coupled-example tests cover original expiry in transport, cooking versus transit,
nonduplicated damage/expiry, partial repair, fabricated service completion and
undated arrivals. Historical quantities and circulation outcomes were checked
against the preceding report with inactive extension dimensions removed.

The full required runner passed: 57 mathematics, 91 PostgreSQL, 6 coupled-example,
3 library, 4 no-AI example and 11 advisory-safety tests; formatting, Clippy and
licence checks; byte-identical regeneration of all three maintained reports; HTTP
restart/provenance, both strict-evidence configurations and 36-table backup/restore.
The 60-period shortfall exercise passed 4,548 requests and one restart in 55.779
seconds; the 35-period closed exercise passed 3,901 requests and two restarts in
46.469 seconds. Actor seeds 7/19 completed with the existing physical-evidence
limitation still reported. The runner stopped and removed its disposable cluster.
No existing database, real participant, real money or model call was used. These
are local results; a subsequent GitHub CI run is a separate check.


## Preserved-food reserves and restart — 7 November 2026

The maintained report is schema 9 with 21 resource dimensions and 20 processes.
Nine raw-reserve comparisons, eleven target preserved-food paths and one
nominal-reserve counterexample add 38,325 simulated periods. The target consumes
packaging material and additional process energy, builds both food stores and
carries sealed-product deadlines unchanged through transport. Its initial product
endowment is explicitly pre-preserved. The former fresh-food scenarios remain
comparators. All 1,245 checked legacy numerical fields remain unchanged, apart
from inactive appended dimensions; no historical failure was deleted.

The target ordinary scenario has zero food expiry and zero unmet food. Prepared
60-day harvest loss and 30-day correlated power loss also have zero unmet food;
early/longer failures and disabled storage retain deficits. The four-day restart
allowance is tested against a nominal-only counterexample, not claimed as a
global optimum. Source-qualified storage evidence and the required real-product
specification fields are included; no generic shelf-life certification is made.
Across the eleven target paths, 762,673 movements settle under one opening vector
with no later issuance. The nine intermediate paths settle 597,298 movements
under their own common vector. Physical shortage remains independent of settlement.

Three additional coupled-example tests cover future-position exclusion of late
or expired arrivals, preserved deadlines at 2/10/30-year abstract durations with
damage conservation, and fail-closed expiry overflow. Scenario assertions enforce
ordinary/prepared coverage, construction presence, disabled-store failure and
nominal-reserve shortfall. Fixed-size process traversal and uniqueness checks
prevent newly added processes from being silently omitted.

The complete required local runner passed: 57 mathematics, 91 PostgreSQL, nine
coupled-example, three library, four no-AI example and 11 advisory-safety tests;
formatting, Clippy and licence checks; byte-identical regeneration of all three
maintained reports; HTTP restart/provenance, both strict-evidence configurations
and 36-table backup/restore. The 60-period shortfall exercise passed 4,548 requests
and one restart in 55.318 seconds; the 35-period closed exercise passed 3,901
requests and two restarts in 46.183 seconds. Actor seeds 7/19 completed with the
existing physical-evidence limitation. The disposable PostgreSQL cluster was
stopped and removed. The initial sandbox-only attempt could not create PostgreSQL
shared memory; the full runner was subsequently rerun successfully with permission.
No existing database, real participants, real money or model calls were used.
