from pathlib import Path
import csv,json,hashlib,collections
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
R=Path(__file__).resolve().parents[1]
def read(n):
    with (R/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round30_summary.json').read_text())
a=read('data/round30_TU_witness_rows.csv');assert len(a)==8
assert collections.Counter(x['pid'] for x in a)=={'P342645':7,'P257722':1}
b=next(x for x in a if x['row_id']=='P342645.197')
assert b['pronunciation']=='* du-u' and b['main_sign']=='TU' and b['akkadian']=='wa-la-dam'
d=next(x for x in a if x['pid']=='P257722')
assert d['main_sign']=='[TU]' and d['pronunciation']=='[*] [...]' and '[wa]' in d['akkadian']
assert len(read('data/round30_UrNanse11_complete.csv'))==36
assert collections.Counter(x['form'] for x in read('data/round30_UrNanse11_verb_contrast.csv'))=={'mu-du₃':9,'mu-tud':7}
assert s['independently_deciphered_new_phonetic_values']==0 and not s['composite_counted_as_new_witness']
src=[x for x in read('SOURCE_REGISTER.csv') if x['local_file'].startswith('round30_sources/')]
assert len(src)==11==len({x['local_file'] for x in src})
for x in src:
    p=R/x['local_file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
    if p.suffix=='.jpg':assert p.read_bytes()[:2]==b'\xff\xd8',p
for x in read('data/round30_file_manifest.csv'):
    p=R/x['file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
    def __init__(self):super().__init__();self.urls=[]
    def handle_starttag(self,t,attrs):
        for k,v in attrs:
            if k in ['href','src'] and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round30.html').read_text())
for p in l.urls:
    if p!='data/round30_checkpoint.txt':assert (R/p).is_file(),p
for file,digest in {
 'Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}.items():assert sha(R.parents[1]/'book'/file)==digest,file
out=dict(status='passed',witness_rows=8,witness_objects=2,source_hashes=len(src),manifest_hashes=len(read('data/round30_file_manifest.csv')),local_links=len(l.urls),complete_inscription_lines=36,book_hashes='PDF unchanged; draft not present in repository')
(R/'data/round30_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
assert all((R/p).is_file() for p in l.urls)
print(json.dumps(out,ensure_ascii=False,indent=2))
