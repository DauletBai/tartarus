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
| A moderate energy reserve | Shared physical stocks and standby capacity, sized for a declared disruption horizon and replenished after use |
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

Explicit needs norm revisions, ordered advisory planning and reviewable drafts with atomic application are implemented. Plans coordinate promises across needs and provisioning units; only later completion and receipt events provide feedback about actual results. Durable coordination now recalculates explicitly monitored scopes after committed commands and records advisory AI commentary separately. Approved needs norms, logistics, global optimization, global attribution of standalone forecasts, independent verification and the full circulation policy remain unfinished. The current modules are parts of a future coordination system, not a complete model of an organism or economy.

The systematic [organism audit](organism-audit.md) records missing responsibilities and test boundaries. See [synthetic exercises](exercises.md) for live monitoring and adversarial scenarios.

## Research candidate: common-entitlement circulation

On 7 November 2026 an offline example explicitly closes a bounded production and
consumption network. The shared productive units' accounted residual is assigned
to unconditional provision through one treasury; personal daily provision is fully
spent in the model. Timing reserves belong to the common system. This is a declared
ownership/accounting assumption, not an automatically applied levy or a finding
about privately owned firms. The live operator may exercise existing transfers on
synthetic data; no new automatic policy or issuance authority is enabled. See
[closed circulation](computation-and-ai-costs.md#closed-circulation-candidate--7-november-2026)
for exact bounds, counterexamples and missing physical links.

The [coupled physical study](physical-circulation.md) now includes finite deposits,
energy, transport, equipment, construction and qualification lead times. Its
synthetic physical/payment checks narrow that research gap; they do not resolve
calibration, individual choice, upstream completeness or live deployment.


## Bounded reserves, not unlimited accumulation

The author's 7 November direction adds the analogy of a modest body-energy reserve.
The engineering interpretation is a **shared physical buffer**: it is accumulated
from actual output, kept within constructed storage capacity, drawn during a
supply failure, and replenished after recovery. This is not a biological claim that
all events are predictable, nor a personal deposit or permission to trade money.

A usable reserve requires both stored material and a working conversion/delivery
path. Fuel without generators does not supply electricity. A promised warehouse
cannot hold stock before it is built. Storage losses and a protected restart stock
must be represented explicitly. More reserve can consume material, labour and
working measure; its adequacy is evaluated against specified service and terminal
reserve constraints, rather than inferred from its size.

The [reserve lifecycle experiment](physical-circulation.md#constructed-reserve-and-recovery-experiment)
compares no new reserve and 7/30/60-period fuel targets with constructed generators
and storage, exact losses and repeated failures. It retains start-up vulnerability
and events exceeding the design horizon. The live service gains no autonomous
issuance, purchase or resource-allocation authority from this calculation.

A reserve must also remain usable: food has an age, transport can lose units,
and standby equipment needs material and labour for maintenance. The offline
[lifecycle submodel](physical-circulation.md#dated-food-transport-loss-and-standby-maintenance)
records these losses and their service consequences explicitly. Neither a large
stock figure nor installed backup capacity alone establishes continuity of care.

The integrated graph now represents that upkeep inside production itself:
constructed backup capacity expires operationally until serviced; service consumes
metal, energy and shared work capacity. Food circulation carries original lot
expiry through transport. Actual losses affect production decisions and monetary
schedules. The three-day meal-buffer candidate improves service while leaving
substantial food expiry, so care and continuity still require better coordination,
not just an enlarged reserve or a larger accounting balance.


## Selected food-reserve form

The author's refined requirement is a preserved-food inventory: dried, sealed or
otherwise appropriately processed product, with no fresh perishable warehouse
stock in the target scenario. Historical fresh-food traces remain only as
comparators. The reserve covers both disruption and restart/delivery lead time;
FEFO rotation connects it to ordinary consumption. Two storage constructions,
processing energy and packaging material are explicit in the offline graph.
The model distinguishes stock duration from nutrient adequacy and local access.
See [preserved-food design](physical-circulation.md#preserved-food-reserves-the-target-architecture)
for tested outcomes, source qualifications and uncalibrated costs.

## Automation and continuity of provision

The new seasonal automation annex treats automated cells as finite capabilities
that require installation, qualified available crews, energy and upkeep material.
A common software/control failure can disable those routes together, so manual
fallback is tested with the same needs and existing capacities. Provision is not
reduced when less human work is required. Potential released time is a result to
be evaluated, not a claim that people are expendable or should lose entitlement.
The analogy of reserve capacity includes both stocked resources and an operable
route to people; neither money nor an AI forecast substitutes for that route.

## Reproduction of shared capabilities

The upstream annex treats a usable power unit, factory or store as a capability
with a finite calendar life. Replacement consumes real material, energy, work and
time. A retired building cannot hold food simply because its quantity was left in
a plan. Finite natural deposits are depleted explicitly; circulation of measure
does not replenish them. Human needs stay unchanged through all tested failures.
Care includes preserving delivery routes and renewing the capacities that sustain
them, alongside the moderate physical buffers already modelled.

Continuity also requires memory: a recalculation must retain resources already
consumed in unfinished work and the true ages of shared equipment. The rolling
annex fixes this history instead of restarting from fresh endowments. Material
recovery can reduce new extraction but consumes capacity and energy and leaves
explicit waste; it is not a source of unaccounted physical growth.

Dated memory preserves the actual origin and age of a resource as it moves through
stages. A renewed plan cannot make old food young, and a future harvest is not
present stock. The offline checkpoint model retains these distinctions; real
observations still require independent custody and review before service use.


## Population and needs feedback

Population planning now keeps age transitions, births, deaths and migration in an
explicit monthly balance. Need is evaluated for each declared age profile before
aggregation; a missing coefficient cannot be covered by another group's surplus.
The role of these profiles is to identify provision, not to rank people by work,
profitability or usefulness. Unknown care and nutrient requirements remain open.
Three population paths make changing assumptions visible; evidence status and
fixture scope are recorded in [input assumptions](input-assumptions.md). The older upstream plan fails the new food-component need gate,
which is feedback requiring a revised physical plan rather than reduced entitlement.


Regional and life-stage profiles now preserve the same person-day total while
identifying different needs. Pregnancy together with assisted care is one joint
profile, not two separate people. Regional shortages remain local signals even if
another region has surplus. Neither basic provision nor care depends on employment,
productivity or a person's ranking. No private health records are introduced.
