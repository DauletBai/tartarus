import sys,json,zipfile,re,collections
from pathlib import Path
R=Path(__file__).resolve().parents[1];sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
out=[];failed=[];parsed=0
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
 cat=json.loads(z.read('etcsri/catalogue.json'))['members']
 for fn in sorted(z.namelist()):
  if not re.search('/corpusjson/Q[0-9]+.json$',fn):continue
  try:d=json.loads(z.read(fn))
  except (ValueError,UnicodeError):failed.append(fn);continue
  parsed+=1;ns=list(nodes(d));ls=line_labels(d);q=Path(fn).stem
  for i,n in enumerate(ns):
   f=n['f']
   if f.get('cf')!='gaba':continue
   out.append(dict(qid=q,ref=n.get('ref'),line=ls.get(n.get('ref','').rsplit('.',1)[0],''),form=f.get('form'),f=f,context_before=[a['f'].get('form') for a in ns[max(0,i-6):i]],context_after=[a['f'].get('form') for a in ns[i+1:i+7]],metadata=cat.get(q,{})))
(R/'data/round26_gaba_corpus.json').write_text(json.dumps(out,ensure_ascii=False,indent=2));print('parsed',parsed,'failed',failed,'tokens',len(out),'compositions',len({a['qid'] for a in out}),'forms',collections.Counter(a['form'] for a in out))
for a in out[:18]:print(a['qid'],a['line'],a['form'],a['f'].get('sense'),a['metadata'].get('designation'),' '.join(a['context_before']), '||',' '.join(a['context_after']))
# Preserve the census as editorial data, not as physical-photo readings.
import csv,hashlib
summary=dict(parsed_files=parsed,failed_files=failed,tokens=len(out),compositions=len({a['qid'] for a in out}),forms=dict(collections.Counter(a['form'] for a in out)),selection='f.cf == gaba; lexical and compound entries with other citation forms excluded',archive_sha256=hashlib.sha256((R/'sources/ETCSRI_2026-10-24_json.zip').read_bytes()).hexdigest())
(R/'data/round26_corpus_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2))
rows=[]
for a in out:
 rows.append(dict(qid=a['qid'],ref=a['ref'],line=a['line'],form=a['form'],canonical_signs=' '.join(z.get('gdl_sign','?') for z in a['f'].get('gdl',[])),editorial_norm=a['f'].get('norm',''),editorial_morphology=a['f'].get('morph2',''),before=' '.join(a['context_before']),after=' '.join(a['context_after']),designation=a['metadata'].get('designation',''),status='редакторский разбор, не самостоятельное чтение фотографии'))
with (R/'data/round26_gaba_tokens.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
