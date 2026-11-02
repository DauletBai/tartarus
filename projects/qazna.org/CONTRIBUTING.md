# Contributing to Qazna

Start with the [organism-inspired principles](docs/organism.md), [architecture](docs/architecture.md) and [provisioning contract](docs/provisioning.md). Economic rule changes must state their assumptions, basis and a reproducible scenario. Do not substitute purchasing power or profit targets for human needs.

Use English for all maintained project files: documentation, comments, examples, error messages and contributor materials. Preserve public API identifiers and historical migration bytes. English as the contribution language does not restrict user-supplied names or descriptions to ASCII. Keep upstream legal notices and attribution intact.

Workflow: development branch → implementation and documentation → `python3 scripts/check.py` → commit → push to GitHub → pull request. Accounting and provisioning changes need meaningful concurrency and rollback checks. Upgrade existing databases through new SQL migrations. Qazna changes must not alter the manuscript, published book PDF or unrelated research archive.

Code contributions are provided under AGPL-3.0-or-later; documentation contributions under CC-BY-SA-4.0. Submit only material you can distribute under these terms. Preserve third-party notices and compatible terms rather than automatically replacing them with the project license. This workflow does not require copyright assignment.

Do not commit passwords, tokens, real residents' personal data, production database dumps or build output. Use synthetic data in tests. Discuss decisions respectfully and with evidence; model failures and negative results are useful research findings.
