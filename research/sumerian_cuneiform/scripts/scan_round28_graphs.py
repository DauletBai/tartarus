"""Compare written sign sequences before relying on word segmentation."""
from pathlib import Path
import sys,json,zipfile,re,collections,csv
R=Path(__file__).resolve().parents[1];sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
patterns={'GABA_BA_A':['GABA','BA','A'],'A_NE_IN':['A','NE','IN'],'A_NE_IN_GI':['A','NE','IN','GI'],'NE_IN_GI':['NE','IN','GI'],'NE_IN_ZI':['NE','IN','ZI']}
hits=[];verbal_a=[];parsed=0;failed=[]
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
 for fn in sorted(z.namelist()):
  if not re.search('/corpusjson/Q[0-9]+.json$',fn):continue
  try:d=json.loads(z.read(fn))
  except (ValueError,UnicodeError):failed.append(fn);continue
  parsed+=1;ns=list(nodes(d));labels=line_labels(d);signs=[]
  for i,n in enumerate(ns):
   f=n['f'];gs=[g.get('gdl_sign') for g in f.get('gdl',[]) if g.get('gdl_sign')]
   if gs[:1]==['A'] and f.get('pos','').startswith('V'):
    verbal_a.append(dict(qid=Path(fn).stem,ref=n['ref'],form=f.get('form'),signs=' '.join(gs),cf=f.get('cf'),sense=f.get('sense'),morph=f.get('morph'),before=' '.join(a['f'].get('form','') for a in ns[max(0,i-3):i]),after=' '.join(a['f'].get('form','') for a in ns[i+1:i+4])))
   for g in f.get('gdl',[]):
    # Determinatives and compound signs are barriers, not silently discarded.
    signs.append(dict(sign=g.get('gdl_sign','<non-simple>'),ref=n['ref'],word=f.get('form'),cf=f.get('cf'),pos=f.get('pos'),line=n['ref'].rsplit('.',1)[0],node_index=i))
  for name,pat in patterns.items():
   for i in range(len(signs)-len(pat)+1):
    ss=signs[i:i+len(pat)]
    if [a['sign'] for a in ss]!=pat:continue
    if len({a['line'] for a in ss})>1:continue # same editorial line only
    ia=ss[0]['node_index'];ib=ss[-1]['node_index'];refs=list(dict.fromkeys(a['ref'] for a in ss))
    hits.append(dict(qid=Path(fn).stem,pattern=name,line=labels.get(ss[0]['line'],''),refs=' '.join(refs),first_sign_index=i,signs=' '.join(pat),inside_one_editorial_word=len(refs)==1,words=' '.join(n['f'].get('form','') for n in ns[ia:ib+1]),before=' '.join(n['f'].get('form','') for n in ns[max(0,ia-4):ia]),after=' '.join(n['f'].get('form','') for n in ns[ib+1:ib+5]),analyses=' | '.join(str(n['f'].get('cf'))+':'+str(n['f'].get('pos')) for n in ns[ia:ib+1])))
for name,data in [('round28_sign_patterns.csv',hits),('round28_A_initial_verbs.csv',verbal_a)]:
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  fields=list(data[0]) if data else ['no_hits'];w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(data)
summary=dict(parsed_files=parsed,failed_files=failed,pattern_counts={k:sum(a['pattern']==k for a in hits) for k in patterns},A_initial_verbs=len(verbal_a),A_initial_verb_forms=dict(collections.Counter(a['form'] for a in verbal_a)),limitations='canonical simple GDL signs; determinatives/compounds form barriers; same editorial line; editorial segmentation retained only as comparison field; model glyph IDs, not independent photo readings')
(R/'data/round28_pattern_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2))
print(json.dumps(summary,ensure_ascii=False,indent=2))
for a in hits[:30]:print(a['qid'],a['pattern'],a['words'],a['analyses'])
for a in verbal_a[:25]:print('VERB_A',a)
