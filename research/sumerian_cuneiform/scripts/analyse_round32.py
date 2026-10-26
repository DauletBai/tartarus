from pathlib import Path
import csv,json,zipfile,sys,re,collections,hashlib
import xml.etree.ElementTree as E
R=Path(__file__).resolve().parents[1];S=R/'round32_sources';NS={'h':'http://www.w3.org/1999/xhtml'}
sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
def clean(e):return ' '.join(''.join(e.itertext()).split())
def write(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
rows=[];failed=[];parsed=0
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
 for fn in sorted(z.namelist()):
  if not re.search(r'/corpusjson/Q[0-9]+\.json$',fn):continue
  try:d=json.loads(z.read(fn))
  except ValueError:failed.append(fn);continue
  parsed+=1;ns=list(nodes(d));labels=line_labels(d)
  for i,n in enumerate(ns):
   f=n['f']
   if f.get('cf')!='pad':continue
   before=ns[max(0,i-4):i]
   # Pattern search is lexical and positional, NOT a semantic classification.
   heart=any(x['f'].get('cf')=='šag' and x['f'].get('pos')=='N' for x in before)
   name=any(x['f'].get('cf')=='mu' and x['f'].get('pos')=='N' for x in before)
   pattern='heart_noun_last4' if heart else 'name_noun_last4' if name else 'other'
   rows.append(dict(qid=Path(fn).stem,ref=n['ref'],line=labels.get(n['ref'].rsplit('.',1)[0],''),form=f.get('form',''),gw=f.get('gw',''),sense=f.get('sense',''),morph=f.get('morph',''),signs=' '.join(g.get('gdl_sign','?') for g in f.get('gdl',[])),before=' '.join(x['f'].get('form','') for x in before),after=' '.join(x['f'].get('form','') for x in ns[i+1:i+5]),context_pattern=pattern,status='editorial lemmatization; patterns not confirmed senses'))
write('round32_pad_corpus.csv',rows)
(R/'data/round32_pad_nodes.json').write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n')
lex=[]
selection={'P527017':[104,105,106],'P365268':[97,111,112,140,141],'P229436':list(range(58,67)),'P250504':[18,119]}
for pid,ids in selection.items():
 root=E.parse(S/(pid+'_edition.html')).getroot()
 for tr in root.findall('.//h:tr',NS):
  if tr.get('id') not in {pid+'.'+str(n) for n in ids}:continue
  cc=[clean(c) for c in tr.findall('h:td',NS)]
  label=tr.find('.//h:span[@class="xlabel "]',NS)
  lexical=pid in ['P527017','P229436','P250504']
  lex.append(dict(pid=pid,row_id=tr.get('id'),line=clean(label) if label is not None else cc[0],pronunciation=cc[2] if lexical else '',main_sign=cc[3] if lexical else '',all_cells=' | '.join(cc),status='edition, not independent wedge reading'))
write('round32_lexical_target_rows.csv',lex)
comparisons=[
dict(pair='utud/tud ↔ туу/tuw-',positive='birth meaning controlled by lexical sources and TU UD spellings',required='explain d/g/w correspondences and initial vowel; no single rule established',result='candidate retained; no historical direction established'),
dict(pair='pad₃ ↔ тап-/таб-/табу',positive='find meaning overlaps; Kazakh тауып documented in grammar example341',required='p-a-d versus t-a-p requires more than final d loss; possible metathesis plus d/t needs independent examples',result='candidate, not a second confirmation of the same sound rule'),
dict(pair='pad₃/find ↔ pad/break',positive='similar scholarly readings; P250504 has pa-a and pa-ad pronunciation prompts',required='distinguish IGI.RU and PAD; do not transfer restored [pa-ad] as if preserved',result='graphic and semantic controls separated'),
dict(pair='UD ↔ past-birth marker',positive='previously proposed grammatical interpretation',required='UD repeated for nine pronunciation prompts in one school list; functions context-dependent; find imperative/past opposition',result='general marker not established; possible local grammatical use remains a separate hypothesis')]
write('round32_comparative_tests.csv',comparisons)
visual=[
dict(file='P229436_photo.jpg',pid='P229436',kind='photographic composite',observation='CBS7074: front, reverse, edges; reverse has repeating main-sign column beside variable short groups',limit='No independent complete phonetic reading; worn upper rows'),
dict(file='P229436_lineart.jpg',pid='P229436',kind='modern handcopy',observation='PBS12,56 copy: reverse col3 repeats main complex for successive entries',limit='Copy not independent clay witness; damage/sic marks retained'),
dict(file='CBS7074_reverse_context.jpg',pid='P229436',kind='unaltered crop',observation='Reverse column compared with copy; repeating graphic complex visible',limit='x820 y4600 w2200 h2650; row-by-row wedge collation incomplete'),
dict(file='P365268_photo.jpg',pid='P365268',kind='photographic composite',observation='K197 and joins: ruled lexical columns, break diagonals and joins visible',limit='Target five entries not fully localized and read independently'),
dict(file='P250504_photo.jpg',pid='P250504',kind='photographic composite',observation='Two written faces, joins and erased/worn areas; several lexical columns',limit='Specific o16/r ii34 not fully collated; no complete Akkadian for o16'),
dict(file='P527017_edition.html',pid='P527017',kind='edition without obtained photo',observation='Target first-column pronunciation [pa-ad] is restored; uncertain/corrected Akkadian remains marked',limit='Photo404; restored cells do not independently prove phonetics'),
dict(file='Kazakh_PDF69.png',pid='modern grammar',kind='rendered publication',observation='Printed63 / PDF69 example341 explicitly uses finding money and find converb',limit='Modern usage control; not a chronological bridge to Sumerian')]
write('round32_visual_audit.csv',visual)
summary=dict(date='2026-10-26',parsed_corpus_files=parsed,failed_corpus_files=failed,pad_tokens=len(rows),pad_compositions=len({x['qid'] for x in rows}),forms=dict(collections.Counter(x['form'] for x in rows)),lexical_patterns=dict(collections.Counter(x['context_pattern'] for x in rows)),lexical_rows=len(lex),lexical_objects=len(selection),UD_school_rows=9,new_independent_phonetic_values=0,book_changed=False)
(R/'data/round32_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
reg=R/'SOURCE_REGISTER.csv'
with reg.open(encoding='utf-8-sig',newline='') as f:r=csv.DictReader(f);fields=r.fieldnames;existing=list(r)
existing=[x for x in existing if not x['local_file'].startswith('round32_sources/')]
log=sum((json.loads((S/n).read_text()) for n in ['fetch_log.json','fetch_witness_log.json','fetch_extra_log.json']),[])
for x in log:
 p=S/x['file']
 if x['returncode'] or not p.exists() or not p.stat().st_size:continue
 typ='photograph' if '_photo.' in p.name else 'modern_handcopy' if 'lineart' in p.name else 'catalogue' if 'card' in p.name else 'editorial_edition_or_dictionary'
 existing.append(dict(local_file=str(p.relative_to(R)),source_url=x['url'],retrieved_date_Asia_Qostanay='2026-10-26',type=typ,bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),rights_and_notes='Research copy; image publication rights not established; restored readings not physical observations'))
with reg.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(existing)
print(json.dumps(summary,ensure_ascii=False,indent=2))
