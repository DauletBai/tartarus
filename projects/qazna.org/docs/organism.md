# Qazna as an organism-inspired provisioning system

The author's direction of 2 November 2026 is to use the living human organism, from cells to organs and their coordination, as an architectural guide. The model aims to sustain human needs and reproduce shared capabilities. Increasing financial profit is not a default objective.

## Conceptual foundation and testable claims

*Tartar and I* supplies the author's connected argument about equal measure,
human dignity and circulation. Read that context to understand why Qazna starts
from needs and shared capabilities rather than profit, interest-bearing debt or
employment-conditioned survival. The project is not an implementation of those
financial incentives with new terminology.

The engineering task is to test whether the proposed rules sustain provision,
choice and renewal under finite resources and disruption. Comparative experiments
must fix needs, endowments, horizon and observation quality, state each policy,
and publish unmet need, reserve recovery, resource losses and accounting results,
including failures. A successful bounded case is not proof that every historical
crisis has one cause or that Qazna prevents every crisis. Agreement with the book
is not required to reproduce the equations or identify a defect. Reading it aids
interpretation; independent technical criticism strengthens the research.

## Five-year circulation and access

The justice criterion is whether each group can obtain what it needs through a change in employment. The continuity criterion is whether the same units can pass through the productive system again in time for the next period. These criteria require different measurements.

**Access at a fixed level of output.** Let $N_g$ be the group's daily need at a fixed unit measure. In the comparison, the working-age group initially receives $N_g$ in earnings. If a fraction $h$ of paid hours is lost at a constant hourly rate and a fraction $r$ of the lost income is replaced, its purchasing gap is $N_g h(1-r)$. Children and people unable to work retain their initial entitlements in every case. The unpaid portion accrues elsewhere in the economy. Under the need rule the group receives $N_g$ independently of hours. For the opening needs of 4,000, 3,000 and 3,000 units, the table shows the working-age group's gap.

| Paid hours lost | No replacement | Half of lost income replaced | Full replacement | Need-based entitlement |
| --- | ---: | ---: | ---: | ---: |
| 0% | 0 | 0 | 0 | 0 |
| 50% | 1,500 | 750 | 0 | 0 |
| 100% | 3,000 | 1,500 | 0 | 0 |

This is a one-day distribution comparison. There is no assumed change in physical output, prices or total income. Full income replacement and need-based provision yield the same coverage here. A real comparison would measure eligibility gaps, administrative burden and access to suitable goods. The trial does not measure motives, predict employment or establish that all existing systems leave wages uninsured.

**Minimum stock for a growing circuit.** Write $D_t$ for the total daily allocation. Weights are 40%, 30% and 30%. Across five 365-day years, $D_t$ takes the successive values 10,000, 10,500, 11,000, 11,500 and 12,000. Each day starts with allocations and closes with the settlements due that day. If common production proceeds clear $d$ days after allocation, then just before clearance the treasury must bridge today and up to $d$ prior allocations. With zero allocations before day zero, the exact opening requirement is

$$
B_{min}(d)=\max_t \sum_{k=0}^{d}D_{t-k}.
$$

The five-year household allocation volume is $365(10{,}000+10{,}500+11{,}000+11{,}500+12{,}000)=20{,}075{,}000$ units. For delays of 0, 1, 2 and 5 days the opening balances are respectively 12,000, 24,000, 36,000 and 72,000. All 62,050 transfers settle in each replay; after the last in-transit units clear, the closing treasury equals its opening balance. Reducing each opening by one unit produces its first missed transfer on day 1,461, 1,462, 1,463 and 1,466 respectively, counting the first day as day one. The published JSON uses zero-based days.

The bound assumes the full common share re-enters circulation, a fixed order of transactions and a known delay. A longer disruption or permanently retained proceeds change the bound. A buffer covers a timing gap. Physical reserves and productive renewal cover different risks. The proposed rule for issuing measure against certified growth is outside this calculation.

To reproduce both stages, run `cargo run --locked --example circulation_study -- /tmp/qazna-circulation.json` from the project directory. Compare `access_sensitivity` and `growth_and_delay` with the [published result](../research/circulation-model.json). The executable checks the rolling sum against every account's transaction-prefix bound and replays each schedule with both the exact opening and one unit less.

