# Production capacity and input dependencies

Version 0.4.0 introduced feasibility assessment and atomic reservation of declared capacity and physical inputs. This extends the organism analogy: a provisioning unit has a function, finite capability and dependencies on other units. The operator chooses the process, capacity period and actual input lots. Profit maximization is not the objective.

## Declarations and versions

`POST /v1/production/capacities` accepts `id`, `unit` (unit UUID), `period` (YYYY-MM-DD), `capacity_unit`, `quantity` and `basis`. A kitchen might declare 120 `oven_minutes` for a day. Only one declaration exists for a unit/day/capacity-code combination. All recipes using it share its limit. Different capacity codes must represent genuinely independent facilities: the system cannot detect one machine declared under two names.

`POST /v1/production/recipes` accepts an immutable process definition:

```json
{
  "id": "00000000-0000-0000-0000-000000000010",
  "unit": "00000000-0000-0000-0000-000000000007",
  "name": "Pack a portion",
  "capacity_unit": "packing_minutes",
  "capacity_per_batch": "2",
  "output_category": "food",
  "output_unit": "packed_meal",
  "output_per_batch": "1",
  "inputs": [
    {"slot": "meal", "category": "food", "unit": "meal", "quantity_per_batch": "1"}
  ],
  "basis": "Experimental process specification, revision 1"
}
```

Top-level `unit` identifies the provisioning unit; an input's `unit` is its physical-unit code. Quantities are positive integer strings. Capacity and input coefficients are per complete batch; output_per_batch is expected output per batch. Input slot names are unique, with at most 32 slots. An empty input list can represent an explicitly declared source process but is not proof of cost-free production.

The recipe UUID identifies an immutable revision. A changed specification gets a new UUID and basis; existing plans retain their old recipe. Automatic supersession, capacity revisions and approved needs norms are separate future mechanisms. Capacity is an operator declaration, not an independent equipment inspection.

`GET /v1/production/capacities/{id}` returns the declaration, `reserved`, `consumed` and `available`. Consumed capacity was added in 0.5; completed work continues to occupy its daily allowance. `GET /v1/production/recipes/{id}` returns the full process definition.

## Feasibility assessment

`POST /v1/production/assess` accepts the same body as plan creation:

```json
{
  "recipe": "00000000-0000-0000-0000-000000000010",
  "capacity": "00000000-0000-0000-0000-000000000009",
  "batches": "4",
  "inputs": [
    {"slot": "meal", "resource": "00000000-0000-0000-0000-000000000005"}
  ],
  "basis": "Packing plan for subsequent provision"
}
```

The recipe and capacity must belong to the same unit and use the same capacity code. Each input needs exactly one distinct lot matching category and physical unit. Its producer account must belong to a provisioning unit. Inputs may come from other branches: production dependencies differ from administrative responsibility. The result identifies each input's `supplying_unit`.

Category and unit alone do not establish substitutability: food measured in grams does not describe composition or quality. The operator explicitly selects a suitable lot. Splitting one input across lots or using one lot in multiple slots is not supported; consolidate the per-batch requirement into one slot where appropriate. Unit conversion is not performed.

The result includes:

- `max_batches`: the minimum whole-batch limit from free capacity, every selected input and the storable output limit;
- `planned_output`: expected output for the requested batch count;
- `capacity_required`, `capacity_available`;
- each input's `required`, `available` and `shortage`, in its own unit;
- `feasible` and reasons `capacity_shortage`, `input_shortage`, `quantity_limit`.

For example, 12 free minutes at 2 minutes per batch allow 6 batches, but water for only 4 batches reduces the limit to 4. More accounting measure does not increase that physical limit. Balances and accounting norms are not used in production feasibility.

Assessment uses a consistent PostgreSQL REPEATABLE READ snapshot and reserves nothing. Its result can become stale immediately. i128 arithmetic and string serialization preserve exact required quantities even above the i64 storage limit; such plans are blocked. `max_batches` concerns the selected lots and capacity, not the optimal plan for the whole system.

## Plan reservations

`POST /v1/production/plans` takes the same body plus `Idempotency-Key`. Capacity and input lots are rechecked under locks. Success returns `status = reserved`, `plan_id` and the pre-reservation assessment. All inputs and capacity are held in one transaction. No money is posted.

Shortage returns HTTP 200, `status = blocked`, null plan_id and a snapshot. This observation is stored in the global command journal. Identical retries return the old result even if conditions change; a new evaluation with reservation needs a new key. Invalid requests, incompatible units and technical failures roll back without consuming the key. Input order does not change request meaning.

`GET /v1/production/plans/{id}` returns the original request, planned output and current state: reserved, cancelled or, since 0.5, completed. `POST /v1/production/plans/{id}/cancel` accepts a basis and command key. It releases all inputs and capacity atomically while preserving history. Repeating the same key is safe; cancelling a closed plan with a new key gives 409. [Completion](completion.md) consumes the reserved cycle instead of releasing it.

`reserved` in `/v1/resources/{id}` includes both resident and production holds. Plans for different dates also share the same physical stock; future expected receipts are not available inputs. A capacity period is a calendar day, not an intraday schedule. Reservations do not expire automatically.

## Guarantees and limitations

Migration 0004 preserves previous migrations and active reservations. Definitions and histories are immutable. Inputs cannot be appended to a previously committed recipe or plan. Deferred constraints verify complete input selection, exact coefficients, capacity compatibility and physical stock. Normal lock order is command key, plan for terminal actions, capacity, then lots in UUID order. Completion additionally locks the output producer account before input lots.

Planning alone creates no finished stock, fulfilled need, money or issuance entitlement. Since 0.5, explicit completion records production, losses and output provenance. Independent verification, logistics, substitutions, multi-resource capacity constraints and automatic priority selection remain future work. The first successfully committed reservation gets the available stock; no policy prioritizes production over residents automatically.

Only existing lots can be inputs. Expected output of another active plan is not stock; circular promises cannot bootstrap inventory. A full technology graph and automatic plan selection from needs remain future tasks. All routes require the local operator token.
