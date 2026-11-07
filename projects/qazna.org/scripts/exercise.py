#!/usr/bin/env python3
"""Synthetic actors exercising the real API in an owned, disposable database.

No model calls, real identities, remote targets or existing databases. Actor age
and quantities are scenario metadata, not medical norms or distribution policy.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import hashlib
import os
from pathlib import Path
import random
import secrets
import signal
import socket
import subprocess
import tempfile
import threading
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, build_opener, HTTPRedirectHandler
import uuid

ROOT = Path(__file__).resolve().parents[1]
BASIS = "Synthetic exercise assumption; not a physiological norm"
PERIOD = "2026-11-03"
TREASURY = "00000000-0000-0000-0000-000000000002"


def uid():
    return str(uuid.uuid4())


def stop(process):
    if process and process.poll() is None:
        process.send_signal(signal.SIGINT)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


class Client:
    def __init__(self, base, tokens):
        self.base, self.tokens = base, tokens
        self.audit = None

    def request(self, path, body=None, key=None, role="operator", expected=200):
        headers = {"Authorization": "Bearer " + self.tokens.get(role, "invalid"), "Content-Type": "application/json"}
        if body is not None:
            headers["Idempotency-Key"] = key or uid()
        request = Request(self.base + path, None if body is None else json.dumps(body).encode(), headers)
        try:
            with build_opener(NoRedirect).open(request, timeout=6) as response:
                code, result = response.status, json.load(response)
        except HTTPError as error:
            code, result = error.code, error.read().decode()
        if self.audit:
            self.audit(dict(kind="http", path=path, role=role, method="GET" if body is None else "POST", idempotency_key=key, request=body, status=code, response=result))
        assert code == expected, (path, code, expected, result)
        return result


@contextmanager
def isolated_database():
    pg_dir = Path(os.environ.get("PG_BIN", "/opt/homebrew/opt/postgresql@16/bin"))
    def pg(name, *args):
        subprocess.run([str(pg_dir / name), *map(str, args)], check=True, stdout=subprocess.DEVNULL)
    with tempfile.TemporaryDirectory(prefix="qazna-exercise-") as directory:
        root = Path(directory)
        data, sock = root / "data", root / "socket"
        sock.mkdir(mode=0o700)
        started = False
        try:
            pg("initdb", "-D", data, "-U", "qazna", "--auth-local=trust", "--auth-host=reject", "--encoding=UTF8", "--locale=C")
            pg("pg_ctl", "-D", data, "-l", root / "postgres.log", "-o", f"-c listen_addresses='' -c unix_socket_directories='{sock}'", "-w", "start")
            started = True
            pg("createdb", "-h", sock, "-U", "qazna", "exercise_demo")
            url = "postgresql://qazna@localhost/exercise_demo?" + urlencode({"host": str(sock)})
            env = dict(os.environ, DATABASE_URL=url)
            subprocess.run([str(ROOT / "target/debug/qazna"), "migrate"], env=env, check=True, stdout=subprocess.DEVNULL)
            pg("psql", "-h", sock, "-U", "qazna", "-d", "exercise_demo", "-v", "ON_ERROR_STOP=1", "-f", ROOT / "fixtures/demo.sql")
            yield url
        finally:
            if started:
                pg("pg_ctl", "-D", data, "-m", "fast", "-w", "stop")


# Embedded and offline: no third-party JS, CDN, tokens, account identities or model access.
HTML = '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Qazna exercise observatory</title>
<style>body{font:16px system-ui;background:#101b28;color:#e9f1f7;max-width:1100px;margin:32px auto;padding:0 20px}h1{font-size:30px}section{padding:18px;background:#1b2b3d;border-radius:12px;margin:18px 0}table{width:100%;border-collapse:collapse}td,th{text-align:left;border-bottom:1px solid #456;padding:9px}small{color:#adc4d9}.ok{color:#7fe0bb}.warn{color:#ffcb78}svg{width:100%;height:180px}code{overflow-wrap:anywhere}</style>
<h1>Qazna · exercise observatory</h1><p>Synthetic residents and provisioning units · deterministic control · operator-approved actions</p><div id="state"></div><section><h2>Provisioning feedback</h2><small>Requested and received quantities, grouped by dimension. No sum across meals, litres and energy.</small><div id="needs"></div></section><section><h2>Coordination queue</h2><svg id="chart" viewBox="0 0 1000 180" role="img" aria-label="Pending coordination jobs over time"></svg><div id="queue"></div></section><section><h2>Exercise evidence</h2><div id="cases"></div></section><p id="age"></p><small>Refreshes every second while served by the exercise runner. A saved report is a historical snapshot. This is neither a penetration test nor proof of production readiness.</small>
<script>const initial=__REPORT__;let lastContact=Date.now();const e=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function table(head,rows){return '<table><tr>'+head.map(x=>'<th>'+e(x)+'</th>').join('')+'</tr>'+rows.map(r=>'<tr>'+r.map(x=>'<td>'+e(x)+'</td>').join('')+'</tr>').join('')+'</table>'}
function render(r){const s=r.samples.at(-1);document.getElementById('state').innerHTML='<section><b class="'+(r.unexpected_failures.length?'warn':'ok')+'">'+e(r.state)+' · '+r.samples.length+' samples</b><p>Readiness: '+e(r.readiness)+'</p><p>'+e(s ? s.alerts.join(', ')||'No operational alerts at latest sample' : 'Waiting for first sample')+'</p><small>Seed '+r.seed+' · '+r.actors.length+' synthetic residents · '+e(r.tick_seconds)+' second target cadence</small></section>';document.getElementById('needs').innerHTML=table(['Period','Category','Unit','Requested','Reserved','Received','Unmet'],(s?.needs||[]).map(n=>[n.period,n.category,n.unit,n.requested,n.reserved,n.received,n.unmet]));document.getElementById('cases').innerHTML=table(['Scenario','Outcome','Evidence'],r.cases.map(c=>[c.name,c.outcome,c.evidence]));const values=r.samples.map(x=>Number(x.coordination.queued)+Number(x.coordination.running));const max=Math.max(1,...values);document.getElementById('chart').innerHTML='<polyline fill="none" stroke="#7fe0bb" stroke-width="3" points="'+values.map((v,i)=>(20+i*960/Math.max(1,values.length-1))+','+(160-v*140/max)).join(' ')+'"/><text x="20" y="18" fill="#adc4d9">Peak pending jobs: '+max+'</text>';document.getElementById('queue').textContent=s ? JSON.stringify(s.coordination):'';document.getElementById('age').textContent='Latest database observation: '+(s?.observed_at||'none')+' · local read '+new Date().toLocaleTimeString();}
render(initial);if(location.protocol!=='file:')setInterval(async()=>{try{let x=await fetch('/report.json',{cache:'no-store'});if(!x.ok)throw Error();render(await x.json());lastContact=Date.now()}catch{document.getElementById('age').textContent='TELEMETRY UNAVAILABLE · last contact '+Math.round((Date.now()-lastContact)/1000)+' seconds ago'}},1000);</script></html>'''


def save_report(out, report):
    data = json.dumps(report, indent=2)
    tmp = out / "report.tmp"
    tmp.write_text(data)
    tmp.replace(out / "report.json")
    (out / "report.html").write_text(HTML.replace("__REPORT__", data.replace("<", "\\u003c")))


def serve_dashboard(out):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            file = {"/": "report.html", "/report.json": "report.json"}.get(self.path)
            if not file:
                self.send_error(404)
                return
            data = (out / file).read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "application/json" if file.endswith("json") else "text/html; charset=utf-8")
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(data)
        def log_message(self, *args):
            pass
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"Exercise dashboard: http://127.0.0.1:{server.server_port}", flush=True)
    return server


def run_exercise(database_url, out, seed=7, tick=1.0, steps=40, dashboard=False):
    """Internal test harness; caller owns the empty disposable demo database."""
    out.mkdir(parents=True, exist_ok=False)
    rng = random.Random(seed)
    tokens = {role: secrets.token_hex(32) for role in ("operator", "observer", "reviewer")}
    env = dict(os.environ, DATABASE_URL=database_url, QAZNA_OPERATOR_TOKEN=tokens['operator'], QAZNA_OBSERVER_TOKEN=tokens['observer'], QAZNA_REVIEW_TOKEN=tokens['reviewer'])
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env['QAZNA_BIND'] = f"127.0.0.1:{port}"
    api = Client(f"http://127.0.0.1:{port}", tokens)
    binary = str(ROOT / "target/debug/qazna")
    report = dict(seed=seed, tick_seconds=tick, state="starting", readiness="known_gaps_remain", actors=[], cases=[], samples=[], unexpected_failures=[], model_calls=0)
    report['provenance'] = {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in [('binary_sha256', Path(binary)), ('harness_sha256', Path(__file__))]}
    save_report(out, report)
    server = serve_dashboard(out) if dashboard else None
    service = worker = None
    started = time.monotonic()
    log = (out / "service.log").open('w')
    events = (out / "events.jsonl").open('w')
    audit_lock = threading.Lock()
    def audit(event):
        event['elapsed_seconds'] = round(time.monotonic()-started,3)
        with audit_lock:
            events.write(json.dumps(event)+'\n');events.flush()
    api.audit = audit
    def evidence(name, outcome, detail):
        event = dict(elapsed_seconds=round(time.monotonic()-started, 3), name=name, outcome=outcome, evidence=detail)
        report['cases'].append(event)
        audit(event)
    def call(path, body=None, **kw):
        return api.request(path, body, **kw)
    def process(command):
        return subprocess.Popen([binary, command], env=env, stdout=log, stderr=log)
    def worker_once():
        subprocess.run([binary, 'coordinate-once'], env=env, stdout=log, stderr=log, check=True, timeout=20)
    decision = {'basis': BASIS}
    try:
        service = process('serve')
        deadline = time.monotonic()+10
        while True:
            try:
                call('/v1/system/status', role='observer'); break
            except (URLError, ConnectionError):
                assert service.poll() is None and time.monotonic()<deadline, 'service did not start'
                time.sleep(.05)
        unit, producer, capacity, recipe = (uid() for _ in range(4))
        call('/v1/accounts', {'id':producer,'kind':'producer'}, expected=201)
        call('/v1/units', dict(id=unit,parent=None,name='Synthetic provisioning centre',function='Food, water and energy provision',basis=BASIS))
        call(f'/v1/units/{unit}/providers', dict(producer=producer,basis=BASIS))
        stocks={}
        for category, dimension, quantity in [('food','meal','6'),('water','litre','30'),('energy','kwh','20'),('food','grain_kg','50')]:
            rid=uid(); stocks[dimension]=rid
            call('/v1/resources',dict(id=rid,producer=producer,category=category,unit=dimension,quantity=quantity,unit_measure='5' if dimension=='meal' else '1',basis=BASIS))
        call('/v1/production/capacities',dict(id=capacity,unit=unit,period=PERIOD,capacity_unit='kitchen_minute',quantity='40',basis=BASIS))
        call('/v1/production/recipes',dict(id=recipe,unit=unit,name='Synthetic cooking',capacity_unit='kitchen_minute',capacity_per_batch='2',output_category='food',output_unit='meal',output_per_batch='3',inputs=[dict(slot='grain',category='food',unit='grain_kg',quantity_per_batch='2')],basis=BASIS))
        options=[]
        for age in [8,16,30,45,70,85]:
            resident=uid(); call('/v1/accounts',dict(id=resident,kind='resident'),expected=201)
            call('/v1/allocations',dict(treasury=TREASURY,resident=resident,period=PERIOD,category='food',amount='100',basis=BASIS))
            actor=dict(age=age,role='synthetic_resident',needs=[]);report['actors'].append(actor)
            for category, dimension in [('food','meal'),('water','litre'),('energy','kwh')]:
                nid=uid();quantity=str(rng.randint(2,4))
                call('/v1/needs',dict(id=nid,resident=resident,assigned_unit=unit,period=PERIOD,category=category,unit=dimension,quantity=quantity,basis=BASIS))
                actor['needs'].append(dict(category=category,unit=dimension,quantity=quantity))
                options.append(dict(need=nid,resources=[stocks[dimension]],production=[dict(recipe=recipe,capacity=capacity,inputs=[dict(slot='grain',resource=stocks['grain_kg'])])] if dimension=='meal' else []))
        plan=dict(period=PERIOD,basis=BASIS,needs=options)
        worker_once()  # Drain setup events before enrolling the monitor.
        monitor=uid();call('/v1/coordination/monitors',dict(id=monitor,plan=plan))
        worker_once()
        calculation=call(f'/v1/coordination/monitors/{monitor}')['latest']
        (out/'calculation.json').write_text(json.dumps(calculation,indent=2))
        # Seed the review gate with a fabricated fact and an unauthorized approval claim.
        review=dict(id=uid(),model='synthetic-adversary',report=dict(summary='Forged claim',findings=[dict(signal='physical_residual',subject=options[0]['need'],quantity='999',explanation='Invented')],questions=[],requires_operator_approval=True))
        reviewpath=f"/v1/coordination/results/{calculation['job']}/reviews"
        call(reviewpath,review,role='reviewer',expected=400)
        review['report']['findings']=[];review['report']['requires_operator_approval']=False
        call(reviewpath,review,role='reviewer',expected=400)
        review['report']['requires_operator_approval']=True
        review['report']['summary']='Synthetic control: no economic action authorized'
        accepted=call(reviewpath,review,role='reviewer')
        assert accepted['id']==review['id']
        assert len(call(reviewpath,role='reviewer'))==1
        evidence('fabricated_AI_evidence','prevented','Invented quantity and approval=false both rejected with HTTP 400')
        for role in ('observer','reviewer','invalid'):
            call('/v1/transfers',{},role=role,expected=401 if role=='invalid' else 403)
        call(f"/v1/planning/drafts/{calculation['draft']}/apply",decision,role='reviewer',expected=403)
        evidence('credential_privilege_escalation','prevented','Observer/reviewer cannot transfer; reviewer cannot apply a draft; invalid key rejected')
        resident=call('/v1/needs/'+options[0]['need'])['resident']
        transfer=dict(from_=TREASURY,to=resident,amount='1');transfer['from']=transfer.pop('from_')
        first=call('/v1/transfers',transfer,key='exercise-replay')
        assert call('/v1/transfers',transfer,key='exercise-replay')==first
        transfer['amount']='2';call('/v1/transfers',transfer,key='exercise-replay',expected=409)
        transfer['amount']='9999';call('/v1/transfers',transfer,expected=409)
        transfer['amount']='9223372036854775808';call('/v1/transfers',transfer,expected=422)
        evidence('replay_overdraft_overflow','prevented','Replay returns same receipt; altered replay, overdraft and out-of-range amount rejected')
        # Fresh draft after transfer; old reviewed state must not be silently applied.
        call(f"/v1/planning/drafts/{calculation['draft']}/apply",decision,expected=409)
        evidence('stale_operator_plan','prevented','Changed balance invalidates old reviewed proposal')
        draft=call('/v1/planning/drafts',dict(id=uid(),plan=plan))
        applied=call(f"/v1/planning/drafts/{draft['id']}/apply",decision)
        reservations=[a['reservation'] for a in applied['actions'] if a['reservation']]
        productions=[a['production_plan'] for a in applied['actions'] if a['production_plan']]
        next_plan=call('/v1/planning/assess',plan)
        assert all(not n['production'] for n in next_plan['needs'])
        assert any(int(n['committed_production'])>0 for n in next_plan['needs'])
        evidence('repeat_production','prevented','Active need-linked production counted as forecast; no duplicate order suggested')
        # Concurrent requests compete for exactly one synthetic unit.
        race=uid();call('/v1/resources',dict(id=race,producer=producer,category='water',unit='litre',quantity='1',unit_measure='1',basis=BASIS))
        def compete(_):
            try:
                return call('/v1/reservations',dict(resident=resident,resource=race,quantity='1',basis=BASIS))
            except AssertionError as error:
                assert error.args[0][1:3]==(409,200), error
                return None
        with ThreadPoolExecutor(max_workers=8) as pool:
            winners=[x for x in pool.map(compete,range(8)) if x]
        assert len(winners)==1
        call(f"/v1/reservations/{winners[0]['id']}/cancel",decision)
        evidence('concurrent_double_reservation','prevented','8 simultaneous requests; exactly 1 hold; cancellation releases it')
        # Explicit truth oracle: the application cannot establish physical existence.
        fake=uid();call('/v1/resources',dict(id=fake,producer=producer,category='food',unit='meal',quantity='100',unit_measure='1',basis='Synthetic dishonest operator declaration'))
        assert call(f'/v1/resources/{fake}')['on_hand']=='100'
        evidence('false_physical_declaration','known_gap','Private scenario truth=0, API declaration=100 accepted by trusted operator. Not detected as fraud by Qazna; excluded from planning candidates.')
        worker=process('coordinate')
        report['state']='running'
        outputs=[]
        for step in range(steps):
            tick_start=time.monotonic()
            if step==2:
                for index,pid in enumerate(productions):
                    if index==0:
                        call(f'/v1/production/plans/{pid}/cancel',decision)
                    else:
                        p=call(f'/v1/production/plans/{pid}');quantity=int(p['planned_output'])-1
                        output=uid();outputs.append(output)
                        completion=dict(output=dict(id=output,producer=producer,quantity=str(quantity),unit_measure='5'),losses=[dict(slot='grain',quantity='1')],basis=BASIS,shortfall_basis='Synthetic damaged portion')
                        call(f'/v1/production/plans/{pid}/complete',completion)
                        call(f'/v1/production/plans/{pid}/complete',completion,expected=409)
                evidence('production_loss_and_cancellation','recorded','One plan cancelled; other cycles completed with shortfall and input loss; second completion rejected')
            if step==4:
                for rid in reservations:
                    call(f'/v1/reservations/{rid}/settle',decision)
                for option in options:
                    if option['production']:
                        option['resources'].extend(outputs)
                d=call('/v1/planning/drafts',dict(id=uid(),plan=plan))
                a=call(f"/v1/planning/drafts/{d['id']}/apply",decision)
                for action in a['actions']:
                    if action['reservation']:
                        call(f"/v1/reservations/{action['reservation']}/settle",decision)
                evidence('receipt_after_output','recorded','Stock delivery settles; future production remains separate from received quantities')
            if step==8:
                stop(worker);worker=None
                call('/v1/transfers',{'from':TREASURY,'to':resident,'amount':'1'})
                evidence('worker_outage','injected','Worker stopped; economic API remains available; commands persist')
            if step==15:
                worker=process('coordinate')
                evidence('worker_restart','recovering','Worker restarted against same disposable database')
            sample=call('/v1/system/status',role='observer')
            sample['elapsed_seconds']=round(time.monotonic()-started,3)
            sample['tick']=step
            assert sample['invariants_passed'], sample['invariants']
            report['samples'].append(sample)
            audit(dict(kind='snapshot',**sample))
            save_report(out,report)
            time.sleep(max(0,tick-(time.monotonic()-tick_start)))
        # Drain remaining jobs with the actual worker algorithm, bounded by wall time.
        stop(worker);worker=None
        # A stopped worker can leave a valid 30-second lease. Recovery needs
        # additional time to reclaim it, calculate and drain the queued work.
        # Keep the production lease unchanged and bound the whole exercise.
        deadline=time.monotonic()+90
        while True:
            status=call('/v1/system/status',role='observer')
            queue=status['coordination']
            if all(queue[k]=='0' for k in ('pending_events','queued','running')):
                break
            assert time.monotonic()<deadline, f'coordination did not recover within 90s: {queue}'
            worker_once()
            if queue['running']!='0':
                time.sleep(.1)  # Avoid repeatedly spawning while a lease expires.
        assert queue['failed']=='0'
        report['samples'].append(status)
        evidence('durable_queue_recovery','detected_and_recovered','Pending events and jobs drained to zero; no failed calculations')
        if tick>=1:
            assert any('coordination_backlog_older_than_5s' in s['alerts'] for s in report['samples'][9:15])
            evidence('outage_visibility','detected','Observer reported backlog older than five seconds during stopped worker interval')
        else:
            evidence('outage_visibility','not_exercised','Accelerated CI run is shorter than the real five-second alert threshold')
        report['state']='completed'
        evidence('snapshot_invariants','passed',f"{len(report['samples'])} consistent snapshots; zero ledger, stock, capacity or need violations")
    except Exception as error:
        report['state']='failed';report['unexpected_failures'].append(str(error))
        raise
    finally:
        stop(worker);stop(service)
        report['duration_seconds']=round(time.monotonic()-started,3)
        save_report(out,report)
        if server:
            server.shutdown();server.server_close()
        events.close();log.close()
    print(f"Exercise {seed}: {report['state']}; {len(report['samples'])} snapshots; known physical evidence gap remains. Report: {out/'report.html'}",flush=True)
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seed',type=int,default=7)
    parser.add_argument('--tick',type=float,default=1.0)
    parser.add_argument('--steps',type=int,default=40)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--dashboard',action='store_true')
    args=parser.parse_args()
    if not .02<=args.tick<=5 or not 20<=args.steps<=3600:
        parser.error('tick must be .02..5 seconds; steps must be 20..3600')
    out=args.output or ROOT/'.local'/'exercises'/f"{time.strftime('%Y%m%d-%H%M%S')}-{args.seed}"
    with isolated_database() as url:
        run_exercise(url,out,args.seed,args.tick,args.steps,args.dashboard)


if __name__=='__main__':
    main()
