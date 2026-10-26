from pathlib import Path
import csv,json,hashlib,collections
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
R=Path(__file__).resolve().parents[1]
def read(n):
 with (R/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round32_summary.json').read_text())
a=read('data/round32_pad_corpus.csv');assert len(a)==147 and len({x['qid'] for x in a})==100
assert len({x['form'] for x in a})==24
assert collections.Counter(x['context_pattern'] for x in a)=={'heart_noun_last4':76,'name_noun_last4':52,'other':19}
b=read('data/round32_lexical_target_rows.csv');assert len(b)==19
assert collections.Counter(x['pid'] for x in b)=={'P229436':9,'P527017':3,'P365268':5,'P250504':2}
assert all('UD' in x['main_sign'] for x in b if x['pid']=='P229436')
assert next(x for x in b if x['row_id']=='P527017.104')['pronunciation']=='[pa-ad]'
assert next(x for x in b if x['row_id']=='P250504.18')['main_sign']=='PAD'
assert next(x for x in b if x['row_id']=='P250504.119')['main_sign']=='IGI.RU'
src=[x for x in read('SOURCE_REGISTER.csv') if x['local_file'].startswith('round32_sources/')];assert len(src)==16
for x in src:
 p=R/x['local_file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
 if p.suffix=='.jpg':assert p.read_bytes()[:2]==b'\xff\xd8'
m=read('data/round32_file_manifest.csv')
for x in m:
 p=R/x['file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['href','src'] and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round32.html').read_text())
for p in l.urls:
 if p!='data/round32_checkpoint.txt':assert (R/p).exists(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
out=dict(status='passed',pad_tokens=147,compositions=100,forms=24,lexical_rows=19,UD_rows=9,source_hashes=len(src),manifest_hashes=len(m),local_links=len(l.urls),book_PDF='unchanged')
(R/'data/round32_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
assert all((R/p).exists() for p in l.urls);print(json.dumps(out,ensure_ascii=False,indent=2))
