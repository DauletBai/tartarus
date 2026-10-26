from pathlib import Path
import subprocess,json,concurrent.futures
R=Path(__file__).resolve().parents[1];S=R/'round38_sources';S.mkdir(exist_ok=True)
names=['BM.34781','BM.77229','BM.48863','BM.34816','BM.33769','BM.34798','HSM.1137','BM.35726']
def get(name):
 file='eBL_'+name.replace('.','')+'.json';url='https://www.ebl.lmu.de/api/fragments/'+name;p=S/file
 r=subprocess.run(['curl','-sS','--fail','-L','--max-time','25',url,'-o',str(p)],capture_output=True,text=True)
 valid=False;info={}
 if not r.returncode and p.exists():
  try:
   d=json.loads(p.read_text());valid='museumNumber' in d
   info=dict(hasPhoto=d.get('hasPhoto'),has_ATF=bool(d.get('atf')),ABZ308_in_model_signs='ABZ308' in d.get('signs','').split(),description=d.get('description','')[:160])
  except ValueError:pass
 return dict(museum=name,file=file,url=url,returncode=r.returncode,valid=valid,error=r.stderr[:200],**info)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:log=list(ex.map(get,names))
(S/'control_fetch_log.json').write_text(json.dumps(log,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(log,ensure_ascii=False,indent=2))
