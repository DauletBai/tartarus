"""Check round 27 counts, provenance, and preservation of the book."""
from pathlib import Path
from collections import Counter
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
import csv,json,hashlib
R=Path(__file__).resolve().parents[1]
def readcsv(p):
 with (R/p).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round27_summary.json').read_text());a=readcsv('data/round27_gin_contexts.csv')
assert len(a)==31==len({x['ref'] for x in a})
assert len({x['qid'] for x in a})==23
assert len(s['forms'])==14 and Counter(x['form'] for x in a)==s['forms']
assert s['forms']['mu-na-gen₆']==1 and s['forms']['mu-na-ge-en₆']==8
assert s['river_immediately_before']==['Q000873.10.3']
w=readcsv('data/round27_witnesses.csv');assert len(w)==5==len({x['pid'] for x in w})
assert sum(x['photo_inspected']=='True' for x in w)==4
assert next(x for x in w if x['pid']=='P216746')['numbered_lines']=='0'
assert len(readcsv('data/round27_full_witness_texts.csv'))==46
v=readcsv('data/round27_target_variants.csv')
assert [x['model_signs'] for x in v]==['GABA BA A NE IN GI','GABA BA A NE IN GI','GABA BA A NE IN GI IN','GABA BA A NE IN ZI','GABA BA A NE IN ZI']
controls=json.loads((R/'data/round27_control_sign_values.json').read_text())
assert controls['en₆']['g']['gdl_sign']=='IN'
assert controls['ge']['g']['gdl_sign']=='GI' and controls['zi']['g']['gdl_sign']=='ZI'
reg=[x for x in readcsv('SOURCE_REGISTER.csv') if x['local_file'].startswith('round27_sources/')]
assert len(reg)==16==len({x['local_file'] for x in reg})
assert not any(x['local_file'].endswith('Q000873_score.html') for x in reg)
for x in reg:
 p=R/x['local_file'];assert p.is_file() and p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
m=readcsv('data/round27_file_manifest.csv')
for x in m:
 p=R/x['file'];assert p.is_file() and p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
 def __init__(self):super().__init__();self.local=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):self.local.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round27.html').read_text())
for p in l.local:assert (R/p).is_file(),p
for name,digest in {
 'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5',
 'Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}.items():assert sha(R.parents[1]/'book'/name)==digest,name
print(json.dumps(dict(status='passed',physical_cones=5,cone_photos=4,source_hashes=len(reg),manifest_hashes=len(m),gin_tokens=31,full_witness_lines=46,local_links=len(l.local),book_hashes='unchanged'),ensure_ascii=False,indent=2))
