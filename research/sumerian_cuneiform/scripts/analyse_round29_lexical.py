from pathlib import Path
import xml.etree.ElementTree as E
import json,csv,zipfile,collections,re,hashlib
R=Path(__file__).resolve().parents[1];S=R/'round29_sources';ns={'h':'http://www.w3.org/1999/xhtml'}
selected=json.loads((R/'data/round29_lexical_target_catalogue.json').read_text());allrows=[];coverage=[];errors=[]
def clean(e):return ' '.join(''.join(e.itertext()).split())
for m in selected:
 pid=m['pid'];p=S/(pid+'_Ea.html')
 try:root=E.parse(p).getroot()
 except E.ParseError as e:errors.append(dict(pid=pid,error=str(e)));continue
 missing_item='This project does not have the requested item' in ' '.join(root.itertext())
 rs=[]
 for tr in root.findall('.//h:tr',ns):
  if not tr.get('id','').startswith(pid+'.') or 'l' not in tr.get('class','').split():continue
  cc=[clean(c) for c in tr.findall('h:td',ns)]
  if len(cc)<2:continue
  label=tr.find('.//h:span[@class="xlabel "]',ns)
  r=dict(pid=pid,museum=m.get('museum_no',''),designation=m.get('designation',''),row_id=tr.get('id'),line=clean(label) if label is not None else cc[0],headword=cc[1] if len(cc)>=4 else '',pronunciation=cc[2] if len(cc)>=4 else '',main_sign=cc[3] if len(cc)>=4 else cc[1],akkadian=cc[4] if len(cc)>=5 else '',cell_count=len(cc),all_cells=' | '.join(cc))
  r.update(has_correction='!' in r['pronunciation'],has_uncertainty='?' in r['pronunciation'],has_damage=any(x in r['pronunciation'] for x in ['[','⸢','⸣']),gaba_target=bool(re.search(r'\bGABA\b',r['main_sign'])),has_full_ga_ba=bool(re.search(r'ga\s*-\s*ba',r['pronunciation'])))
  rs.append(r)
 allrows.extend(rs);coverage.append(dict(pid=pid,museum=m.get('museum_no'),primary_publication=m.get('primary_publication'),web_status='item_not_available' if missing_item else 'edition_available',row_count=len(rs),gaba_rows=sum(x['gaba_target'] for x in rs),full_ga_ba_rows=sum(x['has_full_ga_ba'] for x in rs),bibliographic_selection='OB Nippur Ea + MSL14 pp26–28 in publication fields'))
def csvout(name,rs):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rs[0]) if rs else ['no_rows']);w.writeheader();w.writerows(rs)
csvout('round29_lexical_all_rows.csv',allrows);csvout('round29_lexical_coverage.csv',coverage)
target=[r for r in allrows if r['gaba_target'] or r['has_full_ga_ba']];csvout('round29_gaba_target_rows.csv',target)
archive=R/'round29_sources/dcclt_2026-10-25.zip'
with zipfile.ZipFile(archive) as z:
 cat=json.loads(z.read('dcclt/catalogue.json'))['members'];fs=[n for n in z.namelist() if '/corpusjson/' in n and n.endswith('.json')];empty=[n for n in fs if z.getinfo(n).file_size==0]
 summary=dict(archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),archive_bytes=archive.stat().st_size,archive_members=len(z.namelist()),catalogue_entries=len(cat),catalogue_OB_Nippur_Ea=sum(m.get('subgenre')=='OB Nippur Ea' for m in cat.values()),corpus_files=len(fs),empty_corpus_files=len(empty),valid_corpus_files=0,selected_pages=len(selected),parsed_pages=len(coverage),available_editions=sum(x['web_status']=='edition_available' for x in coverage),unavailable_items=sum(x['web_status']=='item_not_available' for x in coverage),pages_without_rows=[a['pid'] for a in coverage if not a['row_count']],errors=errors,lexical_rows=len(allrows),target_rows=len(target),physical_gaba_witnesses=sorted({r['pid'] for r in target if r['gaba_target']}),full_ga_ba_rows=[r['row_id'] for r in target if r['has_full_ga_ba']])
(R/'data/round29_lexical_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2));print(json.dumps(summary,ensure_ascii=False,indent=2))
for r in target:print(r['pid'],r['line'],r['pronunciation'],'|',r['main_sign'])
