"""Real HTTP process restarts and continuation after logical backup restoration.

Only called by check.py/recovery_check.py inside their disposable test cluster.
"""
from contextlib import contextmanager
import json
import os
import secrets
import socket
import subprocess
import time
from urllib.error import URLError
from exercise import Client, ROOT, stop, uid


@contextmanager
def server(url):
    token = secrets.token_hex(32)
    env = dict(os.environ, DATABASE_URL=url, QAZNA_OPERATOR_TOKEN=token)
    for name in ['QAZNA_OBSERVER_TOKEN', 'QAZNA_REVIEW_TOKEN', 'QAZNA_ATTESTORS_JSON']:
        env.pop(name, None)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env['QAZNA_BIND'] = f'127.0.0.1:{port}'
    process = subprocess.Popen([str(ROOT / 'target/debug/qazna'), 'serve'], env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    client = Client(f'http://127.0.0.1:{port}', {'operator': token})
    try:
        deadline = time.monotonic() + 10
        while True:
            try:
                client.request('/v1/system/status')
                break
            except URLError:
                assert process.poll() is None and time.monotonic() < deadline
                time.sleep(.05)
        yield client.request
    finally:
        stop(process)


def populate_and_restart(url):
    scenario, first, pending = uid(), uid(), uid()
    path = f'/v1/scenarios/dated/{scenario}'
    definition = dict(id=scenario, format_version=1, unit='grain_kg', resources=['store', 'kitchen'],
                      basis='Synthetic recovery fixture, not observed inventory')
    proposal = dict(id=first, expected_generation=0, basis='Synthetic pending harvest and transport', commands=[
        dict(kind='advance', period=0),
        dict(kind='introduce', id=1, resource=0, quantity=20, available=0, expires=5, basis='Opening lot'),
        dict(kind='transfer', source=0, target=1, quantity=7, due=2),
        dict(kind='introduce', id=2, resource=0, quantity=4, available=3, expires=7, basis='Future harvest'),
    ])
    approve = dict(apply=True, basis='Reviewed synthetic state')
    with server(url) as call:
        initial = call('/v1/scenarios/dated', definition, key='scenario-recovery-create')
        draft = call(path + '/drafts', proposal, key='scenario-recovery-proposal')
        assert call(path) == initial
    # Pending draft survives process restart, without being silently accepted.
    with server(url) as call:
        assert call(f'/v1/scenario-drafts/{first}') == draft
        assert call(path) == initial
        applied = call(f'/v1/scenario-drafts/{first}/resolve', approve, key='scenario-recovery-apply')
        saved = call(path)
        assert saved == applied['candidate']
        next_draft = call(path + '/drafts', dict(id=pending, expected_generation=1,
            basis='Next reviewed time boundary', commands=[dict(kind='advance', period=1)]))
    # Accepted bytes and idempotent result survive a second process restart.
    with server(url) as call:
        assert call(path) == saved
        assert call(f'/v1/scenario-drafts/{first}/resolve', approve, key='scenario-recovery-apply') == applied
        assert call(path + '/versions/0') == initial
        assert call(f'/v1/scenario-drafts/{pending}') == next_draft
    print('Dated scenario: 2 process restarts, pending and approved bytes preserved', flush=True)
    return dict(path=path, saved=saved, pending=next_draft)


def resume_restored(url, reference):
    path, expected = reference['path'], reference['pending']
    with server(url) as call:
        assert call(path) == reference['saved']
        pending = expected['request']['id']
        assert call(f'/v1/scenario-drafts/{pending}') == expected
        applied = call(f'/v1/scenario-drafts/{pending}/resolve',
                       dict(apply=True, basis='Review after logical restore'))
        assert applied['candidate'] == expected['candidate']
        assert call(path) == expected['candidate']
        next_draft = call(path + '/drafts', dict(id=uid(), expected_generation=2,
            basis='Verify original arrival and expiry after restore', commands=[dict(kind='advance', period=2)]))
        state = json.loads(next_draft['candidate']['checkpoint'])
        assert state['stocks']['1']['1'] == 7
        assert state['origins']['1']['expires'] == 5
        assert not state['origins']['2']['released']
        assert not state['transit']
    print('Dated scenario: restored history continued under least-privilege runtime role', flush=True)
