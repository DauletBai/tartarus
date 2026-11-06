# AI safety boundaries and verification

The model is an untrusted adviser. No finite checklist or collection of passing tests proves that all hallucinations, prompt injections or software vulnerabilities have been eliminated. The project constrains consequences and checks evidence independently of model confidence.

## Threat model

Protect resident autonomy and privacy, exact accounting, physical inventory, declared capacity, decision provenance, operator credentials and service availability. Inputs that may be wrong or adversarial include names/units/bases, imported observations, operator reports, model responses and future retrieved documents. A valid JSON shape or a confident explanation is not evidence of factual correctness.

| Failure or attack | Implemented control | Remaining boundary |
| --- | --- | --- |
| Invented quantity or entity | Each structured finding cites a saved nonzero signal, exact quantity and known subject; duplicate or unknown references are rejected by Rust and the adapter | Free-text commentary can still be mistaken or omit a risk |
| Hidden instruction in source data | Operator basis text and command keys are removed; remaining JSON is framed as data; no model tools are required | Prompt framing alone is not a security boundary |
| Model attempts an action | Adapter has fixed read/review API paths, no apply operation, no DB credentials in the child and no supplied operator token | The reviewer token cannot mutate economics; resident identities and runtime DB roles remain unfinished |
| Command, browser or connector use | Read-only temporary working directory, user config ignored, shell/browser/apps/plugins/multi-agent execution capabilities disabled; tool events rejected | The local CLI and its implementation remain trusted software, not a hardware isolation boundary |
| False claim that a plan was applied | Reports require `requires_operator_approval: true`; extra action fields are rejected; actual state comes from immutable draft/action records | Operator UI must clearly label commentary and show authoritative state separately |
| Stale or duplicate reasoning | Reviews attach to a specific immutable calculation; drafts are revalidated under locks before operator application | No automatic expiry interval or global revision clock is implied |
| Worker crash or overlapping workers | Durable jobs, bounded retries, lease tokens, expiry checks and atomic draft/result commits | Hosting, supervision and capacity planning remain necessary |
| Delayed committed event | Transactional outbox and undispatched-row scanning, without a sequence high-water mark | Writes outside the command contract do not emit events |
| Malformed or partial response | JSON schema request, strict fields, bounded sizes, completed-turn requirement and local/server validation | Structured output constrains format, not truth |
| Provider quota, auth or timeout | One-shot explicit test calls, 120-second model deadline and no API-key fallback; calculations continue independently | No always-on model availability or subscription latency guarantee |
| Credential leakage | Minimal child environment, no credential-file copying, no session logs published, loopback API only and redirects rejected | Real resident data needs a consent, minimization and retention design before model transmission |
| Unsafe rendered output | Current output is JSON/plain text; no model code or generated HTML is executed | Future UI must escape text and separately govern URLs, markup and downloads |
| Unapproved economic policy | Explicit operator priority, no inferred issuance, fees, personal ratings or cancellation rules | Scientific/economic validity and shortage priorities require human governance |

## Evidence contract

A review has `summary`, `findings`, `questions` and `requires_operator_approval`. A finding contains `signal`, `subject`, `quantity` and `explanation`. Supported evidence is deliberately narrow:

- `physical_residual`: the saved need line's residual quantity;
- `production_forecast`: its expected contribution from production, not stock or receipt;
- `accounting_shortage`: the saved resident accounting gap.

The subject must match the corresponding need/resident UUID and the quantity must match its exact nonzero string. The server rejects unsupported signals, fabricated subjects, wrong amounts, duplicate references, approval bypasses and oversized reports before saving a review. Validation does not certify the summary/explanation/questions, require exhaustive findings or independently verify the original physical declarations. Always inspect the full deterministic proposal as well as the commentary.

A recorded provider/model is declared adapter provenance, not cryptographic attestation that the named model produced the text. The local operator can submit reviews, so production audit must distinguish authenticated model-service reports from human imports. No model report is allowed to mutate the referenced calculation or draft.

## Verification layers

Offline adapter tests cover instruction minimization, fabricated quantities/IDs/signals, duplicate evidence, approval bypass, extra executable fields, API-key login refusal, child credential exclusion, partial streams, tool/error events and redirect refusal. PostgreSQL scenarios independently reject fabricated findings and verify that accepted commentary changes neither stock nor postings. Worker tests exercise lease recovery, stopped/expired workers, concurrent claims, rollback and out-of-order commits.

Explicit live tests use synthetic data and the author's ChatGPT login. A passing example establishes that the configured integration can produce a checked report; it is not a statistical hallucination rate. Before widening use, build a versioned evaluation corpus with correct, ambiguous, contradictory, malicious and out-of-distribution cases. Measure unsupported factual claims, missed critical shortages, prompt-injection success, refusal/timeout rate, latency and review effort against human-labelled references. Keep model changes and evaluation results traceable.

OpenAI's [agent safety guidance](https://developers.openai.com/api/docs/guides/agent-builder-safety) discusses separating untrusted data and approvals; its [structured output documentation](https://developers.openai.com/api/docs/guides/structured-outputs) describes schema-constrained responses. Qazna adds independent domain checks rather than treating either as proof of economic or factual validity.

## Before an external pilot

Implemented since the earlier baseline: a review-only credential, restricted runtime database group, live attestor revocation and logical backup/restore rehearsal. Still required: OIDC and resident/operator/auditor ownership checks; protected transport; per-scope queue quotas/coalescing and retention; model-call budgets; production backup/PITR and disaster exercises; dependency and supply-chain review; signed/reproducible releases; monitoring and incident response; independent review of prompt-injection and data-exfiltration controls; and privacy rules for real participant data. See the [current architecture and test review](architecture-review.md), including the presently hardcoded review-provider identity.

The current local operator token and a database owner can bypass application-level policy. These are explicit deployment limitations, not protections supplied by the organism analogy. Stop a coordination monitor to prevent further calculations for that scope; stopping it does not undo already applied economic actions. Existing cancellation routes retain their audit trails.

## Operational drill evidence (v0.9)

The observer credential cannot read accounts or issue commands. The reviewer credential can only read saved calculations/reviews and append validated reviews. The API-mode adapter uses this reviewer credential, with no operator-token fallback. The [exercise runner](exercises.md) tries privilege escalation and fabricated model facts against the actual HTTP API. Its report escapes all displayed values and never executes model-generated markup.

Independent monitoring detects deliberately injected ledger corruption in an isolated test. A separate drill deliberately accepts a trusted false physical declaration and labels it a known gap. Neither success at numerical checks nor model agreement supplies independent physical evidence. See the [organism audit](organism-audit.md) for the remaining controls.
