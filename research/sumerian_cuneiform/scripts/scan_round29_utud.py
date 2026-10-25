from pathlib import Path
import json,zipfile,csv,sys,collections,re
R=Path(__file__).resolve().parents[1];sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
rs=[];parsed=0;failed=[]
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
 for name in sorted(z.namelist()):
  if not re.search(r'/corpusjson/Q[0-9]+.json$',name):continue
  try:d=json.loads(z.read(name))
  except ValueError:failed.append(name);continue
  parsed+=1;ns=list(nodes(d));labels=line_labels(d)
  for i,n in enumerate(ns):
   f=n['f']
   if f.get('cf') not in ['utud','tud']:continue
   rs.append(dict(qid=Path(name).stem,ref=n['ref'],line=labels.get(n['ref'].rsplit('.',1)[0],''),form=f.get('form'),cf=f.get('cf'),norm=f.get('norm'),morph=f.get('morph'),sense=f.get('sense'),signs=' '.join(g.get('gdl_sign','?') for g in f.get('gdl',[])),before=' '.join(a['f'].get('form','') for a in ns[max(0,i-4):i]),after=' '.join(a['f'].get('form','') for a in ns[i+1:i+5]),status='editorial model, not independently translated photo'))
with (R/'data/round29_utud_contexts.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=list(rs[0]) if rs else ['no_hits']);w.writeheader();w.writerows(rs)
s=dict(parsed_files=parsed,failed_files=failed,tokens=len(rs),compositions=len({x['qid'] for x in rs}),forms=dict(collections.Counter(x['form'] for x in rs)),citation_forms=dict(collections.Counter(x['cf'] for x in rs)),selection="exact f.cf in ['utud','tud']")
(R/'data/round29_utud_summary.json').write_text(json.dumps(s,ensure_ascii=False,indent=2));print(json.dumps(s,ensure_ascii=False,indent=2))
for x in rs[:6]:print(x['qid'],x['form'],x['before'],'||',x['after'])
