from pathlib import Path
import subprocess,json,concurrent.futures
R=Path(__file__).resolve().parents[1];S=R/'round30_sources';S.mkdir(exist_ok=True)
items={
'P342645_Ea.html':'https://oracc.museum.upenn.edu/dcclt/signlists/P342645',
'P257722_Ea.html':'https://oracc.museum.upenn.edu/dcclt/signlists/P257722',
'Ea07_composite.html':'https://oracc.museum.upenn.edu/dcclt/signlists/Q000159',
'P342645_card.html':'https://cdli.earth/artifacts/342645',
'P257722_card.html':'https://cdli.earth/artifacts/257722',
'P222364_card.html':'https://cdli.earth/artifacts/222364',
'P222364_edition.html':'https://oracc.museum.upenn.edu/etcsri/Q001026',
'P257722_photo.jpg':'https://cdli.earth/dl/photo/P257722.jpg',
'P342645_photo.jpg':'https://cdli.earth/dl/photo/P342645.jpg',
'P222364_photo.jpg':'https://cdli.earth/dl/photo/P222364.jpg'}
def get(it):
 name,url=it;p=S/name
 a=subprocess.run(['curl','-sS','--fail','-L','--max-time','35',url,'-o',str(p)],capture_output=True,text=True)
 return dict(file=name,url=url,returncode=a.returncode,bytes=p.stat().st_size if p.exists() else 0,error=a.stderr[:300])
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as e:log=list(e.map(get,items.items()))
(S/'fetch_log.json').write_text(json.dumps(log,ensure_ascii=False,indent=2))
print(json.dumps(log,ensure_ascii=False,indent=2))
