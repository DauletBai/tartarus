# Planning for ten billion people with Kazakhstan as the national demonstrator

Where people live determines where provision must reach them. In Kazakhstan, the October 2026 statistics account for 20,633,529 people across 17 regions and three separately administered cities. Their 228 district and city-administration rows add back to each parent. The country is counted once in the world total. The October count includes 13,254,158 urban and 7,379,371 rural residents. The change since January reconciles exactly to 128,303 natural increase and 5,404 net migration. Both components are available for each region. [BNS October demographic balance](https://stat.gov.kz/api/iblock/element/348013/file/ru/) · [BNS territorial population, table 2](https://stat.gov.kz/api/iblock/element/341323/file/ru/)

## From the world to a district

The UN World Population Prospects 2024 medium variant crosses ten billion between July 2060 (9,989,232,292) and July 2061 (10,013,916,213). The timing depends on the projection variant. The study uses all 237 UN country/area rows in those two years. For target T, world totals L and H and local populations a and b, the population weight is (H−T)a + (T−L)b. Dividing by the sum of the weights accounts for independent rounding of published country and world totals. The Rust proportional allocator rounds by largest remainder and breaks ties by UN location ID. The resulting 237 counts sum to exactly 10,000,000,000. [UN World Population Prospects](https://population.un.org/wpp/)

Kazakhstan receives 28,558,020 people in that scenario. For the regional and district illustration only, keep their October 2026 settlement shares fixed and allocate within each parent. Area names in the district release serve as edition-specific keys under the region. The public data account for residents through counts in their place of residence. They contain no personal addresses or movement histories. A model of future internal migration could change the local figures considerably.

| Location | Recorded residents, October 2026 | Residents in the ten-billion scenario | Annual gross grain requirement at 100 kg each |
| --- | ---: | ---: | ---: |
| Kazakhstan | 20,633,529 | 28,558,020 | 2,855,802 t |
| Rudny city administration, Kostanay Region | 122,453 | 169,482 | 16,948.2 t |

The last column uses a changeable scenario input for grain and pulses before processing. Nutrient targets, feed, seed, household choice and losses would need separate accounts. The current district requirement for Rudny is 12,245.3 t under the same assumption. Crop and climate observations retain their published geographic scale. Assigning a regional yield to each district would introduce precision the data cannot support.

## What the sources measure

| Layer | Coverage of this edition | Use in planning |
| --- | --- | --- |
| UN DESA WPP | 237 countries and areas, July medium-variant population for 2026 and both crossing years | Distinct geographic units, dates and scenario weights |
| FAO via World Bank WDI | 2024 cereal output in 177 areas, 2023 arable hectares in 205, long-term precipitation in 180 and internal renewable freshwater in 181 (2022 entries) | Published cereal output divided by scenario residents illustrates the effect of population change at constant output. Arable land and annual renewable water give context. Basin inflow, irrigation withdrawals, environmental flows and storage are separate quantities. |
| IRENA via UN SDG | Installed renewable capacity in W/person in 223 areas, year-end 2023 | Nameplate generating capacity. Weather-dependent availability, grid constraints and load require time series. |
| USGS Mineral Commodity Summaries 2026 | Copper mine output, refinery output (2025 estimates) and reserves for 17 individually named countries, with separate iron ore and chromite figures for Kazakhstan | Geological reserves are stocks. Production is a flow. Some countries refine imported material. “Other countries” cannot be assigned to unnamed places. |
| BNS regional yearbook | Grain and pulses harvest, 2020–24. Sown hectares and 2024 output of potatoes, vegetables, meat, milk, eggs, cement, electricity and selected pumps | Physical output by area and year. Census geography follows each source edition. Confidential, rounded-negligible and absent figures are unknown. The global cereal indicator excludes pulses. |
| Kazhydromet | 36 monthly temperature and precipitation anomalies, December 2022–November 2025, for 17 regions and the country | Compare April–September precipitation totals and the arithmetic mean of the six monthly temperature anomalies with harvest from the same year. Reference period 1961–90. Mangystau's precipitation series is represented by Fort Shevchenko station. The three separate cities have no city-specific observations in these regional tables. |
| KEGOC 2025 annual report | Installed 26,807 MW and available 20,081 MW on 1 January 2026 under the report's method | Distinguish available power in MW from actual annual generation in kWh. |

Every input has an edition or table, units, original publisher URL and a SHA-256 digest in the [17-source register](../fixtures/geography-sources.json). The [observation snapshot](../fixtures/geography-baseline.json) retains missing values. National aggregates coexist with their children for reconciliation. Adding both levels would count provision twice. The WDI and UN SDG rows are joined to the WPP list by ISO3 and UN statistical code respectively. Upstream aggregates are excluded.

## What the national demonstration finds

* Across regions with a published 2024 harvest, the sum of local shortfalls against the October 2026 population is 598,857.7 t at 100 kg/person/year. For each region the calculation is max(0, residents × 100 − harvest in kg). Even that simple balance requires transfers, opening stocks or outside supply. Mangystau's harvest is unreported in the yearbook table, so its 827,102 residents remain visible outside this subtotal. The national 25,204,800 t harvest alone cannot determine whether food reaches any household.
* In Akmola the April–September precipitation anomaly was +6.4 mm in 2023 and +117.1 mm in 2024. Gross grain and pulse harvest rose from 3,215,500 to 6,036,100 t. These two observations establish co-occurrence. Sown area, soils, heat, cultivar and inputs can also affect output. The 2025 weather record is available beside an unknown harvest from this particular source edition.
* Holding out one year tests how much error a simple forecast can hide. Use the median of the three preceding national harvests. The absolute errors were 720,700 t in 2023 and 8,108,200 t in 2024. The latter is 32.2% of the actual harvest. A deterministic food plan based on a recent central estimate alone would substantially miss the outcome. The same test is available by region when all four observations exist.
* In Kazakhstan USGS reports crude iron ore reserves of 3.8 billion tonnes containing 1.5 billion tonnes of iron, and estimated usable ore production of 35 million tonnes in 2025. For copper, 20 million tonnes in reserves, 710,000 t mined and 500,000 t refined describe three different quantities. Annual electricity generation in the BNS 2024 table was 118.67 TWh. These observations provide checks on a proposed production plan. They do not establish mine, smelter, grid or transport capacity for a particular month.

### How much grain would have to reach other regions?

With the October 2026 population and an annual gross requirement of 100 kg per resident, the published 2024 regional harvests add up to 25,204,900 t. The independently printed national total is 25,204,800 t. Table 4.54 reports in steps of 100 t. Its precision can explain the 100 t difference. Subtracting the two printed totals would assign a negative harvest to Mangystau.

For each positive observation, enclose the harvest within 50 t of the printed value. A source dash gives zero. An unreported quantity can range from zero to the national upper limit less the sum of known regional lower limits. This leaves at most 850 t for the unreported 2024 harvest. Calculate the local gap as the positive part of need less harvest. Sum lower and upper bounds over the regions, allowing the unreported group to use the whole residual when bounding the minimum. Also apply the national supply ceiling to the lower bound. The resulting bounds are conservative because rounding errors can offset each other.

Across all 20 territories the 2024 comparison gives **680,567.9 to 681,717.9 t** of local gross need beyond local production. This is the amount that transfers, stocks or other supply would have to cover in this scenario. It measures neither actual shipments nor hunger. The report repeats the calculation for every harvest year in the snapshot with the same settlement counts and yearbook geography. A later population applied to an earlier harvest is a sensitivity test.

### Food and industrial output at the larger population

As a separate sensitivity test, hold gross output per resident fixed: multiply each reported 2024 quantity by scenario residents divided by October 2026 residents and round up to the next whole source unit. This isolates the scale of population growth. Actual production targets would also depend on consumption, imports, exports, losses, efficiency and the location of plants. The national benchmark uses national output; region-specific scenarios use local output and settlement shares. A confidential regional output remains unknown even when the national total is published.

| Annual quantity in Kazakhstan | Reported 2024 | Scenario at 28,558,020 people |
| --- | ---: | ---: |
| Milk, tonnes | 3,628,500 | 5,022,058 |
| Meat in carcass weight, tonnes | 1,168,700 | 1,617,550 |
| Vegetables, tonnes | 3,570,100 | 4,941,229 |
| Portland cement excluding white cement, tonnes | 12,271,500 | 16,984,480 |
| Electricity generation, TWh | 118.67 | 164.25 |

## Reproduce and extend

Run from the project directory. The Rust demonstration reads the checked-in snapshot and works offline. An optional second argument sets annual gross grain kg per resident. The published report uses 100.

```sh
cargo run --locked --example geography_study -- /tmp/geography-results.json
cmp /tmp/geography-results.json research/geography-results.json
cargo test --locked --example geography_study
```

To reconstruct the observation snapshot independently, install Python 3.11+ and Poppler's pdftotext. The command fetches missing sources into a local cache and checks each digest. On a publisher revision, keep the previous cached edition and review the changed tables and units before updating the register. PDF row labels, XLSX table cells and checks on both geographic levels validate the extraction. The KEGOC national capacity pair and USGS chromite pair are transcribed from the pinned tables.

```sh
python3 scripts/import_geography.py /tmp/qazna-sources /tmp/geography-baseline.json --download
cmp /tmp/geography-baseline.json fixtures/geography-baseline.json
```

**Next calibration.** Link edition-specific districts to official, dated KATO codes and boundaries. For one complete supply chain, join crop areas, field or station weather, soil and water by basin, production sites, processing yields, rail or road capacity and stocks on the same calendar. Backtest yields on withheld years and compare against the median benchmark. Reconcile deliveries to district-level requirements, then connect the verified physical quantities to the existing national circulation replay. The worldwide allocation measures demographic scale. It does not benchmark ten billion accounts or supply a weather forecast for 2061.
