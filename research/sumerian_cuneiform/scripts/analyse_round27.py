"""Round 27: compare physical witnesses separately from editorial composites."""
from pathlib import Path
from collections import Counter
from html.parser import HTMLParser
from html import unescape
import re,csv,json,zipfile,sys
R=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(R/'scripts'))
from census_possessives import nodes
from check_round11 import line_labels
class Text(HTMLParser):
    def __init__(self):super().__init__();self.parts=[]
    def handle_data(self,d):self.parts.append(d)
def plaintext(s):
    p=Text();p.feed(s);return ' '.join(' '.join(p.parts).split())
def writecsv(name,records):
    if not records:return
    with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(records[0]));w.writeheader();w.writerows(records)
paths={
'P216745':('BM 109930','round26_sources/LuUtu_P216745_card.html','round26_sources/LuUtu_BM109930_photo.jpg'),
'P216746':('BM 109931','round26_sources/LuUtu_P216746_card.html','round27_sources/BM109931_photo.jpg'),
'P216747':('YBC 2148','round27_sources/P216747_card.html',''),
'P201507':('PTS 1501','round27_sources/P201507_card.html','round27_sources/PTS1501_photo.jpg'),
'P431422':('PTS 1502','round27_sources/P431422_card.html','round27_sources/PTS1502_photo.jpg')}
all_lines=[];witnesses=[]
for pid,(museum,card,photo) in paths.items():
    source=(R/card).read_text();t=plaintext(source)
    matches=re.findall(r'<div class="translate-indent[^>]*>(.*?)</div>',source,re.S)
    ls=[]
    for block in matches:
        for part in re.split(r'<br\s*/?>',block):
            line=plaintext(part);m=re.match(r'^(\d+)\.\s*(.*)$',line)
            if m:ls.append((int(m[1]),m[2]))
    for number,text in ls:all_lines.append(dict(pid=pid,museum=museum,line=number,editorial_text=text,source=card,status='редакторская транслитерация, не новое чтение фото'))
    target=' / '.join(f'{n}. {s}' for n,s in ls if ('gab' in s or 'bi2-in-ge' in s or 'bi2-in-zi' in s or 'bi2-in-gi' in s))
    periods=re.findall(r'dated to the ([^<]+?) period',source)
    witnesses.append(dict(pid=pid,museum=museum,catalogue_period=unescape(periods[0]) if periods else 'не извлечён',numbered_lines=len(ls),target=target,contains_Ninisina='yes' if any('nin-in-sin2' in s for _,s in ls) else ('unknown' if not ls else 'no'),card=card,photo=photo,photo_inspected=bool(photo),source_url='https://cdli.earth/artifacts/'+pid[1:]))
writecsv('round27_witnesses.csv',witnesses);writecsv('round27_full_witness_texts.csv',all_lines)
contexts=[];failed=[];parsed=0
with zipfile.ZipFile(R/'sources/ETCSRI_2026-10-24_json.zip') as z:
    for fn in sorted(z.namelist()):
        if not re.search('/corpusjson/Q[0-9]+.json$',fn):continue
        try:d=json.loads(z.read(fn))
        except (ValueError,UnicodeError):failed.append(fn);continue
        parsed+=1;ns=list(nodes(d));ll=line_labels(d)
        for i,n in enumerate(ns):
            f=n['f']
            if f.get('cf')!='gin':continue
            contexts.append(dict(qid=Path(fn).stem,ref=n['ref'],line=ll.get(n['ref'].rsplit('.',1)[0],''),form=f.get('form'),norm=f.get('norm'),sense=f.get('sense'),signs=' '.join(g.get('gdl_sign','?') for g in f.get('gdl',[])),before=[a['f'] for a in ns[max(0,i-5):i]],after=[a['f'] for a in ns[i+1:i+4]]))
(R/'data/round27_gin_contexts.json').write_text(json.dumps(contexts,ensure_ascii=False,indent=2))
flat=[]
for a in contexts:flat.append({**{k:a[k] for k in ['qid','ref','line','form','norm','sense','signs']},'before':' '.join(x.get('form','') for x in a['before']),'after':' '.join(x.get('form','') for x in a['after']),'status':'editorial model, not photo reading'})
writecsv('round27_gin_contexts.csv',flat)
counts=dict(Counter(a['form'] for a in contexts))
summary=dict(parsed_files=parsed,failed_files=failed,gin_tokens=len(contexts),gin_compositions=len({a['qid'] for a in contexts}),forms=counts,river_immediately_before=[a['ref'] for a in contexts if a['before'] and a['before'][-1].get('cf')=='id'],physical_witnesses=len(witnesses),photos_inspected=sum(a['photo_inspected'] for a in witnesses),new_photos=3,new_witness_cards=3,numbered_lines_by_witness={a['pid']:a['numbered_lines'] for a in witnesses},selection="f.cf == 'gin'; no semantic inference from counts alone")
(R/'data/round27_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2))
variants=[
 dict(case='BM 109930 / CDLI',reading='gab-ba a bi₂-in-gi',model_signs='GABA BA A NE IN GI',sign_count=6,status='model; full photo localisation unfinished'),
 dict(case='Q000873 / ETCSRI',reading='gaba-ba id₅ bi₂-in-gen₆',model_signs='GABA BA A NE IN GI',sign_count=6,status='composite, not sixth physical witness'),
 dict(case='YBC 2148 / CDLI',reading='gab-ba / a bi₂-in-ge-en₆',model_signs='GABA BA A NE IN GI IN',sign_count=7,status='additional final IN in editorial model; checked against 1915 handcopy, no photo obtained'),
 dict(case='PTS 1501 / CDLI',reading='gaba-ba-a bi₂-in-zi',model_signs='GABA BA A NE IN ZI',sign_count=6,status='candidate sign substitution GI/ZI; photo obtained, diagnostic wedge comparison unfinished'),
 dict(case='PTS 1502 / CDLI',reading='gaba-ba-a bi₂-in-zi',model_signs='GABA BA A NE IN ZI',sign_count=6,status='same model as PTS1501, distinct photographed object')]
writecsv('round27_target_variants.csv',variants)
print(json.dumps(summary,ensure_ascii=False,indent=2))
