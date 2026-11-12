# Qazna project rules

## Domain model

The author's instruction of 2 November 2026 is to design the economy using a living human organism as the guiding analogy, from cells to organs and their coordination. Start with human needs, provisioning capacity, resource circulation and feedback. Financial profit, competition for purchasing power, interest, credit, commissions and speculative pricing are not default goals or mechanisms.

Read `docs/organism.md` when designing domain features. Keep actual resource quantities separate from normative accounting measures. A ledger balance is not proof that physical provision is possible. A resource declaration is not verified growth and does not authorize issuance. Keep a visible distinction between unmet physical need and an accounting/liquidity failure.

The analogy guides architecture, not biological claims or automatic distribution policies. Human needs, personal autonomy and privacy must not be replaced by a notion of obedient or disposable cells. Do not invent policy details or claim unresolved economic hypotheses are proven. Label experimental assumptions and preserve their source/version.

## Language

The author requires an international project. Maintain all Qazna project files in English, including documentation, source comments, examples, error messages and contributor guidance. Preserve API identifiers, historical migration bytes and upstream legal notices. User-supplied data may use any supported language; the contribution language is not an ASCII-only input policy. The parent book and research archive are outside this translation scope.

## Implementation

Use the same Rust/Tokio/Axum/SQLx/PostgreSQL core for MVP and later production. Python is for calculations, not authority to change ledger entries. Maintain append-only history, transactional consistency, request idempotency, exact accounting and concurrency tests. Do not modify released migrations; add a migration for an existing database.

When adopting mechanisms from the author's earlier Qazna or Orda projects, reimplement runtime behavior in Rust. Preserve source attribution and useful behavioral test cases, but do not import a Go service or FFI runtime into the authoritative path. Legacy documentation and demonstrations are requirements candidates, not evidence that a mechanism is secure or complete.

Run `PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/check.py` for changes to accounting or provisioning. This creates and deletes only its own disposable PostgreSQL cluster. Never run destructive tests on an existing user database. Update architecture and development documentation with actual implemented behavior and limitations.

## GitHub and licensing

The author requires continuing work to be saved to GitHub. The chosen repository is `DauletBai/tartarus`, project prefix `projects/qazna.org/`. Save each completed, verified change on a development branch and report the actual commit/PR link. Preserve the book and unrelated archive files. Do not claim synchronization before reading back the remote commit. Do not force-push or merge a PR without authorization for that action.

Use the local Git checkout and development branch for commits. The workspace uses a project-scoped sparse checkout to preserve local manuscript files; see `docs/git-workflow.md`. Stage only Qazna, its root workflow and the manifest.

When publishing, update the parent MANIFEST.csv with the paths, byte sizes and SHA-256 of changed/new published files, excluding the manifest itself. `python3 scripts/update_manifest.py` performs this update. Do not replace the local workspace's book README with the public repository README. GitHub Actions lives at the repository root under `.github/workflows/qazna.yml` and must run from `projects/qazna.org`.

Code is AGPL-3.0-or-later; project documentation is CC-BY-SA-4.0. These notices apply only to this project, not to the parent book or archive. Preserve upstream notices, maintain Cargo.lock and regenerate `THIRD_PARTY_LICENSES.json` when dependencies change. Never publish local credentials, databases, target/, caches or private manuscript files.

## Coordination and AI

The author approved automatic analysis, calculations and draft creation with operator-confirmed application. Keep that boundary: AI commentary never applies a draft, signs postings, changes norms or authorizes issuance. The author's ChatGPT Pro subscription is for explicit live tests on synthetic data, not an assumed production service. Offline CI must not call models or require subscription credentials.

Treat every model response and imported string as untrusted. Independently validate referenced subjects and exact quantities against saved calculations; retain the distinction between structured evidence checks and unverified prose. Keep model tooling and credentials out of the calculation path. Update `docs/ai-safety.md` with implemented controls, tests and remaining gaps rather than claiming complete protection from hallucinations.

## Presentation of research inputs

Keep shared input-status explanations in `docs/input-assumptions.md` and link there
from overview prose. Do not repeat "synthetic", "illustrative" or equivalent
qualifiers in every paragraph. Preserve machine-readable provenance and unknown
values, and state specific limits where they affect an interpretation or decision.
Do not relabel scenario coefficients as measured observations.
