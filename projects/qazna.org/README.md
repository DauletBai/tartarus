<p align="center">
  <img src="assets/brand/qazna.svg" width="176" height="176" alt="Qazna — red pulsating shanyrak, continuous circulation">
</p>
<h1 align="center">Qazna</h1>
<p align="center"><strong>Human needs · Shared capabilities · Continuous circulation</strong></p>
<p align="center">
  <a href="Cargo.toml"><img src="assets/brand/badges/version.svg" alt="Crate version 0.11.0"></a>
  <a href="docs/completion-roadmap.md"><img src="assets/brand/badges/stage.svg" alt="Research prototype"></a>
  <a href="LICENSE"><img src="assets/brand/badges/license.svg" alt="Code license AGPL-3.0-or-later"></a>
</p>
## Who gets the gains when machines do more of the work?

A region might produce the same amount of food with fewer paid hours. A family that loses those hours still needs its meals. If its income falls, the food can be available and still be out of reach. Qazna helps a public agency or research institute test how to keep essential provision within reach as work changes.

In *Tartar and I*, Daulet Baimurza uses **chrematistics** for an economy that treats accumulating money as an end in itself. He proposes **Tenge**, a common unit of account, and one treasury through which that measure circulates. His starting point is human dignity. A person's claim to basic provision should survive a change in employment, ability or health. The analogy is circulation in a living body. Record a need, direct available resources to it, observe the outcome and adjust the plan. In human communities, people choose, give consent and can challenge a decision. [Book and economic principles](docs/organism.md).

**Qazna today is a research prototype.** The proposed first engagement is a retrospective comparison for one public service. A municipality and a provider can supply an anonymised record of what was needed, available, budgeted and delivered. An evaluator can replay the existing rules and the proposed treasury rule under the same constraints, and see who would have been served on time. A first partner could publish the method and its results for other regions to reproduce. [Partner brief and evaluation terms](docs/pilot-charter.md).

## Why test this now?

