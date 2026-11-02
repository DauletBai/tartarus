# Needs and provisioning units

Introduced in 0.3.0, this module registers needs independently of resident balances and stock. A failed provisioning attempt preserves its reasons and observed availability; the need remains registered.

## Responsibility hierarchy

`POST /v1/units` accepts `id`, `parent` (UUID or null), `name`, `function` and `basis`. A community root may coordinate a child food unit. Multiple independent roots are allowed. Parents must already exist and records are immutable, preventing cycles and silent reassignment. The technical depth limit is 64 levels.

`GET /v1/units/{id}` returns the parent, direct children and linked provider accounts. `POST /v1/units/{id}/providers` accepts `producer` and `basis`. An account has one unit assignment. Reassignment and reorganization require a future versioning mechanism.

A resource used to meet a need must belong to a provider assigned to the responsible unit or its descendants. This administrative scope is separate from production dependencies: production inputs may come from other units.

## Register a need

`POST /v1/needs` accepts:

```json
{
  "id": "00000000-0000-0000-0000-000000000008",
  "resident": "00000000-0000-0000-0000-000000000003",
  "assigned_unit": "00000000-0000-0000-0000-000000000006",
  "period": "2026-11-02",
  "category": "food",
  "unit": "meal",
  "quantity": "12",
  "basis": "Declared need in an experimental scenario"
}
```

The resident must have a `resident` account and the assigned unit must exist. Quantity is a positive integer string; unit is an exact physical-unit code. One record is allowed per resident, day, category and unit. Categories are food, water and energy. Quantity is explicit, not automatically derived from age, physiology or a normative registry.

Registering a need creates no allocation or posting. Needs and allocations participate in the same provisioning scenario but are distinct records. Quantity amendments, withdrawal of needs and period closure are not implemented.

## Coverage quantities

`GET /v1/needs/{id}` returns:

| Field | Meaning |
| --- | --- |
| `requested` | Declared quantity |
| `received` | Receipt confirmed through a linked reservation |
| `reserved` | Active promise; receipt not yet confirmed |
| `uncovered` | requested - received - reserved |
| `unmet` | requested - received |

A need for 10 meals with 4 reserved has uncovered 6 but unmet 10. After receipt of those four, received becomes 4 and reserved becomes 0; uncovered and unmet both become 6. Cancellation increases uncovered without inventing receipt. Production completion alone never changes these quantities.

## Attempt provision

`POST /v1/needs/{id}/reserve` accepts `resource`, `quantity` and `basis`; the resident comes from the need. Category and exact physical-unit code must match. The system does not convert units or determine nutritional equivalence. Accounting measure remains separate.

Success returns `status = reserved` and `reservation_id`. The reservation, need link and attempt are committed together. Existing reservation `/settle` and `/cancel` routes resolve the hold; need projections reflect these events.

A business blockage returns HTTP 200, `status = blocked`, null reservation_id and one or more reasons:

| Reason | Meaning |
| --- | --- |
| `physical_shortage` | Insufficient free stock in the selected lot |
| `accounting_shortage` | Insufficient available accounting measure |
| `coverage_limit` | Requested reservation exceeds the remaining uncovered quantity |
| `quantity_limit` | Quantity multiplied by accounting norm exceeds the numeric limit |

The snapshot contains `uncovered_before`, `resource_available`, `accounting_available` and `required_measure`; the last is null on overflow. Reasons describe a selected lot at a point in time, not proof that no alternative exists anywhere.

Blocked attempts create no postings and do not reduce the need. Requests are all-or-nothing; a smaller partial amount must be requested explicitly. An invalid provisioning route (`provider_outside_scope`, HTTP 409), incompatible resource, missing identifier or database failure is not stored as an economic shortage.

## Retries and history

Mutating routes require the operator token and global `Idempotency-Key`. A blocked observation is a successfully stored result and consumes the key. Its replay returns the original snapshot even after funding or stock changes; reassessment requires a new key. A rolled-back technical failure leaves the key unused.

`GET /v1/needs/{id}/attempts` returns the latest 50 attempts in reverse time order. A successful attempt keeps its original reserved status after receipt; GET the need and reservation for current state. `last_attempt_reasons` is also a historical observation, not continuously refreshed diagnostics.

## Unit coverage

`GET /v1/units/{id}/coverage?period=2026-11-02` counts needs assigned to the unit and its descendants once each, grouping by category and exact physical unit. Meals and milliliters are never added together. Aggregates are strings and can exceed the maximum of a single account.

This is a report of responsibility for needs, not a ranking of the unit that physically delivered resources. A parent may have needs supplied by child units. Automated provider choice, priorities and replanning are not implemented; production capacity is handled separately.

## Migration guarantees

Migration 0003 leaves 0001 and 0002 unchanged. Existing reservations remain operational but unattributed to new needs. A successful attempt must be created in the same transaction as its reservation; old deliveries cannot be relabeled retrospectively.

The service locks the need before accounts and the lot. Application checks and database constraints prevent received plus active reserved quantities from exceeding requested quantities. Failure to record history rolls back the financial and physical holds as well.
