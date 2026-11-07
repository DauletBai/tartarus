# Synthetic operational exercises

The harness uses six lightweight actors aged 8, 16, 30, 45, 70 and 85, each with food, water and energy needs, and a provisioning unit with a cooking process. Ages are illustrative metadata; seeded quantities are experimental scenario inputs, not validated age norms. The API has no resident identity model yet: the harness acts as an explicit operator on their behalf. Scenario demands are seeded; generated UUIDs, interleavings and timestamps are not byte-reproducible.

## Run

Build the Rust binary, then run from the project directory:

```sh
CARGO_INCREMENTAL=0 cargo build --locked
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/exercise.py --seed 7 --tick 1 --steps 40 --dashboard
```

The runner starts its own socket-only PostgreSQL cluster and refuses to reuse an output directory. It never accepts a database URL or remote attack target on its command line. It creates a separate synthetic genesis, starts the real Rust server and worker on loopback, then stops only its own processes and deletes its own temporary database. Output defaults to ignored `.local/exercises/DATE-SEED/`. `--output` selects a new artifact directory.

The printed loopback dashboard URL is available during the run. The same `report.html` remains viewable offline afterward. No credentials are placed in HTML or sent to external services. No third-party chart dependencies, browser model code or CDN are used. JSON values are escaped before rendering. All actors and reports are synthetic; model calls are zero by default.

Artifacts:

- `events.jsonl`: timestamped HTTP request/results, named evidence and consistent snapshots. It contains synthetic IDs and request data but no authorization headers or credentials.
- `report.json`: actor assumptions, named scenario outcomes, sample series, failures and readiness limitations.
- `report.html`: dimensional needs table, queue chart and scenario evidence table, refreshing once per second when served by the runner.
- `calculation.json`: a saved synthetic calculation for an optional explicit model review.
- `service.log`: local Rust process diagnostics.

## Scenarios and expected outcomes

| Scenario | Expected evidence |
| --- | --- |
| Forged model quantity / removal of operator approval | HTTP 400, no accepted review |
| Invalid key, observer/reviewer transfer, reviewer application | HTTP 401/403 |
| Same-key retry, changed payload, overdraft, integer overflow | Identical receipt or appropriate rejection, no duplicate debit |
| Reviewed plan with a changed balance | HTTP 409 stale rejection |
| Replanning an already active need-linked production cycle | Committed forecast shown, no duplicate production suggestion |
| Eight concurrent requests for one available unit | Exactly one hold; cancellation releases it |
| Production failure, losses and repeated completion | Recorded cancellation/shortfall, terminal event cannot repeat |
| Actual output delivered to residents | Receipt is distinct from a production forecast |
| Worker outage and restart | Durable pending work; five-second backlog alert in wall-clock runs; eventual queue drain |
| Trusted operator declares nonexistent goods | Accepted declaration; scenario oracle reports a **known gap**, not successful fraud detection |
| Continuous checks | Zero balance, resource, capacity or need violations at each observed snapshot |

The regular `scripts/check.py` executes two accelerated scenarios (seeds 7 and 19) in additional disposable databases. Their subsecond ticks deliberately do not test a real five-second alert delay. A wall-clock run with `--tick 1` checks that delay. The observer has a three-second SQL statement deadline; missing observations are failures, never fabricated successful samples.

This is a bounded operational drill, not a full penetration test or load benchmark. It does not test real identity fraud, collusion, external networks, phishing, malware, hardware outages, long-term demographic evolution, disaster restoration or a complete monetary policy.

## Optional AI commentary

After inspecting `calculation.json`, an explicit ChatGPT-authenticated test can run:

```sh
python3 scripts/ai_review.py --input .local/exercises/RUN/calculation.json --output .local/exercises/RUN/ai-review.json
```

The existing adapter minimizes inputs and validates evidence. A one-shot model explanation is separate from the one-second deterministic monitoring loop. Subscription access does not imply unlimited background service, API credit or a production availability guarantee. Real-time protection never waits for this commentary.

## Multi-period circulation exercise

`scripts/circulation_exercise.py` adds a separate continuous strict-evidence
experiment. It starts an owned disposable PostgreSQL cluster and real loopback
Rust API, enrolls synthetic inspectors/custodian/recipients with distinct tokens,
produces goods each period, attests input and output lots, funds recorded needs
from the finite fixture and confirms actual synthetic receipts. No new money is
issued. Every production and settlement is retried with its original key; selected
reservations receive concurrent identical requests. The API restarts every 30
periods against the same database. Each period reconciles physical output,
consumed capacity, recipient receipt and account balances with a separate oracle.

```sh
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/circulation_exercise.py \
  --steps 180 --tick 1 --output .local/circulation-live
```

