# Coordination, exact calculations and AI review

The author's direction of 2 November 2026 adds a coordination layer inspired by the nervous system: observe changes, calculate feasible responses, present plans and learn from recorded outcomes. The approved first operating mode is advisory: **analysis and plan creation are automatic; applying a plan requires an operator command**. A model does not decide rights, create money or replace a participant's autonomy.

Version 0.8.0 implements durable coordination and an opt-in AI test adapter. The calculation engine is explicitly `ordered-planner-v1`, the existing exact Rust algorithm. It is not a trained AI model or a global optimizer. The language model reviews its results separately.

## Layers and feedback

| Organism-inspired role | Qazna component | Current boundary |
| --- | --- | --- |
| Sensory feedback | Transactional command events | Committed application commands; no physical sensors |
| Local protective response | Rust/PostgreSQL invariants | Exact stock, capacity, balance and history checks |
| Working memory | Persistent jobs, results, drafts and reviews | Recoverable records, not model context as the source of truth |
| Coordination | Rust worker and explicitly selected monitor scopes | Ordered selection; no hidden allocation priorities |
| Deliberation | Optional ChatGPT-authenticated Codex review | Questions and evidence-linked commentary |
| Authorized action | Operator application of a reviewed draft | Rechecks current conditions and commits atomically |
| Outcome feedback | Receipt, cancellation and production completion | New events trigger another calculation |

The system should not funnel every local operation through a slow model call. Protective checks run synchronously with writes. Mathematical planning runs in the background. Model review is currently an explicit, bounded test request. A future optimization worker may use Python/HiGHS with independently checked integer results; it will not gain posting authority. See [architecture](architecture.md) and the [AI threat model](ai-safety.md).

## Monitor API

All endpoints use the existing loopback operator token. POST requests require a global idempotency key.

| Method and path | Purpose |
| --- | --- |
| `POST /v1/coordination/monitors` | Register `{ "id": "UUID", "plan": { ... } }` using the existing planning request |
| `GET /v1/coordination/monitors/{id}` | Read scope, active/stopped state, queued/running/failed counts and latest calculation |
| `POST /v1/coordination/monitors/{id}/stop` | Permanently stop this scope with `{ "basis": "Operator decision" }` |
| `GET /v1/coordination/results/{job}` | Read a saved calculation and its draft link |
| `GET /v1/coordination/results/{job}/reviews` | Read untrusted AI commentary attached to that calculation |
| `POST /v1/coordination/results/{job}/reviews` | Save a bounded, evidence-checked report; never execute it |

A monitor's request is immutable and includes an explicit day, need order, stock candidates and process options. It does not discover new needs or automatically roll forward to another day. Stop it and register a new UUID to change the scope. Registration validates the request but creates only the initial queued job; it does not reserve resources. Existing API idempotency semantics apply, including historical creation replay and GET for current status.

## Running the worker

After migrations, run the worker with the same database connection as the API:

```sh
cargo run --locked -- coordinate
```

For a single dispatch/calculation pass:

```sh
cargo run --locked -- coordinate-once
```

The latter prints a result object or JSON null. The continuous worker waits 500 milliseconds between passes and handles Ctrl-C. This is near-real-time background processing, not a guaranteed subsecond deadline: queue depth, database contention and calculation time add latency. An initial calculation plus subsequent committed-command signals drive work; the worker does not continuously spend a ChatGPT subscription.

## Delivery and recovery

Migration 0008 records an event in the same transaction as each relevant command. Advice creation, monitor controls and AI review commands are excluded to prevent feedback loops; explicit application remains a signal. Raw SQL mutations and empty account creation are outside this event contract. Events are not backfilled on upgrade; a new monitor's initial calculation observes the current state.

