"""Validate round28 scope, sources, local links, and book integrity."""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
import csv,json,hashlib
R=Path(__file__).resolve().parents[1]
def rc(p):
 with (R/p).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round28_summary.json').read_text())
assert s['parsed_files']==1456 and s['failed_files']==['etcsri/corpusjson/Q001066.json']
assert s['pattern_counts']=={'GABA_BA_A':1,'A_NE_IN':1,'A_NE_IN_GI':1,'NE_IN_GI':2,'NE_IN_ZI':0}
assert s['A_initial_verbs']==20 and s['A_initial_with_morph_V2_a']==15
patterns=rc('data/round28_sign_patterns.csv');assert len(patterns)==5
assert {x['qid'] for x in patterns if x['pattern'] in ['GABA_BA_A','A_NE_IN','A_NE_IN_GI']}=={'Q000873'}
par=rc('data/round28_water_parallel.csv');assert len(par)==2
assert [x['canonical_sign'] for x in par]==['A','|A.LAGAB×HAL|']
assert [x['physical_witness'] for x in par]==['P222400','P222402']
assert all(x['following']=='gibil mu-na-dun' for x in par)
assert len(rc('data/round28_translation_hypotheses.csv'))==3
assert len(rc('data/round28_corrections.csv'))==2
reg=[x for x in rc('SOURCE_REGISTER.csv') if x['local_file'].startswith('round28_sources/')]
assert len(reg)==16==len({x['local_file'] for x in reg})
assert not any('Schrakamp' in x['local_file'] or 'Zgoll2020' in x['local_file'] for x in reg)
for x in reg:
 p=R/x['local_file'];assert p.is_file() and p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
 if p.suffix=='.pdf':assert p.read_bytes().startswith(b'%PDF'),p
man=rc('data/round28_file_manifest.csv')
for x in man:
 p=R/x['file'];assert p.is_file() and p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
 def __init__(self):super().__init__();self.local=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):self.local.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round28.html').read_text())
for p in l.local:assert (R/p).is_file(),p
for n in ['26','27']:
 assert 'ДОПОЛНЕНИЕ ЦИКЛА 28' in next((R/'observations').glob('*round'+n+'*')).read_text()
 assert 'Уточнение цикла 28' in (R/('comparison_round'+n+'.html')).read_text()
for name,digest in {
 'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5',
 'Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}.items():assert sha(R.parents[1]/'book'/name)==digest,name
print(json.dumps(dict(status='passed',source_hashes=len(reg),manifest_hashes=len(man),parallel_witnesses=2,pattern_hits=len(patterns),A_initial_tokens=20,hypotheses=3,corrections=2,local_links=len(l.local),book_hashes='unchanged'),ensure_ascii=False,indent=2))
