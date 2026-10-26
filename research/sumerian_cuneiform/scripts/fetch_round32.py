from pathlib import Path
import subprocess,json,concurrent.futures
R=Path(__file__).resolve().parents[1];S=R/'round32_sources';S.mkdir(exist_ok=True)
items={
'pad_find_entry.html':'https://oracc.museum.upenn.edu/epsd2/cbd/sux/o0036618.html',
'pad_find_lexical.html':'https://oracc.museum.upenn.edu/dcclt/o0036618',
'UD_lexical.html':'https://oracc.museum.upenn.edu/osl/signlist/selpages/o0000565-lex.html',
'pad_find_praxis.html':'https://oracc.museum.upenn.edu/epsd2/praxis/cbd/sux/o0036618.html'}
def get(x):
 n,u=x;p=S/n;r=subprocess.run(['curl','-sS','--fail','-L','--max-time','30',u,'-o',str(p)],text=True,capture_output=True)
 return dict(file=n,url=u,returncode=r.returncode,bytes=p.stat().st_size if p.exists() else 0,error=r.stderr[:200])
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:a=list(ex.map(get,items.items()))
(S/'fetch_log.json').write_text(json.dumps(a,indent=2));print(json.dumps(a,indent=2))
