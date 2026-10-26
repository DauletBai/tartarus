from pathlib import Path
import csv,json,hashlib,re,subprocess
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
R=Path(__file__).resolve().parents[1]
def read(p):
 with (R/p).open(encoding='utf-8-sig') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
reg=[r for r in read('SOURCE_REGISTER.csv') if r['local_file'].startswith('round37_sources/')]
assert len(reg)==18 and len({r['local_file'] for r in reg})==18
for r in reg:assert sha(R/r['local_file'])==r['sha256']
crops=read('data/round37_crops.csv');assert len(crops)==9
for r in crops:
 assert sha(R/r['file'])==r['sha256'];assert sha(R/r['parent'])==r['parent_sha256']
 s=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(R/r['file'])],text=True)
 assert int(re.search(r'pixelWidth: (\d+)',s)[1])==int(r['width'])
 assert int(re.search(r'pixelHeight: (\d+)',s)[1])==int(r['height'])
ed=read('data/round37_editorial_rows.csv');assert len(ed)==12
assert next(x for x in ed if x['row_id']=='P227815.9')['main_sign_model']=='[UD.DU]'
assert next(x for x in ed if x['row_id']=='P228706.62')['main_sign_model']=='UD.DU'
assert next(x for x in ed if x['row_id']=='P386409.13')['main_sign_model']!='UD.DU'
greek=read('data/round37_greek_controls.csv');assert len(greek)==7
assert next(x for x in greek if x['canonical_line']=='207')['greek_model']=='[ε]'
assert "4'. %suxgrc [ε]" in (R/'round37_sources/BM34797_editorial.atf').read_text()
assert json.loads((R/'round37_sources/eBL_BM34950.json').read_text())['hasPhoto'] is True
assert json.loads((R/'round37_sources/eBL_BM34797.json').read_text())['hasPhoto'] is False
assert (R/'round37_sources/BM34950_photo.jpg').read_bytes()[:2]==b'\xff\xd8'
assert (R/'round37_sources/P370916_photo.jpg').read_bytes()[:2]==b'\xff\xd8'
assert len(read('data/round37_index_leads.csv'))==11
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,a):
  for k,v in a:
   if k in ('src','href') and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round37.html').read_text())
for p in l.urls:assert (R/p).exists(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
result=dict(status='passed',external_source_hashes=18,crop_dimensions_and_hashes=9,editorial_rows=12,external_alphabet_candidates=7,restored_epsilon_excluded=True,local_links=len(l.urls),new_independently_established_sound_values=0,book_PDF='unchanged')
(R/'data/round37_checkpoint.txt').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False,indent=2))
