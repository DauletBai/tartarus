#!/usr/bin/env python3
"""Multi-period strict-evidence HTTP exercise in an owned disposable database.

One wall-clock tick represents one synthetic day, not one day of real observation.
Finite genesis, no issuance or invented replenishment. All actors are simulated.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import date, timedelta
import hashlib
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import threading
import time
from urllib.error import URLError
from exercise import ROOT, Client, isolated_database, stop, uid, TREASURY

BASIS = 'Synthetic multi-period research; not an adopted economic rule'


def run(url, out, steps=180, tick=1.0):
    if not __debug__:
        raise RuntimeError('Assertions must remain enabled')
    out.mkdir(parents=True, exist_ok=False)
    roles = ['operator', 'inspector_a', 'inspector_b', 'custodian'] + [f'recipient_{i}' for i in range(6)]
    tokens = {r: secrets.token_hex(32) for r in roles}
    principals = {r: uid() for r in roles if r != 'operator'}
    accounts = {r: uid() for r in principals}
    env = dict(os.environ, DATABASE_URL=url, QAZNA_OPERATOR_TOKEN=tokens['operator'],
               QAZNA_ATTESTORS_JSON=json.dumps([dict(principal=principals[r], token=tokens[r]) for r in principals]),
               QAZNA_EVIDENCE_BASIS=BASIS)
    env.pop('QAZNA_OBSERVER_TOKEN', None)
    env.pop('QAZNA_REVIEW_TOKEN', None)
    binary = ROOT / 'target/debug/qazna'
    subprocess.run([str(binary), 'enforce-evidence'], env=env, check=True, stdout=subprocess.DEVNULL)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
    env['QAZNA_BIND'] = f'127.0.0.1:{port}'
    client = Client(f'http://127.0.0.1:{port}', tokens)
    log = (out / 'events.jsonl').open('w')
    service_log = (out / 'service.log').open('w')
    started = time.monotonic()
    report = dict(state='starting', synthetic=True, steps_requested=steps, tick_seconds=tick,
                  model_calls=0, periods=[], restarts=0, concurrent_replays=0,
                  first_accounting_shortage=None, first_physical_shortage=None,
                  provenance={'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                              'harness_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
                  limitations=['Synthetic attestations do not prove independent physical delivery.',
                               'No new monetary policy, demographic realism or national load claim.',
                               'No background worker in this drill; worker recovery has a separate exercise.'])
    lock = threading.Lock()
    count = 0
    def audit(event):
        nonlocal count
        event['elapsed_seconds'] = round(time.monotonic()-started, 6)
        with lock:
            count += 1; log.write(json.dumps(event)+'\n'); log.flush()
    client.audit = audit
    def call(path, body=None, **kw):
        return client.request(path, body, **kw)
    process = None
    def start():
        proc = subprocess.Popen([str(binary), 'serve'], env=env, stdout=service_log, stderr=service_log)
        deadline = time.monotonic()+10
        while True:
            try:
                call('/v1/system/status'); return proc
            except (URLError, ConnectionError):
                if proc.poll() is not None or time.monotonic() >= deadline:
                    stop(proc); raise RuntimeError('Local service failed to start')
                time.sleep(.05)
    def attest(kind, ident, quantity, role):
        payload = f'{kind}:{ident}:{quantity}:{role}'.encode()
        call(f'/v1/evidence/{kind}/{ident}', dict(id=uid(), quantity=str(quantity), decision='confirm',
             evidence_sha256=hashlib.sha256(payload).hexdigest(), basis=BASIS), role=role)
    def certify(ident, quantity):
        for role in ['inspector_a', 'inspector_b']:
            attest('resource', ident, quantity, role)
    expected_treasury = 1000
    expected_provider = 0
    total_received = 0
    try:
        process = start()
        for role, account in accounts.items():
            call('/v1/accounts', dict(id=account, kind='producer' if role=='custodian' else 'resident'), expected=201)
            call('/v1/evidence/principals', dict(id=principals[role], account=account,
                 role='inspector' if role.startswith('inspector') else 'recipient' if role.startswith('recipient') else role, basis=BASIS))
        unit, recipe = uid(), uid()
        call('/v1/units', dict(id=unit, parent=None, name='Synthetic kitchen', function='Food provisioning', basis=BASIS))
        call(f'/v1/units/{unit}/providers', dict(producer=accounts['custodian'], basis=BASIS))
        call('/v1/production/recipes', dict(id=recipe, unit=unit, name='Synthetic six-meal batch',
             capacity_unit='batch', capacity_per_batch='1', output_category='food', output_unit='meal', output_per_batch='18',
             inputs=[dict(slot='grain', category='food', unit='grain_kg', quantity_per_batch='36')], basis=BASIS))
        for day in range(steps):
            began = time.monotonic()
            period = str(date(2026, 11, 6)+timedelta(days=day))
            if day and day % 30 == 0:
                stop(process); process = start(); report['restarts'] += 1
            grain, capacity, output = uid(), uid(), uid()
            call('/v1/resources', dict(id=grain, producer=accounts['custodian'], category='food', unit='grain_kg', quantity='36', unit_measure='1', basis=BASIS))
            certify(grain, 36)
            call('/v1/production/capacities', dict(id=capacity, unit=unit, period=period, capacity_unit='batch', quantity='1', basis=BASIS))
            production = call('/v1/production/plans', dict(recipe=recipe, capacity=capacity, batches='1', inputs=[dict(slot='grain', resource=grain)], basis=BASIS))
            pid = production['plan_id']; assert pid
            quantity = 15 if day % 17 == 11 else 18
            body = dict(output=dict(id=output, producer=accounts['custodian'], quantity=str(quantity), unit_measure='1'), losses=[], basis=BASIS,
                        shortfall_basis='Synthetic output damage' if quantity<18 else None)
            complete_key = f'complete-{day}'
            complete = call(f'/v1/production/plans/{pid}/complete', body, key=complete_key)
            assert call(f'/v1/production/plans/{pid}/complete', body, key=complete_key)==complete
            assert call('/v1/resources/'+output)['available']=='0'
            certify(output, quantity)
            assert call('/v1/resources/'+grain)['on_hand']=='0'
            assert call(f'/v1/production/capacities/{capacity}')['consumed']=='1'
            left = quantity
            received_today = 0
            for i in range(6):
                role = f'recipient_{i}'; resident = accounts[role]; need = uid()
                call('/v1/needs', dict(id=need, resident=resident, assigned_unit=unit, period=period, category='food', unit='meal', quantity='3', basis=BASIS))
                if left < 3:
                    if report['first_physical_shortage'] is None: report['first_physical_shortage'] = day
                    attempt = call(f'/v1/needs/{need}/reserve', dict(resource=output, quantity='3', basis=BASIS))
                    assert attempt['reservation_id'] is None
                    assert call('/v1/needs/'+need)['unmet']=='3'
                    continue
                allocation = dict(treasury=TREASURY, resident=resident, period=period, category='food', amount='3', basis=BASIS)
                if expected_treasury < 3:
                    call('/v1/allocations', allocation, expected=409)
                    if report['first_accounting_shortage'] is None: report['first_accounting_shortage'] = day
                    attempt = call(f'/v1/needs/{need}/reserve', dict(resource=output, quantity='3', basis=BASIS))
                    assert attempt['reservation_id'] is None
                    assert call('/v1/needs/'+need)['unmet']=='3'
                    continue
                funding_key = f'fund-{day}-{i}'
                funding = call('/v1/allocations', allocation, key=funding_key)
                assert call('/v1/allocations', allocation, key=funding_key)==funding
                expected_treasury -= 3
                reserve = dict(resource=output, quantity='3', basis=BASIS)
                key = f'reserve-{day}-{i}'
                if i==0 and day % 10 == 0:
                    with ThreadPoolExecutor(max_workers=4) as pool:
                        attempts = list(pool.map(lambda _:call(f'/v1/needs/{need}/reserve', reserve, key=key), range(4)))
                    assert all(a==attempts[0] for a in attempts)
                    held = attempts[0]; report['concurrent_replays'] += 1
                else:
                    held = call(f'/v1/needs/{need}/reserve', reserve, key=key)
                rid = held['reservation_id']; assert rid
                if i==0:
                    call(f'/v1/reservations/{rid}/settle', dict(basis=BASIS), expected=409)
                    assert call('/v1/needs/'+need)['received']=='0'
                attest('delivery', rid, 3, 'custodian'); attest('delivery', rid, 3, role)
                settle_key = f'settle-{day}-{i}'
                settlement = call(f'/v1/reservations/{rid}/settle', dict(basis=BASIS), key=settle_key)
                assert call(f'/v1/reservations/{rid}/settle', dict(basis=BASIS), key=settle_key)==settlement
                assert call('/v1/needs/'+need)['received']=='3'
                assert call('/v1/accounts/'+resident)['balance']=='0'
                received_today += 3; total_received += 3; left -= 3; expected_provider += 3
            treasury = int(call('/v1/accounts/'+TREASURY)['balance'])
            provider = int(call('/v1/accounts/'+accounts['custodian'])['balance'])
            assert treasury==expected_treasury and provider==expected_provider
            assert treasury+provider==1000 and provider==total_received
            assert int(call('/v1/resources/'+output)['on_hand'])==left
            sample = call('/v1/system/status'); assert sample['invariants_passed']
            report['periods'].append(dict(day=day, period=period, produced=quantity, requested=18,
                 received=received_today, unmet=18-received_today, treasury=treasury, provider=provider,
                 invariants_passed=True, elapsed_seconds=round(time.monotonic()-started,3)))
            report['state']='running'
            tmp = out/'report.tmp'; tmp.write_text(json.dumps(report,indent=2)+'\n'); tmp.replace(out/'report.json')
            time.sleep(max(0,tick-(time.monotonic()-began)))
        if steps>=60:
            assert report['first_accounting_shortage'] is not None
            assert expected_treasury==1 and expected_provider==999
        report['state']='passed'
        report['conclusion']='Exact state survived repeated production, strict evidence, retries and restarts. Finite treasury depletion is an expected economic counterexample, not a software failure.'
    except Exception as error:
        report['state']='failed'; report['error']=repr(error); raise
    finally:
        stop(process); log.close(); service_log.close()
        report['duration_seconds']=round(time.monotonic()-started,3)
        report['http_requests']=count
        report['events_sha256']=hashlib.sha256((out/'events.jsonl').read_bytes()).hexdigest()
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:report[k] for k in ['state','duration_seconds','http_requests','restarts','first_accounting_shortage','first_physical_shortage']}),flush=True)
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--steps',type=int,default=180)
    parser.add_argument('--tick',type=float,default=1.0)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if not 2<=args.steps<=1825 or not .02<=args.tick<=5:
        parser.error('steps=2..1825; tick=.02..5')
    with isolated_database() as url:
        run(url,args.output,args.steps,args.tick)

if __name__=='__main__':main()
