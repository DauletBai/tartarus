#!/usr/bin/env python3
"""Run credential-separated evidence checks against the real local HTTP server."""
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import time
from urllib.error import URLError
from exercise import ROOT, Client, isolated_database, stop, uid


def run_evidence_exercise(url, out, activate=True):
    out.mkdir(parents=True,exist_ok=False)
    tokens={role:secrets.token_hex(32) for role in ['operator','inspector_a','inspector_b','custodian','recipient']}
    actors={role:uid() for role in tokens if role!='operator'}
    env=dict(os.environ,DATABASE_URL=url,QAZNA_OPERATOR_TOKEN=tokens['operator'],QAZNA_ATTESTORS_JSON=json.dumps([dict(principal=actors[r],token=tokens[r]) for r in actors]),QAZNA_EVIDENCE_BASIS='Synthetic strict evidence exercise')
    env.pop('QAZNA_OBSERVER_TOKEN',None);env.pop('QAZNA_REVIEW_TOKEN',None)
    binary=str(ROOT/'target/debug/qazna')
    if activate:
        subprocess.run([binary,'enforce-evidence'],env=env,check=True,stdout=subprocess.DEVNULL)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    env['QAZNA_BIND']=f'127.0.0.1:{port}'
    api=Client(f'http://127.0.0.1:{port}',tokens)
    report={'synthetic':True,'enforcement':True,'cases':[],'physical_truth_proven':False}
    log=(out/'events.jsonl').open('w')
    api.audit=lambda event:log.write(json.dumps(event)+'\n')
    process=subprocess.Popen([binary,'serve'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    def case(name):report['cases'].append({'name':name,'result':'passed'})
    def call(path,body=None,**kw):return api.request(path,body,**kw)
    def statement(q,decision='confirm'):return dict(id=uid(),quantity=q,decision=decision,evidence_sha256='a'*64,basis='Synthetic evidence file digest')
    try:
        deadline=time.monotonic()+10
        while True:
            try:call('/v1/system/status');break
            except URLError:
                assert process.poll() is None and time.monotonic()<deadline
                time.sleep(.05)
        accounts={role:uid() for role in actors}
        for role,account in accounts.items():
            call('/v1/accounts',dict(id=account,kind='producer' if role=='custodian' else 'resident'),expected=201)
            call('/v1/evidence/principals',dict(id=actors[role],account=account,role='inspector' if role.startswith('inspector') else role,basis='Synthetic enrollment'))
        assert call('/v1/accounts/'+accounts['recipient'])['balance']=='0'
        def lot(quantity):
            rid=uid();call('/v1/resources',dict(id=rid,producer=accounts['custodian'],category='food',unit='meal',quantity=quantity,unit_measure='2',basis='Synthetic declaration'));return rid
        fake=lot('100')
        assert call('/v1/resources/'+fake)['available']=='0'
        call('/v1/reservations',dict(resident=accounts['recipient'],resource=fake,quantity='1',basis='Attempt to use nonexistent goods'),expected=409)
        call('/v1/evidence/resource/'+fake,statement('100','dispute'),role='inspector_a')
        case('false_unattested_declaration_cannot_be_used')
        rid=lot('10');path='/v1/evidence/resource/'+rid
        call(path,statement('10'),expected=403)
        call(path,statement('10'),role='custodian',expected=400)
        call(path,statement('9'),role='inspector_a',expected=400)
        case('operator_self_and_wrong_quantity_attestation_rejected')
        call(path,statement('10'),role='inspector_a')
        assert call('/v1/resources/'+rid)['available']=='0'
        call(path,statement('10'),role='inspector_b')
        assert call('/v1/resources/'+rid)['available']=='10'
        case('two_distinct_inspectors_release_lot')
        # Shadow-pilot policy: fund the exact shortfall from existing treasury
        # measure after physical availability is established; never mint money.
        reserve=dict(resident=accounts['recipient'],resource=rid,quantity='2',basis='Synthetic basic provision')
        call('/v1/reservations',reserve,expected=409)
        treasury='00000000-0000-0000-0000-000000000002'
        before=call('/v1/accounts/'+treasury)['balance']
        allocation=dict(treasury=treasury,resident=accounts['recipient'],period='2026-11-04',category='food',amount='10000',basis='Synthetic reserve insufficiency')
        call('/v1/allocations',allocation,expected=409)
        assert call('/v1/accounts/'+treasury)['balance']==before
        assert call('/v1/accounts/'+accounts['recipient'])['balance']=='0'
        allocation['amount']='4';allocation['basis']='Shadow pilot: exact food shortfall from existing reserve'
        call('/v1/allocations',allocation)
        assert int(call('/v1/accounts/'+treasury)['balance'])==int(before)-4
        case('zero_balance_basic_provision_uses_existing_reserve_without_issuance')
        held=call('/v1/reservations',dict(resident=accounts['recipient'],resource=rid,quantity='2',basis='Synthetic delivery'))
        delivery='/v1/evidence/delivery/'+held['id'];settle='/v1/reservations/'+held['id']+'/settle'
        call(settle,dict(basis='No receipt yet'),expected=409)
        call(delivery,statement('2'),role='inspector_a',expected=400)
        call(delivery,statement('2'),role='custodian')
        call(settle,dict(basis='Only sender confirmed'),expected=409)
        call(delivery,statement('2'),role='recipient')
        call(settle,dict(basis='Operator accepts both confirmations'))
        assert call('/v1/accounts/'+accounts['recipient'])['balance']=='0'
        case('custodian_and_actual_recipient_required_before_settlement')
        call(path,statement('10','dispute'),role='inspector_a')
        assert call('/v1/resources/'+rid)['available']=='0'
        case('later_dispute_quarantines_remaining_stock_without_rewriting_receipt')
        report['status']=call('/v1/system/status');assert report['status']['invariants_passed']
        report['result']='passed'
    except Exception as error:
        report['result']='failed';report['error']=str(error);raise
    finally:
        stop(process);log.close();(out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f"Strict evidence exercise passed: {len(report['cases'])} cases; {out}",flush=True)

if __name__=='__main__':
    with isolated_database() as url:
        run_evidence_exercise(url,ROOT/'.local'/'exercises'/('v010-evidence-'+time.strftime('%Y%m%d-%H%M%S')))