The dispatcher claims up to 64 undispatched events with `FOR UPDATE SKIP LOCKED`, creates jobs for active monitors and marks events dispatched atomically. It uses no increasing-ID watermark, so a transaction committing late cannot fall behind an already advanced cursor. Monitor registration and concurrent dispatch are covered by the initial calculation and durable events. All active monitors currently receive all relevant events; dependency filtering, coalescing, quotas and retention remain future scale work.

Workers serialize claims by monitor and skip monitors with active leases. Each job receives a fresh token and a 30-second database-time lease. No connection is retained while a separate read-only calculation runs. Calculations have a 20-second application deadline; transient calculation failures retry after one second, up to three attempts. Expired leases can be reclaimed. Finalization checks the token, expiry and monitor state, so an old or stopped worker cannot publish a late result. A failure during finalization rolls back draft/result changes and leaves the lease to expire for recovery.

The result and any new draft commit together. Unchanged proposal contents, excluding `observed_at`, reuse the previous draft, even if that draft has since been closed; read the draft state before acting. Each calculation retains its own observation timestamp and triggering command. A trigger means "reconsider the current state", not "reconstruct the database exactly as of this command". Multiple signals can therefore yield identical results. Queue delivery records are mutable operational state; monitors, stops, results, reviews and economic history are immutable.

Stopping a monitor cancels queued/running jobs. It preserves previously created drafts and their action history. It does not cancel already applied reservations. Failed jobs remain visible in the monitor's failed count. Correct the cause and register a new monitor for explicit recovery; a general retry-management UI is not implemented.

## ChatGPT Pro: live tests only

The author authorized using their existing Pro subscription to test actual AI behavior. `scripts/ai_review.py` invokes the installed Codex CLI using its saved ChatGPT authentication. It checks the login mode, forces ChatGPT authentication, excludes API-key environment variables and has no paid-API fallback. It does not copy tokens, read authentication files itself, purchase credits or change account settings. Availability and limits belong to the signed-in account. The supported scripting/authentication mechanism is documented in [OpenAI's non-interactive Codex guide](https://learn.chatgpt.com/docs/non-interactive-mode).

A local test can review the synthetic calculation exported by the full check script:

```sh
mkdir -p .local/ai
python3 scripts/ai_review.py \
  --input target/last-smoke-calculation.json \
  --output .local/ai/review.json
```

Use a new output filename each time. To fetch an existing local calculation and save checked commentary back to its history:

```sh
python3 scripts/ai_review.py --job CALCULATION_UUID \
  --output .local/ai/review-2.json --save
```

The API mode requires `QAZNA_OPERATOR_TOKEN` in the parent process. That token and `DATABASE_URL` are not forwarded to Codex. The CLI receives only a minimized snapshot, runs in an empty temporary directory with a read-only sandbox and unnecessary capabilities disabled, and must return a completed structured response without tool activity. The adapter never calls a draft application route; it can only read a calculation and optionally append a review through the fixed API paths.

The adapter was checked with Codex CLI `0.164.0-alpha.11.1`. That version emits an exact diagnostic confirming the deliberately disabled Code Mode execution host. Only this specific notice is tolerated alongside a completed text response; other error items and all tool items are rejected. CLI updates require rerunning the adapter tests. Supply `--model` to test a specific available model. If the CLI does not identify its resolved model, the record honestly uses `codex-default`; it is not a verified immutable model revision.

The offline test suite never calls the model. Three accepted live checks on 2 November covered synthetic shortages, a malicious instruction embedded in a unit label, and the calculation exported by the actual PostgreSQL/HTTP smoke test. Their outputs preserved operator approval and cited the supplied quantities. These are smoke/adversarial examples, not proof that all prompts are safe. Local input/output artifacts remain ignored by Git.

For a future distributed application, [Sign in with ChatGPT](https://developers.openai.com/siwc/token-sharing-open-source) provides an explicit app authorization flow for eligible locally hosted/open-source usage. A production model connection, deployment entitlement, identity design and usage budget still need separate implementation. The author's subscription is not an assumed shared production service.