## What an early adopter could test

In the author's proposal, access to necessities follows a person's needs across their lifetime, including periods when they cannot work. Tenge measures provision at declared norms. A common treasury circulates existing measure; the wider proposal ties additional issuance to verified growth in real capacity. The working prototype tests finite allocations and can reject an unaffordable request. Establishing a credible measure of capacity growth and the institution empowered to certify it remains a separate research problem.

Three different failures can look identical to a household that went without a meal. A loss of paid hours can put a meal out of reach. A provider can have the stock and still miss a payment deadline. A district can have enough money and too little suitable food. The first calls for examining entitlement and household income, the second for checking settlement and liquidity, the third for changing production or distribution. Transferring funds alone resolves only some of these cases. A credible evaluation also counts existing public provision and income support in the comparison.

The book discusses the Chicago Fed's *Modern Money Mechanics*, a historical workbook on bank reserves and deposit expansion. A country-level comparison needs the credit institutions, transfer programmes and rules actually in force in the country and period being studied. The Bank of England's account explains how lending creates deposits and how regulation, monetary policy and borrowers constrain the process. [Chicago Fed workbook](https://fraser.stlouisfed.org/title/modern-money-mechanics-9810/modern-money-mechanics-717249) · [Bank of England](https://www.bankofengland.co.uk/quarterly-bulletin/2014/q1/money-creation-in-the-modern-economy).

The strategic opportunity for an early adopter is to establish a publicly inspectable method for sharing productivity gains. A region that publishes its rules, costs, coverage gaps and failures gives other regions something they can test. The stronger claim that this would confer international economic leadership requires comparative evidence over time. [Local comparison design](pilot-charter.md#comparison-and-decision).

## What the organism analogy asks us to test

Imagine a tissue could claim oxygen in proportion to its existing stock, and
could increase that claim simply by accumulating more. An injured tissue with
a small stock could lose supply just as its need increased. This thought
experiment gives a precise objection to treating accumulated purchasing power
as the signal for allocating necessities. Aristotle already distinguished
provision for the household, which has a purpose and a limit, from acquisition
pursued without limit. [Aristotle, Politics, Book I](https://classics.mit.edu/Aristotle/politics.1.one.html).

There is a documented economic mechanism by which an initial disturbance can
grow. Rising collateral values can expand borrowing and asset purchases. When
prices fall, falling collateral values tighten access to credit and can deepen
the contraction. The relevant question for Qazna is whether households can
still receive essential goods through such a disruption. A drought can also
reduce the physical harvest under any accounting rule. Economic exclusion can
contribute to violent conflict alongside struggles over political power,
security and justice. A provisioning trial can measure exposure to exclusion;
a claim about preventing wars requires different evidence.
[BIS study of the financial cycle](https://www.bis.org/publ/qtrpdf/r_qt1812g.htm)
and [UN–World Bank study of conflict prevention](https://www.worldbank.org/en/topic/fragilityconflictviolence/publication/pathways-for-peace-inclusive-approaches-to-preventing-violent-conflict).

The biological comparison suggests two time scales. Development and renewal
rely on inherited regulatory mechanisms. Their operation responds to conditions
as they change. At the onset of voluntary exercise, central signals and local
vascular responses help adjust blood supply. For Qazna the enduring commitment
is provision across the life course. Forecasts for care, food and capacity
change with evidence and voluntary choices. A child's need for care has the
same standing whether an illness is inherited, environmental or of unknown
origin. Cystic fibrosis, for example, can affect a newborn through inherited
variants. Dignity and equal standing are ethical decisions people make for
one another. An individual must be able to challenge the system's account of
their needs. [NHGRI on gene regulation](https://www.genome.gov/genetics-glossary/Gene-Regulation),
[human exercise study](https://pubmed.ncbi.nlm.nih.gov/27103652/) and
[MedlinePlus Genetics on cystic fibrosis](https://medlineplus.gov/genetics/condition/cystic-fibrosis/).

### What a four-month warning changes

Consider the power modules in the existing grain plan. The 72 monthly
observations span January 2026 through December 2031, including the predecessor
year. The opening 20 modules retire in July 2027. A new module requires three
months of construction and becomes usable the following month. Each module
provides 2,000,000 generation activity units per month over 24 months.
For the published generation demand, six modules cover the peak in the first
twelve months after retirement. The three schedules below all start 40 modules
and use the same opening equipment and generation demand.

| Start policy | Months short of generation capacity | Sum of uncovered generation activities | Active modules in December 2031 |
| --- | ---: | ---: | ---: |
| Inherited timetable | 12 | 115,417,239 | 20 |
| Start six when each six-module tranche is needed | 4 | 37,520,290 | 14 |
| Start those six four months before each need | 0 | 0 | 14 |

To reproduce, set month zero to January 2026 and retain the published schedule
for generation process 31, repeating the first 12 periods before the original
60. The inherited construction starts (month, quantity) are (26, 20) and
(50, 20). The second schedule advances six units from each to months 18 and
42. The third advances those units to months 14 and 38. A start in month
`s` contributes from `s + 4` through `s + 27`, inclusive. In each month
subtract active capacity from scheduled generation and retain the positive
part. The total is measured in process activities, not kWh or people served.
[Physical inputs, capacities and delays](../fixtures/extended-scenario.json)
and [published baseline shortfalls](../research/field-rotation.json).

The early start in March 2027 requires 6,000,000 energy ticks, 60,000 material
ticks, 6,000 work ticks and six units of shared factory capacity that month.
Moving starts preserves total module count but changes when those resources
are needed and when the equipment will next retire. The 14 active modules
at the end of the revised schedules also matter for the following years.
This comparison checks one capacity constraint. Plant construction, grain
storage, all competing work and treasury liquidity still need a joint replay.

**Design rule.** Keep a dated renewal obligation for each essential capability.
At review time, look ahead at least through preparation, construction and
commissioning. Compare surviving capacity and committed completions with
expected need and the chosen physical reserve. When coverage falls short,
propose the latest feasible start and expose any competing use of its inputs.
Carry the ages, outstanding work and next retirement dates across every
five-year boundary. Recheck after new observations and let an operator review
the physical and accounting consequences before application.

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

## Care, learning and replenishing capabilities

Care and education now have non-storable regional service balances. Need derives
from population exposure, regardless of employment or productivity. Capacity
comes from available people, rooms and equipment; preparation of new specialists
uses current mentors, while construction uses shared builders and physical inputs.
A shortage preserves the need and explains the capacity gap. Aggregate slots
must not be interpreted as compulsory work assignment or a person's worth.

## Adapting without losing memory

A temporary reduction in available capability can now trigger a verified proposal
to defer unstarted development. Care remains unchanged while preparation uses
available mentor time. Work already begun stays in the commitment ledger. If no
permitted schedule can sustain provision, the deficit remains explicit. This is
feedback around shared physical reserves and capacity, not permission to redefine
people's needs or compel extra work.


The feedback journal now remembers a declared disruption even when available
capabilities cannot yet satisfy the need. A reviewed repair and its circulation
can be associated with that declaration; the association does not assert that
care was delivered or the disruption ended. Recorded need, physical feasibility,
accounting feasibility and observed recovery remain separate responsibilities.


Several signals can describe one disruption. Combining them now preserves their
separate origins; contradictory values require an explicit correction. A correction
can change our current account of events without erasing the intervention already
reviewed. The extended-absence case shows a delayed training course missing future
care needs: adaptation needs available capabilities and preparation time, not only
circulation of accounting measure. Human need is never reduced to fit the plan.

## Emergency use of shared spare capacity

A protected reserve is useful only if a reviewed emergency can draw on it. The
new release record keeps the original floor, identifies the triggering incident,
limits each pool/period draw, and sets a recovery deadline. Care and education
needs remain unchanged. No record creates staff time, equipment or money.
Recovery here means restoring protected free capacity, not repaying a loan or
storing unused human work. An unresolved shortage remains a failed plan.
