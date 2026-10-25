"""Focused TU/birth audit; editions, witnesses and photographs remain separate."""
from pathlib import Path
import csv, json, zipfile, sys, collections, hashlib, html
import xml.etree.ElementTree as E

R=Path(__file__).resolve().parents[1]
S=R/'round30_sources'
sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
NS={'h':'http://www.w3.org/1999/xhtml'}
def clean(e): return ' '.join(''.join(e.itertext()).split())
def output(name,rows):
    with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

# Exactly seven neighbouring witness rows and one damaged parallel, selected by IDs.
# The headword and English translation are editorial and not counted as clay columns.
lex=[]
for fn,pid,ids in [('P342645_Ea.html','P342645',range(195,202)),
                   ('P257722_Ea.html','P257722',[64])]:
    root=E.parse(S/fn).getroot()
    for tr in root.findall('.//h:tr',NS):
        if tr.get('id') not in {pid+'.'+str(i) for i in ids}:continue
        cc=[clean(c) for c in tr.findall('h:td',NS)]
        label=tr.find('.//h:span[@class="xlabel "]',NS)
        # MS3178 has an additional, empty sign-component column.
        akk_index=5 if pid=='P342645' else 4
        lex.append(dict(pid=pid,row_id=tr.get('id'),line=clean(label) if label is not None else cc[0],
                        editorial_headword=cc[1],pronunciation=cc[2],main_sign=cc[3],
                        akkadian=cc[akk_index],editorial_translation=cc[akk_index+1] if len(cc)>akk_index+1 else '',
                        all_cells=' | '.join(cc),
                        evidence='witness edition; photo inspected structurally, not independently read',
                        restoration='pronunciation absent; TU restored; Akkadian partly restored' if pid=='P257722' else 'see brackets in individual cells'))
output('round30_TU_witness_rows.csv',lex)

# Local corpus: retain a complete inscription, not a context window selected by sense.
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
    d=json.loads(z.read('etcsri/corpusjson/Q001026.json'))
    labels=line_labels(d);ls=collections.OrderedDict();forms=[]
    for n in nodes(d):
        ref=n['ref'].rsplit('.',1)[0];ls.setdefault(ref,[]).append(n)
    rows=[]
    for ref,ns in ls.items():
        for n in ns:
            f=n['f']
            if f.get('form') in ['mu-tud','mu-du₃']:
                forms.append(dict(ref=n['ref'],line=labels.get(ref,''),form=f['form'],
                                  signs=' '.join(g.get('gdl_sign','?') for g in f.get('gdl',[])),
                                  cf=f.get('cf',''),morph=f.get('morph','')))
        rows.append(dict(qid='Q001026',pid='P222364',ref=ref,line=labels.get(ref,''),
                         text=' '.join(n['f'].get('form','') for n in ns),
                         status='editorial transliteration; handcopy inspected; photograph unavailable'))
output('round30_UrNanse11_complete.csv',rows)
output('round30_UrNanse11_verb_contrast.csv',forms)

comparisons=[
 dict(item='sound key',sumerian='du-u | TU | wa-la-dam',kazakh='туу / tuw-',
      result='shared birth meaning is a useful candidate; no regular sound history established',
      source='P342645 r ii26; Grezsa p209 table5',level='ancient bilingual list via edition + modern Kazakh analysis'),
 dict(item='participle',sumerian='tud-da; model dud + a',kazakh='tuw-GAn / туған',
      result='comparable nonfinite function; different suffixes; DA sign is not automatically a -da morpheme',
      source='Q000377 Cylinder A ii28; Jagersma p639; Grezsa p209',level='competing morphological analysis, not new phonetic decipherment'),
 dict(item='finite verb',sumerian='mu-tud; model mu + n + dud + zero',kazakh='productive past suffix -DI: -dy/-di/-ty/-ti',
      result='prefix-rich Sumerian pattern differs from Kazakh suffixal past; visual d is insufficient to equate morphology',
      source='Q001026; Jagersma pp216,333; Dotton/Wagner printed43 PDF49',level='modern grammar models checked against selected spellings'),
 dict(item='historical control',sumerian='utud / tud / du₂; not identical editorial spellings',kazakh='tog- / tug- and modern tuw-',
      result='need independently recurring d/g/w correspondences and contact chronology; none supplied by this single pair',
      source='Erdal2004 p95; round29 historical control',level='unresolved historical relationship'),
 dict(item='creation context',sumerian='mu-tud with divine names; alan ... mu-tud in other inscriptions',kazakh='туу birth sense',
      result='cannot count every mu-tud as biological birth or evidence of biological creation',
      source='Q001026; Jagersma p216; 92-token corpus selected in round29',level='context and semantic extension')]
