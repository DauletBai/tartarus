"""Bounded public-page acquisition: all selected MSL14 pp26–28 Ea records."""
from pathlib import Path
import subprocess,json,concurrent.futures
R=Path(__file__).resolve().parents[1];S=R/'round29_sources'
selected=json.loads((R/'data/round29_lexical_target_catalogue.json').read_text())
def get(m):
 pid=m['pid'];p=S/(pid+'_Ea.html');url='https://oracc.museum.upenn.edu/dcclt/'+pid
 r=subprocess.run(['curl','-sS','--fail','-L','--max-time','25',url,'-o',str(p)],capture_output=True,text=True)
 return dict(pid=pid,file=p.name,url=url,returncode=r.returncode,bytes=p.stat().st_size if p.exists() else 0,error=r.stderr[:250])
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as e:log=list(e.map(get,selected))
(S/'lexical_fetch_log.json').write_text(json.dumps(log,ensure_ascii=False,indent=2));print(json.dumps(dict(selected=len(selected),nonempty=sum(x['bytes']>0 for x in log),failed=[x for x in log if x['error'] or not x['bytes']]),ensure_ascii=False,indent=2))
