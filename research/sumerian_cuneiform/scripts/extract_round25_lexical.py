from pathlib import Path
import xml.etree.ElementTree as E
import csv,json
R=Path(__file__).resolve().parents[1];S=R/'round25_sources';D=R/'data';ns={'h':'http://www.w3.org/1999/xhtml'}
def clean(e):return ' '.join(''.join(e.itertext()).split())
def parse(file):
 root=E.fromstring((S/file).read_text());rows=[]
 for tr in root.findall('.//h:tr',ns):
  cells=tr.findall('h:td',ns);label=tr.find('.//h:span[@class="xlabel "]',ns)
  if len(cells)<2 or not tr.get('id') or 'l' not in tr.get('class','').split():continue
  cc=[clean(c) for c in cells]
  if len(cc)==2:
   rows.append(dict(source=file,row_id=tr.get('id'),line=clean(label) if label is not None else cc[0],editorial_headword='',pronunciation='',main_sign=cc[1],akkadian='',other_cells=[]));continue
  if len(cc)<4:continue
  rows.append(dict(source=file,row_id=tr.get('id'),line=clean(label) if label is not None else cc[0],editorial_headword=cc[1],pronunciation=cc[2],main_sign=cc[3],akkadian=cc[4] if len(cc)>4 else '',other_cells=cc[5:]))
 return rows
def out(name,rows):
 with (D/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
files=['P227708_oracc.html','P240227_oracc.html','Syllabary_B_composite_Q000145.html','P230301_oracc.html','P365230_oracc.html','P382557_oracc.html']
allrows=[r for f in files for r in parse(f)]
(D/'round25_lexical_rows.json').write_text(json.dumps(allrows,ensure_ascii=False,indent=2))
selected=[r for r in allrows if r['source']=='P227708_oracc.html' and r['row_id'] in [f'P227708.{i}' for i in range(22,31)]]
assert len(selected)==9
for r in selected:
 r['has_editorial_correction']='!' in r['pronunciation'];r['has_uncertainty']='?' in r['pronunciation'];r['has_damage']='[' in r['pronunciation'] or '⸢' in r['pronunciation'];r['independent_sound_assignment']=False
out('round25_nine_pronunciation_entries.csv',selected)
control=[r for r in allrows if (r['source']=='P240227_oracc.html' and r['line'] in ["i 9'","ii 10'","ii 11'","ii 12'","ii 13'","ii 14'"]) or (r['source']=='Syllabary_B_composite_Q000145.html' and r['line'] in ['Tablet 2, 365','Tablet 2, 367'])]
out('round25_control_readings.csv',control)
counts={f:sum(r['source']==f for r in allrows) for f in files}
(D/'round25_extraction_summary.json').write_text(json.dumps(dict(row_counts=counts,selected=9,corrections=sum(r['has_editorial_correction'] for r in selected),notes='Physical witnesses and composite kept separate. MIN and editorial supplied text retained. All nine chosen entries have a leading lexical bullet marker *. It is distinct from the post-grapheme collation flag; see ATF lexical conventions.'),ensure_ascii=False,indent=2))
print(json.dumps(dict(row_counts=counts,selected=9,control=len(control),corrections=sum(r['has_editorial_correction'] for r in selected)),ensure_ascii=False))
