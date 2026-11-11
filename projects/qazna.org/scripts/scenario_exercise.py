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
        coupled_reference = populate_coupled(call, model_reference)
        extended_reference = populate_extended(call)
    with server(url) as call:
        assert call(model_reference['path']) == model_reference['saved']
        assert call(model_reference['draft_path']) == model_reference['pending']
        assert call(coupled_reference['path']) == coupled_reference['saved']
        assert call(coupled_reference['draft_path']) == coupled_reference['pending']
        assert call(extended_reference['coupled_path']) == extended_reference['coupled']
        assert call(extended_reference['draft_path']) == extended_reference['pending']
    print('Physical model: full accepted and pending state survives process restart', flush=True)
    print('Dated scenario: 2 process restarts, pending and approved bytes preserved', flush=True)
    return dict(path=path, saved=saved, pending=next_draft, model=model_reference, coupled=coupled_reference, extended=extended_reference)


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
        resume_coupled_pending(call, reference['coupled'])
        resume_model(call, reference['model'])
        resume_coupled_current(call, reference['coupled'])
        resume_extended(call, reference['extended'])
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


def populate_coupled(call, model):
    identifier = uid()
    path = f'/v1/scenarios/coupled/{identifier}'
    policy = json.loads((ROOT / 'fixtures/circulation-policy.json').read_text())
    call('/v1/scenarios/coupled', dict(id=identifier, format_version=1,
        model_id=model['saved']['definition']['id'], policy=policy,
        basis='Synthetic fixed-norm circulation linked to approved production'))
    first = call(path + '/drafts', dict(id=uid(), expected_generation=0,
        model_generation=1, unavailable_events=[], basis='Reviewed complete schedule'))
    call(f"/v1/coupled-drafts/{first['request']['id']}/resolve",
         dict(apply=True, basis='Operator reviewed synthetic circulation'))
    saved = call(path)
    evidence = json.loads(saved['checkpoint'])['evidence']
    assert len(evidence['links']) == 35 and len(evidence['events']) == 84
    assert evidence['money']['settled_volume'] == '198'
    assert evidence['all_scheduled_payments_settled']
    pending = call(path + '/drafts', dict(id=uid(), expected_generation=1,
        model_generation=1, unavailable_events=[83], basis='Declared future interruption'))
    return dict(path=path, saved=saved, pending=pending,
                draft_path=f"/v1/coupled-drafts/{pending['request']['id']}")


def resume_coupled_pending(call, reference):
    assert call(reference['path']) == reference['saved']
    assert call(reference['draft_path']) == reference['pending']
    approved = call(reference['draft_path'] + '/resolve',
        dict(apply=True, basis='Separate review after logical restore'))
    assert approved['candidate'] == reference['pending']['candidate']
    evidence = json.loads(approved['candidate']['checkpoint'])['evidence']
    assert not evidence['all_scheduled_payments_settled']
    assert evidence['money']['opening_total'] == '160'
    assert sum(map(int, evidence['money']['closing'])) == 160
    assert evidence['physical']['generation'] == 1


def resume_coupled_current(call, reference):
    draft = call(reference['path'] + '/drafts', dict(id=uid(), expected_generation=2,
        model_generation=3, unavailable_events=[83], basis='Advance with reviewed physical and dated history'))
    approved = call(f"/v1/coupled-drafts/{draft['request']['id']}/resolve",
        dict(apply=True, basis='Reviewed coupled continuation'))
    assert approved['candidate']['generation'] == 3
    evidence = json.loads(approved['candidate']['checkpoint'])['evidence']
    assert evidence['physical']['generation'] == 3
    assert sum(map(int, evidence['money']['closing'])) == 160
    assert call(reference['path'] + '/versions/1') == reference['saved']
    print('Coupled circulation: restart/restore, pending failure and updated physical anchors preserved', flush=True)



def populate_extended(call):
    fixture = json.loads((ROOT / 'fixtures/extended-scenario.json').read_text())
    definition = fixture['definition']
    definition['id'] = uid()
    path = '/v1/scenarios/models/' + definition['id']
    call('/v1/scenarios/models', definition)
    draft = call(path + '/drafts', dict(id=uid(), expected_generation=0,
        input=fixture['input'], basis='Integrated five-year fixture'))
    call('/v1/model-drafts/' + draft['request']['id'] + '/resolve',
         dict(apply=True, basis='Reviewed physical and dated food evidence'))
    cid = uid()
    cp = '/v1/scenarios/coupled/' + cid
    call('/v1/scenarios/coupled', dict(id=cid, format_version=1, model_id=definition['id'],
        policy=fixture['policy'], basis='Minimum opening for the declared schedule'))
    draft = call(cp + '/drafts', dict(id=uid(), expected_generation=0,
        model_generation=1, unavailable_events=[], basis='Reviewed exact circulation'))
    approved = call('/v1/coupled-drafts/' + draft['request']['id'] + '/resolve',
        dict(apply=True, basis='Separate operator review'))
    evidence = json.loads(approved['candidate']['checkpoint'])['evidence']
    assert len(evidence['links']) == 2795 and len(evidence['events']) == 5740
    assert evidence['money']['opening_total'] == fixture['reference']['money']['opening']
    assert evidence['all_scheduled_payments_settled']
    data = copy.deepcopy(fixture['input'])
    data['boundary'] = 30
    pending = call(path + '/drafts', dict(id=uid(), expected_generation=1, input=data,
        basis='Advance without rewriting production or original food age'))
    return dict(path=path, pending=pending, draft_path='/v1/model-drafts/' + pending['request']['id'],
        coupled_path=cp, coupled=approved['candidate'])


def resume_extended(call, reference):
    assert call(reference['coupled_path']) == reference['coupled']
    assert call(reference['draft_path']) == reference['pending']
    call(reference['draft_path'] + '/resolve', dict(apply=True, basis='Reviewed after logical restore'))
    draft = call(reference['coupled_path'] + '/drafts', dict(id=uid(), expected_generation=1,
        model_generation=2, unavailable_events=[], basis='Circulation with advanced physical boundary'))
    approved = call('/v1/coupled-drafts/' + draft['request']['id'] + '/resolve',
        dict(apply=True, basis='Reviewed after restore'))
    old = json.loads(reference['coupled']['checkpoint'])['evidence']
    new = json.loads(approved['candidate']['checkpoint'])['evidence']
    assert old['events'] == new['events'] and old['money'] == new['money']
    before = json.loads(old['physical']['checkpoint'])
    after = json.loads(new['physical']['checkpoint'])
    assert before['food_routes'] == after['food_routes']
    assert before['materials'] == after['materials']
    assert after['input']['boundary'] == 30
    assert call(reference['coupled_path'] + '/versions/1') == reference['coupled']
    print('Extended five-year scenario: food origins, packaging mass and exact circulation survive restart/restore', flush=True)
