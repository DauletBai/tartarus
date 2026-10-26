from pathlib import Path
import subprocess,json,concurrent.futures
R=Path(__file__).resolve().parents[1];S=R/'round31_sources';S.mkdir(exist_ok=True)
urls={
'P238324_edition.html':'https://oracc.museum.upenn.edu/dcclt/nineveh/P238324',
'P365750_edition.html':'https://oracc.museum.upenn.edu/dcclt/nineveh/P365750',
'P349438_edition.html':'https://oracc.museum.upenn.edu/dcclt/P349438',
'praxis_utud.html':'https://oracc.museum.upenn.edu/epsd2/praxis/cbd/sux/o0042158.html',
'P238324_card.html':'https://cdli.earth/artifacts/238324',
'P365750_card.html':'https://cdli.earth/artifacts/365750',
'P349438_card.html':'https://cdli.earth/artifacts/349438',
'P238324_photo.jpg':'https://cdli.earth/dl/photo/P238324.jpg',
'P365750_photo.jpg':'https://cdli.earth/dl/photo/P365750.jpg',
'P349438_photo.jpg':'https://cdli.earth/dl/photo/P349438.jpg'}
def get(it):
 name,url=it;p=S/name
 r=subprocess.run(['curl','-sS','--fail','-L','--max-time','35',url,'-o',str(p)],capture_output=True,text=True)
 return dict(file=name,url=url,returncode=r.returncode,bytes=p.stat().st_size if p.exists() else 0,error=r.stderr[:220])
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as ex:log=list(ex.map(get,urls.items()))
(S/'fetch_log.json').write_text(json.dumps(log,ensure_ascii=False,indent=2))
print(json.dumps(log,ensure_ascii=False,indent=2))
