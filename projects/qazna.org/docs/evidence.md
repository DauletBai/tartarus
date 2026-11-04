# Independent attestation protocol

Version 0.10 adds credential-separated statements about resource lots and resident delivery. This is an evidence workflow, not a claim that the software independently observes physical reality or prevents collusion.

## Activation and compatibility

Migration 0010 preserves historical economic records. Existing research databases start with evidence enforcement disabled; status explicitly reports `physical_evidence.enforced=false` and an alert. Enable it once, after resolving outstanding resident and production holds:

```sh
export QAZNA_EVIDENCE_BASIS='Explicit pilot transition after resolving outstanding holds'
cargo run --locked -- enforce-evidence
```

Activation is durable and cannot be undone through the API. Pending holds block activation. Command transactions and database triggers serialize activation against resource use. Old unverified lots then become unavailable for new use until attested. Historical receipts remain historical receipts. This is an explicit migration boundary, not an automatic claim that older records were verified.

## Roles and observations

The operator enrolls a principal with `POST /v1/evidence/principals`: `id`, `account`, `role`, `basis`. Roles are `inspector`, `custodian` and `recipient`. A principal/account/role association is immutable. The custodian must be a producer account and the recipient a resident. There can be only one enrolled principal per account/role.

A local administrator separately configures `QAZNA_ATTESTORS_JSON` as an array of `{"principal":"UUID","token":"DISTINCT_SECRET_AT_LEAST_32_BYTES"}`. Never put actual tokens in source control. At most 64 credentials are supported in this local pilot. Tokens must differ from each other and from operator/observer/reviewer credentials. Token rotation currently requires a service restart; production identity, organizational independence, replacement enrollment and external signature verification remain future work. Version 0.11 adds permanent administrator revocation and scoped evidence reads; see [recovery and access](recovery-and-access.md).

An attestor can only read or submit evidence at `GET/POST /v1/evidence/{kind}/{subject}`, where kind is `resource` or `delivery`. The server derives the principal from the credential; clients cannot supply a different actor in the body. The operator/reviewer/observer credential cannot submit an attestor statement. Operator application of economic actions remains separate.

A statement contains `id`, `decision` (`confirm` or `dispute`), positive canonical string `quantity`, lowercase 64-character `evidence_sha256`, and `basis`. Use a digest of the retained evidence document. Qazna stores that digest; it does not fetch documents, establish their authenticity, or currently provide a document archive. A statement applies to an immutable subject whose quantity, dimension and parties are fixed by its recorded ID. Wrong quantities and ineligible principals are rejected.

## Enforced gates

- A resource needs confirmations from two inspector accounts distinct from its producer. Two credentials for the same inspector account do not provide two independent votes.
- Pending/disputed lots have zero usable availability. Resident reservations, production inputs and planning use the same availability. Direct SQL inserts also face evidence constraints.
- Completion consumes attested inputs but does not automatically attest output. Each new output lot awaits its own observations, preserving production provenance.
- A delivery needs confirmation by the actual resource custodian and the reservation's actual recipient. An inspector or unrelated resident cannot stand in for either party.
- Settlement requires both an attested resource and attested delivery; cancellation remains available. An evidence statement itself never transfers accounting measure or marks receipt.
- A later dispute is append-only and blocks further use/settlement. It does not erase a completed receipt or reverse a payment. Dispute adjudication and replacement/correction protocols are deliberately unfinished.

Same-key retries retain the original response; changed identity or content conflicts. Repeating the same actor/subject/decision under a new key conflicts. An actor may append a dispute after an earlier confirmation, without rewriting history.

## Verification and remaining limits

The PostgreSQL suite covers activation, strict lot and delivery gates, self-attestation, incorrect quantities, duplicates, direct SQL, append-only disputes, restricted HTTP credentials and production output quarantine. `scripts/evidence_exercise.py` runs the actual HTTP binary with separate synthetic credentials and records results. The ordinary historical actor exercise stays in compatibility mode and continues to demonstrate its known trust gap; its dashboard now exposes disabled enforcement.

The first operational improvement is that a lone unverified declaration cannot be used in strict mode. Two colluding enrolled inspectors can still make a false claim appear attested. Account separation is not established real-world independence. A database owner, compromised host or administrator controlling all credentials remains trusted. No automated confiscation, identity rating or monetary issuance was added. Real deployments require independently governed enrollment, secure evidence custody, key lifecycle management, external audit and a dispute process.
