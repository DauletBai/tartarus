# Production completion, losses and provenance

Version 0.5.0 closes a previously committed production plan with an explicit operator report. One atomic transaction consumes its input holds, records capacity use, records losses and creates usable output. Cancellation and completion are mutually exclusive. This is full-cycle execution; partial execution, unused-input returns and incremental completion are not implemented.

## Confirm a completed cycle

`POST /v1/production/plans/{id}/complete` requires the operator token and global `Idempotency-Key`:

```json
{
  "output": {
    "id": "00000000-0000-0000-0000-000000000011",
    "producer": "00000000-0000-0000-0000-000000000004",
    "quantity": "1",
    "unit_measure": "20"
  },
  "losses": [
    {"slot": "meal", "quantity": "1"}
  ],
  "basis": "Observed packing result, report EXP-001",
  "shortfall_basis": "One of the two planned portions was damaged"
}
```

The example assumes a committed plan for two packing batches, each requiring one meal and producing one packed meal. The output lot UUID must be new. Its producer account must belong directly to the recipe's executing unit. Output category and physical unit come from the immutable recipe; they cannot be changed in this request. The operator supplies the positive accounting norm for the new lot; it is not automatically derived from input norms, money or market prices.

Use `output: null` when there is no usable output. No empty lot is created. Actual output may range from zero to planned output; over-yield is rejected rather than silently accepted as extra stock. A nonempty `shortfall_basis` is mandatory when actual output is below the plan. `basis` is always required and supports the whole report, including input loss declarations. These fields are plain descriptions or evidence references, not uploaded documents or signatures.

`losses` lists unique input slots with positive integer quantities. Omit a slot to record zero loss. Loss cannot exceed that slot's reserved quantity. Every reserved input is withdrawn in full:

- `withdrawn` is the original input reservation;
- `loss` is the declared unusable portion;
- `used = withdrawn - loss` is the portion recorded as process use.

These are two classifications of one withdrawal, not two stock debits. Output shortfall is measured in output units; input losses remain in their respective input units. There is no equation adding grams, milliliters and meals. The recipe does not yet encode mass balance, chemical transformations or quality verification. A full withdrawal with zero output can therefore be recorded with an explicit reason, without claiming that all input mass became output loss.

## State and capacity

Completion changes the plan to `completed`. Input quantities move from reserved to consumed, so completing work does not release those quantities back into stock. Planned capacity moves from reserved to consumed; `available = quantity - reserved - consumed`. The full planned capacity is charged even when output is lower. This explicit full-cycle assumption must not be used for a job that actually needs partial closure; that workflow is pending.

No postings are created and no money is issued. Finished output is available as a normal resource for another production plan or for resident provisioning. It does not satisfy a resident's need until a linked resident reservation is settled.

The original plan-creation receipt remains a historical reserved response on replay. GET the plan for current state. Completion replays return the same receipt and output identifier without consuming inputs twice. A different completion key after closure gets `production_closed`; changed parameters with the original key get `idempotency_conflict`.

## Read reports and follow provenance

`GET /v1/production/plans/{id}/completion` returns the plan and recipe IDs, output lot or null, planned and actual output, output shortfall, output category/unit, capacity consumed, bases and input breakdown. An uncompleted or unknown plan returns `production_not_found`.

`GET /v1/resources/{id}/provenance` returns:

- `origin = production`, the immutable completion report and input lot IDs for produced stock;
- `origin = declared`, the declaration basis and null completion for operator-declared or legacy stock.

The response contains one production step. Follow each input's provenance endpoint to traverse a chain. Initial declarations are not relabeled as independently verified sources. Both provenance reads and their nested report use a consistent database snapshot. Public anonymous access is not enabled.

## Atomicity and migration

Migration 0005 leaves migrations 0001-0004 unchanged and preserves active plans. Completion requires a plan committed in an earlier transaction. Output must be created in the completion transaction, be unique to that completion and match recipe quantity, dimensions and executing unit. An old lot cannot be attributed retrospectively. These rules also prevent a completion from treating its own output as an existing input.

The service locks command key, plan, capacity, output producer account and then input lots in UUID order. Terminal database triggers serialize completion versus cancellation. Deferred constraints check output linkage, losses, capacity and stock totals; histories reject changes and deletion. Loss records must be created with their completion, not added later. A failed output insert, report or loss record rolls back all changes and leaves the command key reusable.

The report records what the operator asserts happened. It does not independently verify measurements, nutrition, equipment, real growth or authority to issue money. Reversals, corrections, multiple output products, partial completion, over-yield approval, evidence signatures and independent audit roles are future work.
