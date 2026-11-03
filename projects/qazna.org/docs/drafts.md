# Reviewed planning drafts and atomic application

Version 0.7.0 persists server-generated provisioning proposals and applies their selected stock and production actions in one transaction. The operator decides whether to proceed. Saving a draft creates no holds, transfers, output or receipts.

## API workflow

All routes require the local operator bearer token. POST routes require a global `Idempotency-Key`. Drafts contain no user-specific access policy; the API remains loopback-only.

| Method and path | Contract |
| --- | --- |
| `POST /v1/planning/drafts` | Save `{ "id": "UUID", "plan": { ... } }`; `plan` uses the [assessment request](norms-and-planning.md) |
| `GET /v1/planning/drafts/{id}` | Read the original request, immutable proposal, current state and action links |
| `POST /v1/planning/drafts/{id}/apply` | Confirm all reviewed actions with `{ "basis": "Operator decision" }` |
| `POST /v1/planning/drafts/{id}/discard` | Close a pending draft without actions, with a stated basis |

Creation computes its own proposal; clients cannot supply an arbitrary proposal to execute. The request preserves the caller's candidate and priority order. The result includes `format_version: 1`, `state: "pending"`, UTC `created_at`, `request`, `proposal`, null `outcome_basis` and an empty `actions` array. Review the returned proposal, including residual need, forecast surplus and accounting shortages, before applying its ID. The earlier `/v1/planning/assess` route remains read-only and does not save a draft.

States are `pending`, `applied` and `discarded`. Only pending drafts can transition. Applied drafts return one action link for every reviewed selection, ordered by need, stock before production and candidate order. Links include `need`, `kind`, `reservation`, `attempt` and `production_plan`; irrelevant IDs are null. Individual reservation and production endpoints provide their subsequent execution state. The proposal's `advisory` flag describes the original snapshot even after the draft has been applied.

Repeated requests with the same key and payload return the original response. In particular, replaying creation after application still returns its original pending response; GET returns the current state. A different payload on that key returns `idempotency_conflict`. Reusing a draft UUID with a new creation key returns `draft_exists`.

## Revalidation and transaction boundaries

Creation takes a consistent REPEATABLE READ observation. It releases the initial command-key transaction before opening the observation transaction, then reacquires the key and checks for a concurrent replay before saving. It never waits for a second pool connection while retaining the first. The observation can become outdated before the draft is saved; application always revalidates it.

Application locks the command key, draft, all referenced needs in UUID order, all candidate capacities in UUID order, all participating resident/provider accounts in UUID order and all candidate resource lots in UUID order. Unused candidates participate in locking as well. This extends the existing single-action lock orders. It recomputes the proposal under these locks, compares every returned field except `observed_at`, and rejects changed results with `draft_stale`. This is equality of the reviewed proposal, not a revision counter for every inventory event: changes that leave all returned observations and selections identical do not invalidate it.

It then checks aggregated accounting gaps and creates each resident hold and production plan in the reviewed sequence, with one terminal draft event and action links. Existing reservation/planning logic runs inside the same outer transaction. Either every selected action and command receipt commits, or all of them roll back. There are no separate per-action command keys to retry independently after a failed batch.

Migration 0007 adds immutable drafts, terminal events and normalized action links. Deferred constraints require all reviewed action positions, exact stock quantities and production inputs, matching need scope/dimensions, and actions created in the closing transaction. The draft itself must have been committed earlier. Partial application, retrospective attachment and later action insertion are rejected. Previously released migrations 0001-0006 remain unchanged. These constraints supplement application validation; database owners can override protections and require separate access controls and audit.

## Errors and recovery

| Error | HTTP | Operator response |
| --- | --- | --- |
| `draft_not_found` | 404 | Check the draft ID |
| `draft_exists` | 409 | Retrieve the existing draft or use a new UUID |
| `draft_closed` | 409 | Read its terminal state; retry a lost successful response with its original key |
| `draft_stale` | 409 | Create and review a fresh draft; optionally discard the old one |
| `insufficient_funds` | 409 | Resolve the accounting gap explicitly, then create and review a fresh draft |
| `draft_empty` | 409 | Review residual needs and candidates; there are no selected actions to apply |
| `storage_unavailable` | 503 | Retry the same command key after recovery |

A failed application leaves the draft pending and does not consume its key. It does not store a blocked reservation attempt; the saved proposal remains the review record. Funding an account after review can change the accounting observation and therefore requires a fresh draft. Discarding requires no fresh assessment and never releases holds from an applied draft.

## Boundaries

An applied production plan reserves inputs and capacity; expected output does not become real stock, resident coverage or permission to issue measure. The operator must record completion, then separately reserve and confirm delivery of resulting stock. Stock holds are promises; settlement records receipt. Neither kind of application transfers money immediately.

Drafts can contain partial provision with nonzero residual need; application does not invent missing resources. There is no hidden prioritization or automatic funding. The deterministic, bounded selection policy remains unchanged.

Active production linked through an applied draft is now counted as `committed_production` in planner v2. It reduces the remaining planning gap after stock selection, never actual uncovered/unmet quantities or receipt. Excess commitments are reported as `forecast_excess`; they are not automatically cancelled. Cancellation or completion removes an active forecast. Completed output must be explicitly selected as stock and delivered. Standalone production and cross-need surplus are not automatically attributed. The batch locks capacities of existing commitments as well as new candidates to serialize terminal changes during revalidation.

Existing v1 draft history remains readable. A pending v1 proposal differs from planner v2 and is rejected as stale; create and review a new draft. There is no batch reversal; individual cancellation routes release unconsumed holds while retaining draft audit links.
