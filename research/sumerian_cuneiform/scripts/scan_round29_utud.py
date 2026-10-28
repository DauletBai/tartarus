from pathlib import Path
import json,zipfile,csv,sys,collections,re
R=Path(__file__).resolve().parents[1];sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
rs=[];parsed=0;failed=[];ud_occurrences=[];poss_dative=[]
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
 for name in sorted(z.namelist()):
  if not re.search(r'/corpusjson/Q[0-9]+.json$',name):continue
  try:d=json.loads(z.read(name))
  except ValueError:failed.append(name);continue
  parsed+=1;ns=list(nodes(d));labels=line_labels(d)
  for i,n in enumerate(ns):
   f=n['f']
   # Editorial GDL search: one record per word containing an exact UD token.
   g=f.get('gdl',[])
   morphology=f.get('morph2','')
   if 'N3=1-SG-POSS' in morphology and 'DAT-H' in morphology:
    poss_dative.append(dict(ref=n['ref'],form=f.get('form',''),cf=f.get('cf',''),sense=f.get('sense',''),morph2=morphology,signs=' '.join(part.get('gdl_sign','?') for part in g)))
   if any(part.get('gdl_sign')=='UD' for part in g):
    ud_occurrences.append(dict(qid=Path(name).stem,ref=n['ref'],form=f.get('form',''),cf=f.get('cf',''),sense=f.get('sense',''),signs=' '.join(part.get('gdl_sign','?') for part in g),before=' '.join(a['f'].get('form','') for a in ns[max(0,i-4):i]),after=' '.join(a['f'].get('form','') for a in ns[i+1:i+5])))
   if f.get('cf') not in ['utud','tud']:continue
   rs.append(dict(qid=Path(name).stem,ref=n['ref'],line=labels.get(n['ref'].rsplit('.',1)[0],''),form=f.get('form'),cf=f.get('cf'),norm=f.get('norm'),morph=f.get('morph'),sense=f.get('sense'),signs=' '.join(g.get('gdl_sign','?') for g in f.get('gdl',[])),before=' '.join(a['f'].get('form','') for a in ns[max(0,i-4):i]),after=' '.join(a['f'].get('form','') for a in ns[i+1:i+5]),status='editorial model, not independently translated photo'))
with (R/'data/round29_utud_contexts.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=list(rs[0]) if rs else ['no_hits']);w.writeheader();w.writerows(rs)
s=dict(parsed_files=parsed,failed_files=failed,tokens=len(rs),compositions=len({x['qid'] for x in rs}),forms=dict(collections.Counter(x['form'] for x in rs)),citation_forms=dict(collections.Counter(x['cf'] for x in rs)),selection="exact f.cf in ['utud','tud']")
# Exact GDL match does not include every compound shape or bypass editorial readings.
assert len(ud_occurrences)==365 and len({x['qid'] for x in ud_occurrences})==158
counts=collections.Counter((x['cf'],x['sense']) for x in ud_occurrences)
assert counts[('ud','sun')]==114 and counts[('ud','day')]==90 and counts[('ud','when')]==12
assert sum('TU' in x['signs'].split() for x in rs)==86
assert sum('UD' in x['signs'].split() for x in rs)==1
assert collections.Counter(x['sense'] for x in rs)=={'fashion a statue, to bring a statue into being':47,'bear':45}
assert len(poss_dative)==10 and sum(x['signs'].endswith('MU RA') for x in poss_dative)==3
assert {x['ref'] for x in poss_dative if x['signs'].endswith('MU RA')}=={'Q001965.4.2','Q002115.2.1','Q004103.3.1'}
s['possessive_dative_editorial_controls_2026_10_28']=dict(records=10,explicit_MU_RA=3,other_records_without_explicit_RA=7,examples=poss_dative,scope='editorial morphology, not blind identification; two nin records may not be independent ancient sources')
s['utud_selected_sign_audit_2026_10_28']=dict(records=92,TU=86,UD=1,sense_bear=45,sense_fashion_statue=47,scope='selected editorial corpus word records, not independent tablets or proof of biological events')
s['UD_editorial_gdl_audit_2026_10_28']=dict(selection='words with at least one exact gdl_sign UD, excludes compound representations and unparsed Q001066',word_records=len(ud_occurrences),composition_files=len({x['qid'] for x in ud_occurrences}),editorial_cf_sense_labels=len(counts),sun=114,day=90,when=12,utud_bear=counts[('utud','bear')],scope='editorial corpus annotations; compositions not counted as independent tablets; results are not new sound decipherment')
(R/'data/round29_utud_summary.json').write_text(json.dumps(s,ensure_ascii=False,indent=2));print(json.dumps(s,ensure_ascii=False,indent=2))
# Reuse the existing UD controls; Q examples are editorially annotated, with no new image claims.
control=R/'data/round31_UD_tense_control.csv'
with control.open(encoding='utf-8-sig') as f:
 reader=csv.DictReader(f);fields=reader.fieldnames;existing=list(reader)
existing=[v for v in existing if v['ref'] in ('Q001965.4.2','Q001965.7.1')]
refs=['Q000376.72.1','Q000377.269.1','Q001056.254.1','Q000377.195.4','Q001829.7.1','Q000984.28.2']
byref={x['ref']:x for x in ud_occurrences}
assert all(x in byref for x in refs)
for ref in refs:
 x=byref[ref]
 existing.append(dict(ref=ref,pid='Q composition; physical witness not verified',form=x['form'],cf=x['cf'],morph='sense: '+x['sense'],signs=x['signs'],status='editorial GDL context; no independent photo or phonetic decipherment; '+x['before']+' || '+x['after']))
with control.open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(existing)

for x in rs[:6]:print(x['qid'],x['form'],x['before'],'||',x['after'])
