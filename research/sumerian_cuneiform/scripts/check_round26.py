"""Validate round 26 outputs without treating editorial readings as new decipherments."""
from pathlib import Path
from collections import Counter
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
import csv,json,hashlib
R=Path(__file__).resolve().parents[1]
def rows(name):
    with (R/name).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round26_corpus_summary.json').read_text())
t=rows('data/round26_gaba_tokens.csv')
assert len(t)==11==len({a['ref'] for a in t})
assert len({a['qid'] for a in t})==7
assert Counter(a['form'] for a in t)==s['forms']
assert all(a['canonical_signs'].split()[0]=='GABA' for a in t)
assert s['parsed_files']==1456 and s['failed_files']==['etcsri/corpusjson/Q001066.json']
assert digest(R/'sources/ETCSRI_2026-10-24_json.zip')==s['archive_sha256']
v=rows('data/round26_LuUtu_line10_variants.csv')
assert ' '.join(a['canonical_sign'] for a in v)=='GABA BA A NE IN GI'
assert len(rows('data/round26_LuUtu_full_editorial_text.csv'))==11
assert len(rows('data/round26_form_models.csv'))==4
m=rows('data/round26_file_manifest.csv')
for a in m:
    p=R/a['file'];assert p.is_file(),p
    assert p.stat().st_size==int(a['bytes']),p
    assert digest(p)==a['sha256'],p
reg=[a for a in rows('SOURCE_REGISTER.csv') if a['local_file'].startswith('round26_sources/')]
assert len(reg)==len({a['local_file'] for a in reg})
for a in reg:
    p=R/a['local_file'];assert p.is_file(),p
    assert p.stat().st_size==int(a['bytes']),p
    assert digest(p)==a['sha256'],p
class Links(HTMLParser):
    def __init__(self):super().__init__();self.local=[]
    def handle_starttag(self,tag,attrs):
        for k,v in attrs:
            if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):self.local.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round26.html').read_text())
for target in l.local:assert (R/target).is_file(),target
expected={
'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5',
'Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}
for name,h in expected.items():assert digest(R.parents[1]/'book'/name)==h,name
print(json.dumps(dict(status='passed',tokens=len(t),compositions=7,forms=4,full_text_lines=11,source_hashes=len(reg),manifest_hashes=len(m),local_links=len(l.local),book_hashes='unchanged'),ensure_ascii=False,indent=2))
