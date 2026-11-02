# Qazna as an organism-inspired provisioning system

The author's direction of 2 November 2026 is to use the living human organism, from cells to organs and their coordination, as an architectural guide. The model aims to sustain human needs and reproduce shared capabilities. Increasing financial profit is not a default objective.

## Mapping the analogy

| Biological analogy | Qazna concept |
| --- | --- |
| Cellular level | Smallest recorded need, resource unit and provisioning event |
| Tissues and organs | Provisioning units with functions, capacities, inputs and outputs |
| Exchange and transport | Physical resource movement and associated accounting circulation |
| Feedback | Unmet needs, stocks, delays, reservations, losses, wear and constraint violations |
| Maintaining stability | Matching provisioning plans to capacities, resources and reserves |
| Growth and recovery | Verified changes in real capabilities; separately, a decision on permissible changes to monetary measure |

This is a design analogy, not proof of the economic model or a claim that social decisions follow from biology. A person remains an autonomous participant with needs, choice and privacy, not an obedient or disposable cell.

## Implementation principles

1. Start with the need: what is required, for whom, how much and for which period. Then identify responsible units, capacity and the provisioning route.
2. Keep physical dimensions separate from accounting measure. Do not sum water, meals and energy without an explicit conversion model.
3. `unit_measure` is a normative accounting measure per resource unit, not a price determined automatically by demand, profit or bidding. The operator supplies an experimental value and records its basis. A centrally approved registry of norms is not implemented.
4. A reservation is promised provision; settlement records receipt. Resource and accounting changes occur atomically. Cancellation releases both holds.
5. Physical shortage and insufficient available accounting measure are distinct signals. Do not silently create money to resolve either.
6. Basic needs are not conditioned on profitability, employment or a person's rating. These criteria are absent from the current module.
7. Preserve history for feedback and audit. Corrections need a stated basis; deleting inconvenient results does not restore system stability.

## Current implementation

Allocations transfer existing treasury measure to a resident with a period, category and basis. The operator supplies the amount; it is not derived automatically from a needs norm. The allocation category does not impose a hidden restriction on spending the balance.

A resource lot belongs to a provisioning account whose technical type is `producer`. This does not assume a private seller, auctions or profit maximization. Resident settlement transfers accounting measure to this account. Returning measure to the treasury requires an explicit economic rule; no automatic return was introduced as an unstated assumption.

Needs, responsibility hierarchies, allocations, resource lots, reservations, receipts, cancellations and coverage reports are implemented. Blocked attempts preserve physical and accounting shortage observations. Declared capacity and immutable recipes support production planning against selected inputs. Completion consumes the whole reserved cycle, records input losses and output shortfall, and creates a traceable output lot when usable output exists. Completion is an operator report, not independent verification, monetary growth authorization or proof of resident receipt.

Explicit needs norm revisions, ordered advisory planning and reviewable drafts with atomic application are implemented. Plans coordinate promises across needs and provisioning units; only later completion and receipt events provide feedback about actual results. Durable coordination now recalculates explicitly monitored scopes after committed commands and records advisory AI commentary separately. Approved needs norms, logistics, global optimization, automatic reconciliation of outstanding forecasts, independent verification and the full circulation policy remain unfinished. The current modules are parts of a future coordination system, not a complete model of an organism or economy.
