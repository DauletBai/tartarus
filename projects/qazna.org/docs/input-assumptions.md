# Input assumptions and evidence status

This is the common reference for the project's calculation inputs. Explanatory
chapters describe mechanisms and results without repeating the same qualification
in every paragraph. Machine-readable provenance, unknown values and failure cases
remain part of the calculation contract.

## Reading an input

The registry distinguishes illustrative coefficients, source-attributed values and
unknown values. Each coefficient retains its unit, region, reference period,
version, recording date and basis. Attribution identifies a source; independent
review establishes whether it supports the value and scope. An exact calculation
can be correct for its inputs without those inputs accurately representing a place
or population. Real trials measure and revise coefficients; they also retain
measurement error and uncertainty rather than automatically making every input exact.

Existing examples remain reproducible snapshots. New evidence currently creates a
new reviewed scenario, not a silent rewrite of the old one. No illustrative data
are relabelled as observed data. Numerical ranges describe declared alternatives,
not statistical confidence bounds unless a separately documented method supports
that interpretation.

## Regional and life-stage profiles

The maintained example partitions the 2027–2031 age-cohort exposure into two
regions (40% north, 60% south) and seven joint profiles: general, assisted care,
three pregnancy stages, postpartum, and second-stage pregnancy with assisted care.
The demographic reference path supplies each month's age-group person-days.
Regional shares and profile schedules are scenario inputs, not regional census or
pregnancy observations. They are not a fertility, diagnosis or pregnancy-outcome
model, and no personal medical records are collected.

Assisted exposure uses one tenth of each regional age-group exposure. Each of the
three pregnancy stages and postpartum uses one hundredth of adult exposure;
the combined pregnancy/assistance profile uses one five-hundredth. Remaining
exposure belongs to the general profile. These are disjoint joint categories:
the combined category is not added again as two separate people. In real input
preparation, intersecting conditions must likewise form disjoint cells; validating
totals alone cannot establish that source classifications are accurate.

Care-time coefficients represent a planning workload: 60 minutes per assisted
person-day, 62 for the combined profile, 2 for each pregnancy-stage profile and
5 for postpartum; general profiles use 10 minutes for the youngest group,
15 for the oldest and zero for the other groups. Water-component coefficients
are 2 litres in the north and 3 in the south, with one additional litre for the
pregnancy profiles. These values exercise scheduling and regional reconciliation;
they are not prescriptions or validated clinical staffing requirements. Gross
food components and education-capacity rules inherit the age-profile coefficients.
Protein and vitamin D remain unknown. Care needs do not depend on employment.

The WHO [antenatal recommendations](https://www.who.int/publications-detail-redirect/9789241549912)
and [care overview](https://www.who.int/activities/promoting-healthy-pregnancy/promoting-healthy-pregnancy)
provide a source for subsequent clinical service modelling, including eight
scheduled contacts during pregnancy. They do not supply the per-day workload
coefficients above. Contact timing, appointment duration, emergencies, gestational
age, outcomes, local protocols and professional review require separate data.
The WHO [disability data guidance](https://www.who.int/activities/collection-of-data-on-disability)
also motivates disaggregation; the current assisted-care flag does not infer a
clinical diagnosis or a person's capacity, worth or eligibility.

Exposure follows the population engine's opening-month convention. Births and
arrivals contribute from the next period; within-month clinical events and immediate
newborn care are not represented yet. Pregnancy labels do not generate births or
validate transitions, and the age grouping does not infer sex-specific fertility.
Regional partitions do not model migration, routes or construction of local care
facilities. Those require new balances, not assumptions hidden in profile names.

The accepted runtime example retains the preceding finite-opening-stock water
fixture solely to exercise profile binding, review and recovery. It is not a
five-year water-storage proposal. The resulting regional delivery schedule must
sum exactly to physical deliveries, but it is not independent evidence of actual
delivery or local transport capacity. The full model rejects any mapped local
deficit, even if aggregate provision balances. Unmapped needs remain visible.

## Revisions still needed

Calibration work must connect verified population/health/service observations,
consent and aggregation rules, physical resource and workforce coefficients,
regional routes, uncertainty and independent reproducibility. A source URL, model
output or successfully funded plan cannot substitute for those observations.
These evidence tasks are distinct from the implemented arithmetic and software tests.