The output must be a new directory; use another ignored local destination when
rerunning. This restriction protects earlier raw evidence. The maintained public
summary is [`research/circulation-results.json`](../research/circulation-results.json).
Raw timestamped HTTP events and a private process log stay in the local output.
The runner records their digest, actual binary/harness hashes and duration. Tokens
are not included in the event log. One tick represents one synthetic day: 180
periods are not 180 days of real-world observation.

The 6 November wall-clock run completed **180 periods in 182.282 seconds**, made
**9,352 HTTP requests**, performed **five API restarts** and **six concurrent
reservation replay groups**. Every observed invariant and independent balance
reconciliation passed. An injected output loss first caused a physical shortage
at zero-based period 11. The finite treasury first became insufficient at period
56; its final amount was 1 while the provider held 999 of the initial 1,000 units.
No later simulated need was erased to manufacture full coverage. This is an
expected economic counterexample: sum conservation alone does not supply a
repeatable circulation rule. Software correctness is not success of that funding
policy.

The default full check adds 60 accelerated periods in another empty disposable
cluster, covering a restart, losses and treasury depletion without claiming a
wall-clock endurance test. The separate original exercise covers the background
worker outage and queue recovery. Neither drill proves real-world independence
of the scripted attestors, absence of collusion or production readiness.

Validation on 6 November 2026: the complete `scripts/check.py` suite passed, including 15 mathematical tests, 91 PostgreSQL integration tests, exact regeneration of the horizon results, backup/restore checks, and the additional 60-period circulation run (4,548 HTTP requests, one API restart). The separately preserved 180-period report remains the longer paced run, with its own executable and harness hashes.

## Shared-entitlement circulation exercise

`--closed` selects the explicit candidate described in
[closed circulation](computation-and-ai-costs.md#closed-circulation-candidate--7-november-2026).
It uses the same fixture with 1,000 initial units, six residents, separately
credentialed synthetic attestors, and the real Rust/PostgreSQL API. Each period:

1. Production consumes 36 declared grain units and produces 18 meals; inspectors
   certify the lots. The resource-to-money measure is fixed at ten per meal.
2. The treasury allocates 30 to each resident. Six three-meal needs are reserved,
   confirmed by custodian and recipient, and settled. Each personal balance is zero.
3. Retail receives 180. Its declared intermediate payment is 144 to production;
   production pays 72 to raw materials and 18 to equipment; raw materials pay 18
   to equipment. These upstream monetary contracts do not constitute independently
   verified upstream physical supplies.
4. The common provision entitlements are 36 from retail, 54 from production,
   54 from raw materials and 36 from equipment. Existing operator-authorized
   transfer commands credit these to the single treasury. Productive accounts close
   at zero, the treasury at 1,000; the total measure remains 1,000.

The trace has neither a wage eligibility test nor interest, loans, a tax or new
issuance. It assumes the declared residual belongs to common provision from the
outset. The generic transfer API enforces accounting and access rules; it does
**not** independently enforce that ownership assumption or validate the economic
basis of an operator's command. An approved entitlement registry and complete
physical/monetary coupling remain implementation work.

Every transfer is replayed with its original key and both endpoint balances are
checked against the independent oracle. Selected transfers and reservations receive
four concurrent retries. Besides the regular 30-period restarts, the API restarts
in period 20 after the production-to-raw-material payment but before the rest of
the cycle; the completed transfer is replayed before continuation. No cycle-level
all-or-nothing claim is made: durable individual operations support this explicit
recovery sequence.

The runner now copies the executable into its ignored private output directory
before starting, preventing concurrent builds from changing the binary used by
later restarts. Published provenance records the pinned executable, harness and
event-stream hashes. Raw synthetic HTTP traces and the executable remain local.
The full check also exercises 35 accelerated closed periods in its own disposable
database, including both restart positions. It keeps the older 60-period depletion
counterexample as a separate regression, rather than changing it to pass by adding
an unrecorded funding source.

The public `research/circulation-results.json` groups maintained completed traces
by scenario, retaining the earlier result and its provenance. These are software
and synthetic-evidence trials, not observation of a real economy or a national
throughput benchmark.


Validation on 7 November 2026: the final pinned 180-period run passed in
**284.448 seconds**, with **19,939 HTTP requests**, **six API restarts** and
**36 groups of concurrent identical requests**. All 3,240 declared meals were
recorded received across the six synthetic residents. Every period closed with
1,000 in the treasury, zero at retail and the upstream units, and zero at residents;
no physical or accounting shortage occurred in this deliberately sufficient case.
The preserved failed-supply and finite-treasury cases remain separate.

The full final `scripts/check.py` run passed: 22 mathematical tests, 91 PostgreSQL
integration tests, the existing unit/Python/example suites, both exact report
regenerations, HTTP restart/evidence tests, backup/restore, a 60-period depletion
trace, a 35-period closed trace (3,901 requests, two restarts), and both seeded
actor exercises. These counts are test coverage, not a statistical failure-rate
estimate or a demonstration of nationwide throughput.
