from pathlib import Path
import csv,hashlib,json,subprocess,re
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
R=Path(__file__).resolve().parents[1]
def read(p):
 with (R/p).open(encoding='utf-8-sig') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
rows=read('data/round35_crops.csv');assert len(rows)==9
for r in rows:
 assert sha(R/r['file'])==r['sha256']
 assert sha(R/r['parent'])==r['parent_sha256']
 s=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(R/r['file'])],text=True)
 assert int(re.search(r'pixelWidth: (\d+)',s)[1])==int(r['width'])
 assert int(re.search(r'pixelHeight: (\d+)',s)[1])==int(r['height'])
reg={r['local_file']:r for r in read('SOURCE_REGISTER.csv')}
new=[r for k,r in reg.items() if k.startswith('round35_sources/')];assert len(new)==11
for r in new:assert sha(R/r['local_file'])==r['sha256']
for n in ['GI','ZI']:assert json.loads((R/f'round35_sources/eBL_{n}.json').read_text())['name']==n
assert len(read('data/round35_photographic_collation.csv'))==3
assert len(read('data/round35_tests.csv'))==5
assert len(read('data/round35_school_model_rows.csv'))==9
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['src','href'] and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round35.html').read_text())
for p in l.urls:assert (R/p).exists(),p
assert (R/'round35_sources/modern_glyph_reference.png').read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
result=dict(status='passed',crop_dimensions_and_hashes=9,external_reference_hashes=11,school_model_rows=9,local_viewer_links=len(l.urls),new_phonetic_values=0,book_PDF='unchanged')
(R/'data/round35_checkpoint.txt').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False,indent=2))
