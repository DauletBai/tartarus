from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
from collections import Counter
import zipfile,csv,json,hashlib
R=Path(__file__).resolve().parents[1]
def rc(n):
 with (R/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round29_summary.json').read_text());l=s['lexical'];u=s['utud']
assert (l['selected_pages'],l['available_editions'],l['unavailable_items'],l['lexical_rows'])==(40,31,9,477)
allrows=rc('data/round29_lexical_all_rows.csv');assert len(allrows)==477 and len({x['row_id'] for x in allrows})==477
assert Counter(x['cell_count'] for x in allrows)=={'4':330,'2':143,'3':4}
assert l['full_ga_ba_rows']==['P227708.26'] and l['physical_gaba_witnesses']==['P227708']
assert len(rc('data/round29_new_phonetic_rows.csv'))==8
assert (u['tokens'],u['compositions'],len(u['forms']))==(92,57,14)
assert len(rc('data/round29_utud_contexts.csv'))==92
h=json.loads((R/'data/round29_history_entries.json').read_text());assert len(h)==15
assert Counter(x['pid'] for x in h)=={'P201507':7,'P431422':4,'P216745':4}
assert all('bi2-in-zi' in x['line10'] for x in h if x['pid'] in ['P201507','P431422'])
p=R/'round29_sources/dcclt_2026-10-25.zip';assert sha(p)==l['archive_sha256']
with zipfile.ZipFile(p) as z:
 assert z.testzip() is None
 fs=[n for n in z.namelist() if '/corpusjson/' in n and n.endswith('.json')]
 assert len(fs)==461 and all(z.getinfo(n).file_size==0 for n in fs)
 assert len(json.loads(z.read('dcclt/catalogue.json'))['members'])==10219
reg=[x for x in rc('SOURCE_REGISTER.csv') if x['local_file'].startswith('round29_sources/')]
assert len(reg)==54==len({x['local_file'] for x in reg})
assert sum(x['type']=='unavailable_item_response' for x in reg)==9
for x in reg:
 p=R/x['local_file'];assert p.is_file() and p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
 if p.suffix=='.pdf':assert p.read_bytes().startswith(b'%PDF'),p
m=rc('data/round29_file_manifest.csv');assert len(m)==66
for x in m:
 p=R/x['file'];assert p.is_file() and p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
 def __init__(self):super().__init__();self.local=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):self.local.append(unquote(v.split('#')[0]))
lk=Links();lk.feed((R/'comparison_round29.html').read_text())
for p in lk.local:assert (R/p).is_file(),p
for name,digest in {
 'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5',
 'Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}.items():assert sha(R.parents[1]/'book'/name)==digest,name
print(json.dumps(dict(status='passed',available_editions=31,unavailable_items=9,lexical_rows=477,new_phonetic_rows=8,history_entries=15,utud_tokens=92,source_hashes=len(reg),manifest_hashes=len(m),local_links=len(lk.local),book_hashes='unchanged'),ensure_ascii=False,indent=2))
