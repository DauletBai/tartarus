from pathlib import Path
import csv,json,hashlib,re,subprocess
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
R=Path(__file__).resolve().parents[1]
def read(p):
 with (R/p).open(encoding='utf-8-sig') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
reg=[r for r in read('SOURCE_REGISTER.csv') if r['local_file'].startswith('round39_sources/')];assert len(reg)==2
for r in reg:assert sha(R/r['local_file'])==r['sha256']
def info(p):return subprocess.check_output(['pdfinfo',str(R/p)],text=True)
assert re.search(r'Pages:\s+482\b',info('round39_sources/PSBA24_1902.pdf'))
assert re.search(r'Pages:\s+18\b',info('round39_sources/Pinches1902_GreekTranscriptions.pdf'))
for r in read('data/round39_image_transforms.csv'):
 assert sha(R/r['parent'])==r['parent_sha256'];assert sha(R/r['file'])==r['sha256']
 assert r['operation'].startswith('horizontal reflection')
for r in read('data/round39_crops.csv'):
 assert sha(R/r['parent'])==r['parent_sha256'];assert sha(R/r['file'])==r['sha256']
 s=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(R/r['file'])],text=True)
 assert int(re.search(r'pixelWidth: (\d+)',s)[1])==int(r['width'])
 assert int(re.search(r'pixelHeight: (\d+)',s)[1])==int(r['height'])
assert len(read('data/round39_greek_prefix_collation.csv'))==3
assert len(read('data/round39_historical_comparison.csv'))==5
assert len(read('data/round39_page_map.csv'))==6
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,a):
  for k,v in a:
   if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round39.html').read_text())
for p in l.urls:assert (R/p).exists(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
out=dict(status='passed',external_source_hashes=2,original_volume_pages=482,article_excerpt_pages=18,recorded_orientation_operations=1,crop_dimensions_and_hashes=6,Greek_prefixes_located=3,local_links=len(l.urls),new_complete_cuneiform_readings=0,book_PDF='unchanged')
(R/'data/round39_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n');print(json.dumps(out,ensure_ascii=False,indent=2))
