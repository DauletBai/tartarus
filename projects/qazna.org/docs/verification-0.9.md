# Verification record: 0.9.0

Local verification on 3 November 2026, macOS, Rust 1.97.0 and PostgreSQL 16.10. GitHub CI separately runs the offline suite on Ubuntu/PostgreSQL 16; consult the PR checks for its actual status.

- Required `scripts/check.py`: passed rustfmt, Clippy with warnings denied, dependency license inventory, 3 Rust unit tests, 11 offline AI adapter tests and 81 PostgreSQL integration tests.
- HTTP smoke: passed durable coordination/reviews, production provenance and atomic application across three service restarts.
- Accelerated actor scenarios: seeds 7 and 19 passed, 21 snapshots each; no model calls.
- Final wall-clock scenario: seed 7, six residents, 18 needs, 40 one-second target ticks; 41 consistent snapshots in 42.732 seconds including setup/shutdown. Largest measured interval between periodic snapshots: 1.454 seconds.
- Queue recovery: 35 calculations completed, zero remaining events/queued/running/failed jobs. Five-second backlog warning observed during worker outage.
- All observed accounting, stock, capacity and need invariants passed. Eight concurrent requests for one available unit produced exactly one successful hold.
- A dishonest trusted declaration of 100 nonexistent meals was accepted. The private scenario oracle knew the true quantity was zero; Qazna did not independently detect the lie. This remains a known gap, not a prevented attack.
- One explicit live ChatGPT-authenticated test on the earlier wall-clock calculation returned five evidence-checked findings and retained operator approval. The CLI did not identify its exact model version; the adapter recorded `codex-default`. No model-controlled economic action occurred.
- Live and saved report rendering was checked in Safari. Quantities remain separated by period/category/unit, and evidence limitations are visible.

Local artifacts are excluded from Git: `.local/exercises/v09-release/` contains final report, JSONL events, service log, exact binary/harness hashes and check output. `.local/exercises/v09-wallclock/ai-review.json` contains the separately retained live model commentary. These synthetic test outputs are not statements about real residents or production.

Final artifact hashes:

| Artifact | SHA-256 |
| --- | --- |
| `report.json` | `377c700ca6a145631fefc4edf3b66763a0351ed192ff0d24cc37e72b6786586e` |
| `events.jsonl` | `966c8e872680464097826a310a1153e22141e8dce643f372e7d4bd3d6781ae17` |
| `checks.log` | `c38253cc4b23fd40d9061d4764dbdc592d017a9c3ef2c5c0074d34eb95a15571` |
| Tested local binary | `c976d5c5d6532755c7b363edeb07fc3a2c78a01305074efeaa5244d13ead3b9a` |
| Exercise harness | `433112ba89d1f0cbb74a35a1ad5911b89551ac8445c830c31f6e92f706316531` |

The [organism audit](organism-audit.md) lists unresolved design responsibilities. Passing this bounded exercise does not establish complete security, industrial performance, biological validity or economic viability.
