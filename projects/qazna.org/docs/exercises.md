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