output('round30_tuu_grammar_comparison.csv',comparisons)
visual=[
 dict(file='round30_sources/P342645_photo.jpg',kind='photographic composite',observation='Two broad inscribed faces, edges, ruling and damaged areas visible; manuscript is MS3178',limit='No independent reading of every wedge or all three cells of r ii26'),
 dict(file='round30_sources/MS3178_reverse_context.jpg',kind='unaltered crop',observation='Reverse face columns, line ruling, extensive central break and resumed written areas visible',limit='x590 y3450 w1700 h2280; exact r ii26 localization remains provisional'),
 dict(file='round30_sources/MS3178_TU_candidate_region.jpg',kind='unaltered crop',observation='Candidate vicinity inspected at higher scale',limit='x850 y3930 w840 h720; not labelled a confirmed row crop'),
 dict(file='round30_sources/P257722_photo.jpg',kind='photographic composite of joined fragments',observation='Multiple joined fragments, large loss in middle and separately shown fragments',limit='Not a single complete tablet face; edition r ii5 restores TU and loses pronunciation'),
 dict(file='round30_sources/P342645_detail.jpg',kind='additional photographs of same object',observation='Alternative views show ruling and damage from other lighting/angles',limit='Same MS3178 witness, not independent testimony'),
 dict(file='round30_sources/P222364_lineart.jpg',kind='modern handcopy',observation='II3/II6/III3/III5/IV2/IV4/V1 repeat a different final complex from II1/III1/III7/IV6/V3/V5/V7/V9 and II4',limit='Copy supports graphic distinction MU TU vs MU DU3 in editorial model; does not independently establish sound or meaning; II4/III1 partly damaged'),
 dict(file='round30_sources/Jagersma_PDF360.png',kind='rendered scholarly grammar',observation='Printed333 PDF360 examples33a/b are actual child-bearing clauses in the published model',limit='TSA45 5:3 and MVN2:317 9 photographs not located in this cycle'),
 dict(file='round30_sources/Jagersma_PDF666.png',kind='rendered scholarly grammar',observation='Printed639 PDF666 example51 analyzes dud plus nominalizer a',limit='Scholarly morphological proposal; not direct observation of a sound'),
 dict(file='round30_sources/Jagersma_PDF243.png',kind='rendered scholarly grammar',observation='Printed216 PDF243 example51 has explicit alan statue as object of birth verb',limit='Semantic extension; not a biological birth claim')]
output('round30_visual_audit.csv',visual)

summary=dict(witness_objects=2,selected_witness_rows=len(lex),
             complete_birth_equivalences_in_selected_editions=1,damaged_birth_parallels=1,
             composite_counted_as_new_witness=False,UrNanse11_lines=len(rows),
             UrNanse11_verbs=dict(collections.Counter(x['form'] for x in forms)),
             previous_corpus_tokens=92,previous_corpus_compositions=57,
             independently_deciphered_new_phonetic_values=0,
             candidate_status='birth-related lexical resemblance; historical relation unresolved',
             books_changed=False)
(R/'data/round30_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')

# Successful external source acquisitions only; failed request remains in fetch log.
reg=R/'SOURCE_REGISTER.csv'
with reg.open(encoding='utf-8-sig',newline='') as f:reader=csv.DictReader(f);fields=reader.fieldnames;registered=list(reader)
registered=[x for x in registered if not x['local_file'].startswith('round30_sources/')]
log=json.loads((S/'fetch_log.json').read_text())
log.extend([dict(file='P342645_detail.jpg',url='https://cdli.earth/dl/photo/P342645_d.jpg',returncode=0),
            dict(file='P222364_lineart.jpg',url='https://cdli.earth/dl/lineart/P222364_l.jpg',returncode=0)])
for x in log:
    p=S/x['file']
    if x['returncode'] or not p.is_file() or not p.stat().st_size:continue
    isphoto=p.suffix=='.jpg' and 'lineart' not in p.name
    typ='photograph' if isphoto else 'modern_handcopy' if 'lineart' in p.name else 'composite_editorial_edition' if 'composite' in p.name else 'catalogue' if 'card' in p.name else 'witness_editorial_edition'
    registered.append(dict(local_file=str(p.relative_to(R)),source_url=x['url'],
                           retrieved_date_Asia_Qostanay='2026-10-25',type=typ,
                           bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),
                           rights_and_notes='Research copy; image publication rights not established; edition distinguished from physical witness'))
with reg.open('w',encoding='utf-8-sig',newline='') as f:
    w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(registered)
print(json.dumps(summary,ensure_ascii=False,indent=2))
