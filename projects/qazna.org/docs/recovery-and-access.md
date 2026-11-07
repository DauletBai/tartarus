# Revocation, runtime privileges and recovery

Version 0.11 separates three additional controls from the economic model.

## Principal revocation and scoped reads

An administrator with a separately held database connection can run:

```sh
QAZNA_REVOKE_PRINCIPAL='principal-uuid' \
QAZNA_REVOCATION_BASIS='Incident reference and accountable decision' \
DATABASE_URL='administrator-connection' cargo run --locked -- revoke-principal
```

Revocation is append-only and permanent for that principal. Identical requests are
idempotent; changing the recorded basis conflicts. The API provides no operator,
reviewer or attestor route to revoke someone. Live requests check database state;
service restart is not required to revoke an existing principal. Concurrent writes
serialize on the principal row. Requests already authorized before revocation can
finish; subsequent statements, including old idempotency replays, are denied.

Revocation does not erase past evidence, undo settlement or automatically change
whether an old confirmation counts. Suspect historic claims require an explicit
independent dispute. A replacement identity/re-enrollment procedure is still open;
removing and replacing a token configuration alone does not un-revoke a principal.
This is a security control, not a punishment or a way to delete a person's needs.

Recipients can read evidence only for their own deliveries and related resources;
custodians can read their resources and deliveries; inspectors can read resource
evidence, not unrelated delivery records. Operator-wide access remains a local
administrative capability. OIDC, independently governed enrollment and comprehensive
resident account/organization authorization remain unfinished.

## Database runtime group

Run `fixtures/runtime_permissions.sql` as the migration owner in a dedicated Qazna
database after migrations. Provision a separately authenticated login with membership
in the `qazna_runtime` NOLOGIN group; keep owner/admin credentials out of the runtime.
Do not give that login other roles, ownership or PUBLIC privileges that widen access.
Reapply reviewed grants after new migrations. Default future-table privileges are
not automatically expanded.

The script grants schema usage, table reads/inserts/row-lock-compatible updates and
sequence access. Existing immutable-history triggers reject updates. It grants no
DELETE, TRUNCATE, schema creation or ownership; migration history, evidence activation
and revocation writes are excluded. The runtime cannot disable triggers. The group
is a damage-reduction boundary, not an independent business-authorization engine:
a compromised application can still exercise its granted insertion capabilities.
A database superuser remains trusted. The test login used in CI is disposable and
relies solely on the runner's private socket; do not copy that authentication setup
to production.

## Logical restore rehearsal

`scripts/check.py` runs `scripts/recovery_check.py` inside its own disposable cluster:

1. Hash the canonical ordered rows of all public tables in a populated strict-mode DB.
2. Create a custom-format logical dump, restore into a new database and compare every
   table fingerprint, including evidence, accounting and migration history.
3. Verify immutable-history triggers, enforcement and forged-transaction-ID rejection.
4. Verify runtime denial of six destructive/administrative operations.
5. Run the actual HTTP evidence/provisioning scenario on the restored database using
   a non-owner runtime login, keeping evidence activation under administrator control.

Migration 0012 moves the transaction-dependent journal CHECK to a BEFORE INSERT
trigger. A dump restores historical rows before installing triggers; normal inserts
still require the current transaction ID. This fixes logical restore without editing
released migrations or rewriting historical rows. `build.rs` ensures new migration
files trigger rebuilding of the SQLx-embedded migration set.

This does not establish encrypted off-site backups, point-in-time recovery,
cross-region continuity, production RPO/RTO, or credential recovery. Those require
separate operational work. Dump files contain sensitive data in real deployments;
protect them, restrict access and verify retention before handling participant data.