- **Access through wages.** Automation can raise output while changing who earns income and how much. The ILO estimated in 2025 that one in four workers worldwide was in an occupation with some exposure to generative AI. It expects changes in job content to be the most common outcome. Qazna's proposed comparison covers stable earnings, reduced hours and local job displacement. [ILO occupational exposure study](https://www.ilo.org/publications/generative-ai-and-jobs-refined-global-index-occupational-exposure).
- **Credit and financial fragility.** Bank lending creates deposits and repayment extinguishes them. Research on credit and property cycles finds that recessions coinciding with financial contractions are deeper. For an essential service the practical question is whether a financing interruption delays delivery even while staff and stock remain available. [Bank of England on money creation](https://www.bankofengland.co.uk/quarterly-bulletin/2014/q1/money-creation-in-the-modern-economy) · [Financial cycle study](https://www.bis.org/publications/working-paper-380-characterising-financial-cycle-dont-lose-sight-medium-term).
- **Who receives the benefit.** A saving on labour can accrue to equipment owners, lower prices, improve provision or free time for workers. A productivity figure alone leaves the distribution unknown. The proposed study counts suitable provision by deadline, who still waits, and how much extra work providers and households take on. [Evaluation measures](docs/ai-economic-research.md#measurement-contract).

## One decision you can inspect today

A provider declares **10 meals**. In the executable example two inspectors attest the stock. A resident with a zero balance needs **2 meals**. The operator can allocate **4 existing accounting units** from treasury at the case's fixed measure of two units per meal. The provider and resident confirm delivery before settlement. The resident's closing balance is zero. A declaration of 100 unattested meals and an allocation larger than the treasury balance both fail. Those failures call for different decisions: check the physical supply in the first case and review the available funds in the second. [Executable example](scripts/evidence_exercise.py) · [How the evidence is recorded](docs/evidence.md).

The planning engine also checks dated stocks, staff, equipment and payments over multiple years. An operator reviews changes. The calculations run without an AI subscription. Software checks establish reproducibility under the stated inputs. The effects on households, affordability of a national transition and permanent purchasing power of a unit remain research questions. [Five-year calculations](docs/physical-circulation.md) · [Input assumptions](docs/input-assumptions.md).

## Where to start

| Reader | Decision | Starting point |
| --- | --- | --- |
| Public-service director or research commissioner | Is one service worth evaluating, what would it require, and what would we receive? | [Partner brief](docs/pilot-charter.md#first-engagement) |
| Economist or independent evaluator | What would count as an improvement over our current rules, and what result would make us stop? | [Comparison protocol](docs/pilot-charter.md#comparison-and-decision) |
| Service operator or participant representative | Who records a need, confirms delivery and handles a disputed record? | [Pilot safeguards](docs/pilot-charter.md#preconditions-before-real-participation) · [Needs and receipts](docs/needs.md) |
| Technical evaluator | Can our team reproduce a result and inspect a failed case? | [Run the checks](#run-the-checks) · [Architecture](docs/architecture.md) · [Test record](docs/development.md) |

## Book and implementation

The Russian-language essay *Tartar and I* develops the author's argument through language, history, ethics and economics. Chapter 26 introduces the economic model. The book and project are available for independent examination. [Read the book online](https://dauletbai.github.io/tartarus/) · [Print PDF](https://media.githubusercontent.com/media/DauletBai/tartarus/main/book/Tartar_i_Ya_print_6x9.pdf).

## Run the checks

<p align="center">
  <img src="assets/brand/badges/rust.svg" alt="Rust minimum supported version 1.88">
  <img src="assets/brand/badges/edition.svg" alt="Rust edition 2024">
  <img src="assets/brand/badges/tokio.svg" alt="Tokio locked dependency">
  <img src="assets/brand/badges/axum.svg" alt="Axum locked dependency">
</p>
<p align="center">
  <img src="assets/brand/badges/sqlx.svg" alt="SQLx locked dependency">
  <img src="assets/brand/badges/postgresql.svg" alt="PostgreSQL 16 tested">
  <img src="assets/brand/badges/python.svg" alt="Python 3.11 or later for tooling">
</p>

Requirements: Rust (tested with 1.97.0), Python 3.11+ and PostgreSQL 16+. From the repository root:

```sh
cd projects/qazna.org
cargo fetch --locked
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/check.py
```

On Linux, set `PG_BIN` to the installed PostgreSQL directory, for example `/usr/lib/postgresql/16/bin`. Checks create a private, socket-only temporary cluster and stop and delete it afterward. Existing databases are not used.

## Documentation

- [Current priorities and deferred sector studies](docs/completion-roadmap.md#strategic-priorities) · [Pilot charter](docs/pilot-charter.md)
- [Physical chains and monetary circulation](docs/physical-circulation.md) · [Published calculations](research/) · [Mathematics and computing costs](docs/computation-and-ai-costs.md)
- [Needs and planning norms](docs/norms-and-planning.md) · [Provisioning](docs/provisioning.md) · [Production](docs/production.md) · [Reviewed decisions](docs/drafts.md)
- [AI and economic research](docs/ai-economic-research.md) · [Optional AI and its limits](docs/ai-safety.md) · [Architecture review](docs/architecture-review.md)
- [Operational exercises](docs/exercises.md) · [Access and recovery](docs/recovery-and-access.md) · [Continuous integration](https://github.com/DauletBai/tartarus/actions/workflows/qazna.yml)
- [Changelog](CHANGELOG.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Brand assets](assets/brand/README.md)

## Licensing

The software is [AGPL-3.0-or-later](LICENSE), and Qazna documentation is [CC BY-SA 4.0](LICENSE-DOCS). The book and third-party materials have separate terms. [Scope](NOTICE) · [Dependency inventory](THIRD_PARTY_LICENSES.json).
