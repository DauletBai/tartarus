from pathlib import Path
import csv,json,hashlib,collections
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
R=Path(__file__).resolve().parents[1]
def read(n):
 with (R/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round33_summary.json').read_text())
assert (s['media_files'],s['raster_files'],s['vector_files'],s['pdf_files'])==(282,256,2,24)
assert (s['valid_pdf_files'],s['document_pages'],s['reviewed_overview_images'])==(23,5229,95)
inv=read('data/round33_media_inventory.csv');assert len(inv)==282 and len({x['file'] for x in inv})==282
for x in inv:
 p=R/x['file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
pr=read('data/round33_primary_review.csv');rv=read('data/round33_visual_overview.csv');assert len(pr)==len(rv)==95
assert [x['review_id'] for x in pr]==[x['review_id'] for x in rv]==['V'+str(i).zfill(3) for i in range(1,96)]
assert all(x['sounds_assigned']=='0' for x in rv)
reg={x['local_file']:x for x in read('SOURCE_REGISTER.csv')}
for x in pr:assert x['sha256']==reg[x['file']]['sha256']
assert collections.Counter(x['layout_family'] for x in rv)==s['layout_families']
pages=read('data/round33_document_pages.csv');assert len(pages)==24 and sum(int(x['pages']) for x in pages if x['pages'])==5229
assert len(read('data/round33_micro_observations.csv'))==4
for i in range(1,7):assert (R/f'round33_sources/contact_{i:02d}.jpg').read_bytes()[:2]==b'\xff\xd8'
mf=read('data/round33_file_manifest.csv')
for x in mf:
 p=R/x['file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['href','src'] and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round33.html').read_text())
for p in l.urls:
 if p!='data/round33_checkpoint.txt':assert (R/p).exists(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
out=dict(status='passed',inventoried_media=282,original_image_source_hashes=95,review_entries=95,document_pages_catalogued=5229,contact_sheets=6,micro_observations=4,manifest_hashes=len(mf),local_links=len(l.urls),new_phonetic_values=0,book_PDF='unchanged')
(R/'data/round33_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n');assert all((R/p).exists() for p in l.urls)
print(json.dumps(out,ensure_ascii=False,indent=2))
