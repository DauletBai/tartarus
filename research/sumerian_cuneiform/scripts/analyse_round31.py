"""Round31: syllabic spellings, physical witnesses, and contextual counts."""
from pathlib import Path
import csv,json,collections,hashlib,re,zipfile,sys
import xml.etree.ElementTree as E
R=Path(__file__).resolve().parents[1];S=R/'round31_sources';NS={'h':'http://www.w3.org/1999/xhtml'}
def clean(e):return ' '.join(''.join(e.itertext()).split())
def write(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
root=E.parse(S/'praxis_utud_occurrences.html').getroot();contexts=[];heading='';last=None
for e in root.iter():
 if e.tag.endswith('h2'):heading=clean(e)
 if e.get('class')=='ce-result':last=e
 if e.get('class')=='ce-label' and last is not None:
  a=e.find('h:a',NS)
  if a is None:continue
  words=[w for w in last.findall('h:span',NS) if 'w' in w.get('class','').split()]
  selected=[clean(w) for w in words if 'selected' in w.get('class','').split()]
  assert len(selected)==1
  repeats=sum(clean(w)==selected[0] for w in words)
  contexts.append(dict(ref=a.get('data-iref'),pid=a.get('data-iref').split('.')[0],label=clean(a),
                       category=heading,selected_form=selected[0],same_form_tokens_in_context=repeats,
                       context=clean(last),status='editorial concordance, not independent photo reading'))
write('round31_praxis_contexts.csv',contexts)
targets=[]
for fn,pid,ids in [('P238324_edition.html','P238324',[39]),('P365750_edition.html','P365750',[71,74]),('P349438_edition.html','P349438',[3]),('P223429_praxis.html','P223429',[3,5]),('P343032_praxis.html','P343032',[3,4])]:
 root=E.parse(S/fn).getroot()
 for tr in root.findall('.//h:tr',NS):
  if tr.get('id') not in {pid+'.'+str(n) for n in ids}:continue
  cc=[clean(c) for c in tr.findall('h:td',NS)]
  targets.append(dict(pid=pid,row_id=tr.get('id'),line=cc[0],all_cells=' | '.join(cc),
                      has_graphic_UD='tu-ud' in ' | '.join(cc),
                      status='witness edition; damage brackets retained; translations editorial'))
write('round31_target_rows.csv',targets)
whole=[]
for pid in ['P223429','P343032']:
 root=E.parse(S/(pid+'_praxis.html')).getroot()
 for tr in root.findall('.//h:tr',NS):
  cc=[clean(c) for c in tr.findall('h:td',NS)]
  if len(cc)>=2 and tr.get('id','').startswith(pid+'.'):
   whole.append(dict(pid=pid,row_id=tr.get('id'),line=cc[0],text=cc[1],status='complete available editorial lines, not new translation'))
write('round31_early_complete_texts.csv',whole)
sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
control=[]
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
 d=json.loads(z.read('etcsri/corpusjson/Q001965.json'))
 for n in nodes(d):
  if n['ref'] not in ['Q001965.4.2','Q001965.7.1']:continue
  f=n['f'];control.append(dict(ref=n['ref'],pid='P289683',form=f['form'],cf=f['cf'],morph=f.get('morph',''),signs=' '.join(g.get('gdl_sign','?') for g in f.get('gdl',[])),status='editorial GDL; handcopy inspected, no photograph acquired'))
existing_control=R/'data/round31_UD_tense_control.csv'
if existing_control.exists():
 with existing_control.open(encoding='utf-8-sig') as f:
  previous=list(csv.DictReader(f))
 control.extend(x for x in previous if x['ref'] not in {'Q001965.4.2','Q001965.7.1'})
write('round31_UD_tense_control.csv',control)
visual=[
dict(pid='P238324',file='P238324_photo.jpg',type='photographic composite',observation='Joined fragments Rm348/Rm604, damaged left areas, ruled bilingual columns and holes visible',limit='Joined pieces are not independent tablets; museum number field BM dash is incomplete; target DUMU damaged'),
dict(pid='P238324',file='P238324_lineart.jpg',type='modern handcopy',observation='Caption Rm604; obverse17 shows four successive sign positions on left of bilingual ruling',limit='Image says obverse on preceding plate; joined Rm348 not fully represented; not a complete new copy of all joins'),
dict(pid='P238324',file='Rm604_birth_band.jpg',type='unaltered photo crop',observation='Band covering proposed rows14–19 compared with Rm604 handcopy; leftmost area eroded',limit='x1250 y500 w690 h180; target row17 identification contextual; not every wedge independently collated'),
dict(pid='P365750',file='P365750_photo.jpg',type='photographic composite',observation='Several faces, breaks and multiple columns; side label K4332 and joins',limit='Full inscription not read independently; precise photo localization ii3/ii6 open'),
dict(pid='P365750',file='P365750_lineart.jpg',type='modern handcopy',observation='K4332 copy preserves arranged columns including top of second column',limit='Copy is scholarly interpretation; divine-name damage remains uncertain'),
dict(pid='P343032',file='P343032_photo.jpg',type='photographic composite',observation='Front/back and sides; worn but visible horizontal lines; no scene depicting beings',limit='Graphic inspection does not determine origin of intelligence'),
dict(pid='P343032',file='P343032_lineart.jpg',type='modern handcopy',observation='Line3 contains two repeated three-sign groups; line4 ends in a similar two-sign group after longer prefix chain',limit='Edition identifies IM TU UD and TU UD; sounds rely on broader sign model'),
dict(pid='P343032',file='VS17_12_obverse.jpg',type='unaltered photo crop',observation='Whole written front inspected against handcopy; upper rows worn, lower groups clearer',limit='x240 y295 w650 h410; partial graphic comparison, not independent full reading'),
dict(pid='P223429',file='P223429_lineart.jpg',type='modern handcopy',observation='Obverse1 and3 end in matching groups; edition BA TU UD; erasures elsewhere explicitly marked',limit='Photo request404; handcopy not silently treated as photograph'),
dict(pid='P349438',file='P349438_edition.html',type='edition without image',observation='Opening row has u3-tu-ud-da in a creation context',limit='Photo404; catalogue no-image; no visual verification claimed')]
visual.append(dict(pid='P289683',file='P289683_lineart.jpg',type='modern handcopy',observation='Eleven ruled lines; line4 and7 compared with edition; UD used within two different editorial words',limit='No photograph acquired; alternative graph readings and morphology remain model-dependent'))
write('round31_visual_audit.csv',visual)
tests=[
dict(question='Could d belong only to following -da?',evidence='P238324 o ii17 tu-ud-da; P223429 o1/o3 ba-tu-ud; P343032 l3 im-tu-ud twice',result='Separate UD occurs before DA and where DA absent; supports stem-final consonant in current syllabic model',limit='Sound assignment UD=ud not independently established by these photographs'),
dict(question='Is initial u mandatory in every written form?',evidence='tu-ud alongside u3-tu-ud; ba-tu-ud and im-tu-ud',result='Written forms vary; some u3 may be grammatical; cannot assume every u3 part of immutable root',limit='Detailed phonological history of utud/dud unresolved'),
dict(question='Does same TU UD sequence take different affixes?',evidence='ba-tu-ud; im-tu-ud; nu-tu-ud; nu-tu-ud-da; im-ma-ni-in-tu-ud',result='Stable written sequence combined with different prefixed and suffixed material; concrete agglutinative pattern',limit='Different functions of each prefix require separate paradigmatic test'),
dict(question='Does UD necessarily mean sun in tu-ud?',evidence='Lexical offspring equivalence and distribution with verbal prefixes',result='Syllabic reading explains spelling without translating UD as sun each time',limit='Reject or support by whole clauses, not isolated pictorial associations'),
dict(question='Does this establish Kazakh ancestry?',evidence='Kazakh tuw-; older tog-/tug-; Sumerian TU UD spellings',result='Birth-related candidate remains; stronger ancient spelling control, no d/g/w rule or historical direction established',limit='Need independently recurring sound correspondences and contact chronology'),
dict(question='Does literary creation establish extraterrestrial intelligence?',evidence='P343032 classified incantation; fish/reed-related terms in edition; no depicted beings on inspected photographs',result='Creation-language evidence; no explicit verifiable statement of extraterrestrial arrival obtained',limit='No new full translation of incantation; genre does not resolve metaphysical claim')]
tests.append(dict(question='TU command versus UD completed birth, or UD as past marker?',evidence='TU alone in completed statue creation; TU UD before DA and without DA; Q001965.4 UD MU RA analyzed as birth; Q001965.7 UD annotated with lexical sense sun; syntactic interpretation is separate',result='Separate competing grammatical hypothesis retained; current evidence does not establish TU as command or UD as a general past marker',limit='Need secure commands and statements with independently known temporal reference; negative and participial forms alone do not refute past meaning'))
write('round31_hypothesis_tests.csv',tests)
summary=dict(date='2026-10-26',praxis_declared_instances=25,concordance_contexts=len(contexts),concordance_objects=len({x['pid'] for x in contexts}),
             context_word_occurrences=sum(x['same_form_tokens_in_context'] for x in contexts),
             double_token_contexts=[x['ref'] for x in contexts if x['same_form_tokens_in_context']>1],
             target_rows=len(targets),target_objects=len({x['pid'] for x in targets}),early_complete_lines=len(whole),
             new_independent_phonetic_values=0,book_changed=False)
(R/'data/round31_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
reg=R/'SOURCE_REGISTER.csv'
with reg.open(encoding='utf-8-sig',newline='') as f:reader=csv.DictReader(f);fields=reader.fieldnames;existing=list(reader)
existing=[x for x in existing if not x['local_file'].startswith('round31_sources/')]
log=json.loads((S/'fetch_log.json').read_text())+json.loads((S/'fetch_extra_log.json').read_text())+json.loads((S/'fetch_tense_control_log.json').read_text())
log += [dict(file='P238324_lineart.jpg',url='https://cdli.earth/dl/lineart/P238324_l.jpg',returncode=0),dict(file='P365750_lineart.jpg',url='https://cdli.earth/dl/lineart/P365750_l.jpg',returncode=0),dict(file='praxis_utud_occurrences.html',url='https://oracc.museum.upenn.edu/epsd2/praxis/sux?xis=sux.r002009',returncode=0)]
for x in log:
 p=S/x['file']
 if x['returncode'] or not p.exists() or not p.stat().st_size:continue
 kind='photograph' if '_photo.' in p.name else 'modern_handcopy' if 'lineart' in p.name else 'catalogue' if 'card' in p.name else 'concordance' if 'occurrences' in p.name else 'editorial_edition_or_dictionary'
 existing.append(dict(local_file=str(p.relative_to(R)),source_url=x['url'],retrieved_date_Asia_Qostanay='2026-10-26',type=kind,bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),rights_and_notes='Research copy; external publication rights not established; modern reading distinguished from photograph'))
with reg.open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(existing)
print(json.dumps(summary,ensure_ascii=False,indent=2))
