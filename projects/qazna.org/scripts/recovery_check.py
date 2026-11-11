#!/usr/bin/env python3
"""Restore a populated synthetic database and exercise least-privilege HTTP access.

Called only by check.py in its own private disposable cluster. This is a logical
backup rehearsal, not PITR, off-site storage, production RPO or regional failover.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import time
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[1]


def check_recovery(pg, root, socket_dir, source='evidence_demo'):
    def sql(database, query, expected=True, contains=None):
        p = subprocess.run([pg('psql'), '-X', '-h', str(socket_dir), '-U', 'qazna', '-d', database,
                            '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], capture_output=True, text=True)
        if expected and p.returncode:
            raise AssertionError(p.stderr)
        if not expected:
            assert p.returncode != 0, 'restricted operation unexpectedly succeeded'
            if contains:
                assert contains in p.stderr, p.stderr
        return p.stdout

    def fingerprint(database):
        tables = sql(database, "SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename").splitlines()
        result = {}
        for table in tables:
            assert table.replace('_', '').isalnum()
            rows = sql(database, f'SELECT row_to_json(t)::text FROM public."{table}" t ORDER BY row_to_json(t)::text COLLATE "C"')
            result[table] = hashlib.sha256(rows.encode()).hexdigest()
        return result

    from scenario_exercise import populate_and_restart, resume_restored
    source_url = 'postgresql://qazna@localhost/' + source + '?' + urlencode({'host': str(socket_dir)})
    reference = populate_and_restart(source_url)
    started = time.monotonic()
    before = fingerprint(source)
    archive = root / 'synthetic-backup.dump'
    subprocess.run([pg('pg_dump'), '-h', str(socket_dir), '-U', 'qazna', '-d', source,
                    '-Fc', '--no-owner', '--no-privileges', '-f', str(archive)], check=True)
    restored = 'recovery_demo'
    subprocess.run([pg('createdb'), '-h', str(socket_dir), '-U', 'qazna', restored], check=True)
    subprocess.run([pg('pg_restore'), '-h', str(socket_dir), '-U', 'qazna', '-d', restored,
                    '--no-owner', '--no-privileges', '--exit-on-error', str(archive)], check=True)
    assert fingerprint(restored) == before, 'restored table contents differ'
    assert sql(restored, "SELECT count(*) FROM evidence_statements").strip() != '0'
    assert sql(restored, "SELECT count(*) FROM evidence_enforcement").strip() == '1'
    assert sql(restored, "SELECT count(*) FROM dated_scenario_outcomes").strip() == '2'
    assert sql(restored, "SELECT count(*) FROM model_scenario_outcomes").strip() == '1'
    assert sql(restored, "SELECT count(*) FROM coupled_scenario_outcomes").strip() == '1'
    # Verify restored triggers still reject history mutation.
    sql(restored, 'DELETE FROM evidence_statements', expected=False)
    sql(restored, 'DELETE FROM evidence_enforcement', expected=False)
    sql(restored, "INSERT INTO journal_transactions(id,idempotency_key,kind,request,created_xid) VALUES('00000000-0000-0000-0000-000000000099','forged-xid','transfer','{}','1'::xid8)", expected=False, contains='transaction must use its current transaction ID')

    sql(restored, (ROOT / 'fixtures/runtime_permissions.sql').read_text())
    sql(restored, 'CREATE ROLE qazna_test_runtime LOGIN IN ROLE qazna_runtime')
    for forbidden in [
        'CREATE TABLE public.unauthorized_table(id int)',
        'ALTER TABLE evidence_statements DISABLE TRIGGER ALL',
        'DELETE FROM evidence_statements',
        'TRUNCATE evidence_statements CASCADE',
        'UPDATE evidence_statements SET basis=basis',
        'UPDATE dated_scenarios SET initial=initial',
        'DELETE FROM dated_scenario_drafts',
        'TRUNCATE dated_scenario_outcomes',
        'UPDATE model_scenarios SET definition=definition',
        'DELETE FROM model_scenario_drafts',
        'TRUNCATE model_scenario_outcomes',
        'UPDATE coupled_scenarios SET definition=definition',
        'DELETE FROM coupled_scenario_drafts',
        'TRUNCATE coupled_scenario_outcomes',
        "INSERT INTO evidence_principal_revocations(principal_id,basis) SELECT id,'bypass' FROM evidence_principals LIMIT 1",
    ]:
        sql(restored, 'SET ROLE qazna_runtime; ' + forbidden, expected=False)
    # Exercise all strict-evidence HTTP routes under the runtime role in a fresh DB.
    # The restored strict DB has intentional disputes; preserve it for reconciliation.
    from evidence_exercise import run_evidence_exercise
    runtime_url = 'postgresql://qazna_test_runtime@localhost/' + restored + '?' + urlencode({'host': str(socket_dir)})
    # Activation is already durable, so the runtime must only serve, never reactivate.
    run_evidence_exercise(runtime_url, root / 'runtime-evidence', activate=False)
    resume_restored(runtime_url, reference)
    report = dict(result='passed', tables_verified=len(before), table_sha256=before,
                  elapsed_seconds=round(time.monotonic()-started,3), runtime_denials=15, scenario_restart_restore=True, model_restart_restore=True, coupled_restart_restore=True,
                  limitation='Synthetic logical backup; no PITR or production RPO/RTO guarantee')
    (root / 'recovery-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f"Backup/restore and runtime privilege checks passed: {len(before)} tables", flush=True)
