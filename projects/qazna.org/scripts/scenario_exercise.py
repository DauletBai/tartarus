"""Real HTTP process restarts and continuation after logical backup restoration.

Only called by check.py/recovery_check.py inside their disposable test cluster.
"""
from contextlib import contextmanager
import copy
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
        model_reference = populate_model(call)
    with server(url) as call:
        assert call(model_reference['path']) == model_reference['saved']
        assert call(model_reference['draft_path']) == model_reference['pending']
    print('Physical model: full accepted and pending state survives process restart', flush=True)
    print('Dated scenario: 2 process restarts, pending and approved bytes preserved', flush=True)
    return dict(path=path, saved=saved, pending=next_draft, model=model_reference)


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
        resume_model(call, reference['model'])
    print('Dated scenario: restored history continued under least-privilege runtime role', flush=True)


def populate_model(call):
    dated_id, model_id = uid(), uid()
    dated_path = f'/v1/scenarios/dated/{dated_id}'
    model_path = f'/v1/scenarios/models/{model_id}'
    call('/v1/scenarios/dated', dict(id=dated_id, format_version=1, unit='test_unit',
         resources=['warehouse'], basis='Synthetic aligned absolute days'))
    d = call(dated_path + '/drafts', dict(id=uid(), expected_generation=0,
        basis='Synthetic stock at day 1', commands=[dict(kind='advance', period=0), dict(kind='advance', period=1),
        dict(kind='introduce', id=1, resource=0, quantity=97, available=1, expires=20,
             basis='Synthetic stock, not physical attestation')]))
    call(f"/v1/scenario-drafts/{d['request']['id']}/resolve", dict(apply=True, basis='Reviewed'))
    call('/v1/scenarios/models', dict(id=model_id, format_version=1,
        resources=[dict(name=n, unit='test_unit') for n in ['material', 'machine', 'food']],
        period_unit='day', time_origin='Synthetic day zero',
        dated_bindings=[dict(scenario_id=dated_id, model_resources=[0], basis='Same absolute days and units')],
        basis='Synthetic recovery fixture; no economic calibration'))
    data = json.loads((ROOT / 'fixtures/model-scenario.json').read_text())
    data['dated_generations'] = [1]
    first = call(model_path + '/drafts', dict(id=uid(), expected_generation=0, input=data, basis='Initial physical plan'))
    call(f"/v1/model-drafts/{first['request']['id']}/resolve", dict(apply=True, basis='Operator reviewed'))
    saved = call(model_path)
    state = json.loads(saved['checkpoint'])
    assert state['rolling']['opening_stock'] == [97, 1, 0]
    assert len(state['rolling']['pending']) == 2
    assert state['rolling']['opening_cohorts'][0]['expires'] == 3
    # Modify future supply only, preserve the immutable prefix and two starts.
    data = copy.deepcopy(data)
    data['model']['seasonal']['physical']['periods'][5]['supplies'][0] = 2
    for row in data['plan'][5:]:
        row['closing'][0] += 2
    pending = call(model_path + '/drafts', dict(id=uid(), expected_generation=1, input=data, basis='Future supply revised'))
    return dict(path=model_path, saved=saved, pending=pending,
                draft_path=f"/v1/model-drafts/{pending['request']['id']}", dated_path=dated_path)


def resume_model(call, reference):
    assert call(reference['path']) == reference['saved']
    assert call(reference['draft_path']) == reference['pending']
    approved = call(reference['draft_path'] + '/resolve', dict(apply=True, basis='Review after backup restore'))
    assert approved['candidate'] == reference['pending']['candidate']
    data = copy.deepcopy(approved['request']['input'])
    # Material consumed during period 2; equipment completion retains its age.
    d = call(reference['dated_path'] + '/drafts', dict(id=uid(), expected_generation=1,
        basis='Synthetic next day', commands=[dict(kind='advance', period=2),
        dict(kind='withdraw', resource=0, quantity=1)]))
    call(f"/v1/scenario-drafts/{d['request']['id']}/resolve", dict(apply=True, basis='Reviewed'))
    data['boundary'] = 3
    data['dated_generations'] = [2]
    next_draft = call(reference['path'] + '/drafts', dict(id=uid(), expected_generation=2,
        input=data, basis='Advance committed boundary after restore'))
    state = json.loads(next_draft['candidate']['checkpoint'])
    assert state['rolling']['opening_stock'] == [96, 2, 0]
    assert [v['expires'] for v in state['rolling']['opening_cohorts']] == [3, 6]
    assert state['materials'][0]['initial'] == 102
    assert state['dated'][0]['generation'] == 2
    call(f"/v1/model-drafts/{next_draft['request']['id']}/resolve", dict(apply=True, basis='Operator reviewed continuation'))
    assert call(reference['path'] + '/versions/1') == reference['saved']
    print('Physical model: restored version advances with original cohorts, WIP and dated anchors', flush=True)
