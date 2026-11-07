# Versioned needs norms and provisioning proposals

Version 0.6.0 adds explicit norm revisions and deterministic, read-only plan selection. It does not infer physiological requirements, choose social priorities or grant entitlements automatically. Operators state the assumptions and select the applicable revision for each resident/day.

## Immutable norms

`POST /v1/need-norms` requires operator authorization and `Idempotency-Key`:

```json
{
  "id": "00000000-0000-0000-0000-000000000012",
  "code": "experimental-meals",
  "revision": 1,
  "supersedes": null,
  "category": "food",
  "unit": "meal",
  "quantity": "3",
  "applicability": "Explicitly selected synthetic resident/day scenario",
  "basis": "Experimental assumption, not an approved nutrition standard"
}
```

A code uses lowercase ASCII letters, digits, dots, underscores or hyphens, up to 64 characters. Revision is a positive JSON integer; quantity retains the positive integer-string contract. Category is food, water or energy. Applicability and basis are descriptive evidence fields, not executable eligibility rules.

Revision 1 has no predecessor. A subsequent revision references `supersedes`, keeps the same code, category and physical unit, and increments the revision by one. Each revision can have one successor; concurrent attempts cannot fork it. Changing dimensions requires a new code. No effective-date policy or automatic selection of the newest revision is implied. `GET /v1/need-norms/{id}` returns a particular revision.

`POST /v1/needs/from-norm` accepts `id`, `resident`, `assigned_unit`, `period`, `norm` and an application `basis`. It copies the selected revision's quantity and dimension into a new need and records the norm link in the same transaction. Ordinary uniqueness by resident/day/category/unit still applies. No funds or stock are required. A later norm revision does not rewrite existing needs. Operators can still register explicit needs through `/v1/needs`.

`GET /v1/needs/{id}/norm` returns the linked revision, or JSON null for a manually declared or legacy need; an unknown need returns 404. Existing needs cannot be retrospectively attributed to a norm. Revision conflicts return 409 `norm_revision_conflict`, missing revisions 404 `norm_not_found`, and invalid chains 400 `invalid_request`.

## Select a proposal

`POST /v1/planning/assess` requires the operator token, but no idempotency key because it writes nothing:

```json
{
  "period": "2026-11-02",
  "basis": "Operator-selected need and candidate order",
  "needs": [
    {
      "need": "00000000-0000-0000-0000-000000000014",
      "resources": ["00000000-0000-0000-0000-000000000011"],
      "production": [
        {
          "recipe": "00000000-0000-0000-0000-000000000010",
          "capacity": "00000000-0000-0000-0000-000000000009",
          "inputs": [
            {"slot": "meal", "resource": "00000000-0000-0000-0000-000000000005"}
          ]
        }
      ]
    }
  ]
}
```

Provide 1-32 distinct needs and at most 64 stock/process candidates in total, within the existing 16 KiB HTTP body limit. Empty candidate lists are allowed and expose residual need. All needs and selected capacity records must match the requested day. Duplicate stock candidates within a need are rejected. Candidate lots and process outputs must match the need's category and unit. Delivering providers and executing process units must lie within the need's assigned responsibility subtree. Process inputs may come from other units under the existing production contract. Even unused candidates are validated, so a malformed list is not silently accepted.

The reported policy is `listed_need_order_stock_then_production`:

1. Visit needs in exactly the caller's order; the server introduces no hidden fairness ranking.
2. For each need, start from its current uncovered quantity, excluding existing active resident promises and confirmed receipts.
3. Select from listed existing stock in order, reducing a shared in-memory stock budget.
4. For the remainder, visit production options in order. Request enough whole batches to cover it, capped by shared capacity, inputs and numeric limits.
5. Reduce the same stock budget for selected production inputs and a shared budget for capacity. Both budgets span all needs and candidates in the request.
6. Report residual quantity and any forecast surplus. Expected output and surplus are never added back as stock or reused for another need in this proposal.

The algorithm is a bounded greedy proposal, not a global optimum or a social allocation policy. Reordering needs or alternatives can change the result. It neither searches an unlisted catalogue nor plans recursive production chains. A complete whole batch can exceed the remaining need; that excess is visible as `surplus`, not hidden production or money creation.

## Read the result

`advisory` is always true. `observed_at` identifies the UTC database snapshot; `period` is the operator's calendar period, not a timezone conversion. The endpoint uses a read-only REPEATABLE READ transaction. All quantities are strings and calculations use exact integers.

Each need line includes:

- `uncovered`: the current starting quantity;
- `stock_selected`: existing stock suggested for resident reservations;
- `expected_from_production`: the part of expected output assigned to this line;
- `residual`: quantity not covered by these suggestions;
- `stock`: lot IDs, quantities and required accounting measure;
- `production`: existing production-plan request bodies, expected contribution and surplus;
- `evaluations`: production assessments at the remaining demand and shared budgets when each option was considered.

The proposal does not change actual uncovered/unmet quantities. A physical residual of zero does not mean provision has happened or that a production report has been verified.

Accounting is reported separately per resident: current available measure, total required for selected existing stock and the shortage. Insufficient money does not remove a physical suggestion or invalidate a need. Future production's accounting norm is not yet declared, so its future cost is deliberately excluded. Each stock suggestion fits the current i64 reservation amount limit; a very large norm can limit a line even when more physical stock exists. Aggregated accounting requirements can exceed i64 and remain exact strings.

## Applying and refreshing advice

The assessment route takes no holds and stores no durable draft. It can become stale immediately. Version 0.7 adds [persisted drafts and atomic application](drafts.md): save the planning request, review the server-generated proposal and explicitly apply its ID. The server revalidates the complete reviewed result under locks before creating all selected holds in one transaction. Individual reservation and production routes remain available for independent actions.
Do not execute production ahead of earlier stock suggestions that consume the same inputs and assume the old advice remains feasible. The listed need order and per-need stock-before-production sequence are part of the proposal. Accounting gaps require a separate explicit allocation decision; the planner never funds accounts. Repeated assessments on unchanged data produce the same selections, but a new observation timestamp.

Migration 0006 adds norms and same-transaction need-source links without editing migrations 0001-0005. Prior needs remain manual/unattributed. Norm creation/application preserve global idempotency, rollback and immutable history. Automatic demographic eligibility, approved norms, fairness optimization and attribution of standalone production forecasts remain future work.

## Active commitments (v0.9)

Policy `listed_need_order_stock_then_committed_then_production_v2` selects available stock first, then subtracts active draft-linked production assigned to the same need before proposing new cycles. `committed_production` shows the total outstanding commitment and `forecast_excess` shows any amount beyond the gap after stock. Neither field changes the need record, adds stock, counts receipt or issues accounting measure. Cancellation/completion removes the commitment; completed output must be explicitly selected and delivered.
