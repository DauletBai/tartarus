from pathlib import Path
import subprocess,json,concurrent.futures
R=Path(__file__).resolve().parents[1];S=R/'round37_sources';S.mkdir(exist_ok=True)
urls={}
for pid,sub in [('P228706',''),('P227815',''),('P386409','signlists/')]:
 urls[pid+'_edition.html']='https://oracc.museum.upenn.edu/dcclt/'+sub+pid
 urls[pid+'_card.html']='https://cdli.earth/artifacts/'+pid[1:]
 urls[pid+'_photo.jpg']='https://cdli.earth/dl/photo/'+pid+'.jpg'
 urls[pid+'_lineart.jpg']='https://cdli.earth/dl/lineart/'+pid+'_l.jpg'
def get(it):
 name,url=it;p=S/name
 r=subprocess.run(['curl','-sS','--fail','-L','--max-time','30',url,'-o',str(p)],capture_output=True,text=True)
 valid=not r.returncode and p.exists() and p.stat().st_size>0
 if valid and name.endswith('.jpg'):valid=p.read_bytes()[:2]==b'\xff\xd8'
 return dict(file=name,url=url,returncode=r.returncode,valid=valid,bytes=p.stat().st_size if p.exists() else 0,error=r.stderr[:200])
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:log=list(ex.map(get,urls.items()))
(S/'fetch_log.json').write_text(json.dumps(log,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(log,ensure_ascii=False,indent=2))
