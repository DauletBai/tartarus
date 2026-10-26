from pathlib import Path
import csv,hashlib,json,re,subprocess
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
R=Path(__file__).resolve().parents[1]
def read(p):
 with (R/p).open(encoding='utf-8-sig') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
rows=read('data/round36_crops.csv');assert len(rows)==11
for row in rows:
 assert sha(R/row['file'])==row['sha256'];assert sha(R/row['parent'])==row['parent_sha256']
 s=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(R/row['file'])],text=True)
 assert int(re.search(r'pixelWidth: (\d+)',s)[1])==int(row['width'])
 assert int(re.search(r'pixelHeight: (\d+)',s)[1])==int(row['height'])
school=read('data/round36_school_rows.csv');assert len(school)==12
assert {r['row_id'] for r in school}=={f'P229436.{i}' for i in range(58,70)}
pair=next(r for r in school if r['row_id']=='P229436.68')
assert pair['main_sign_model']=='UD.DU' and pair['pronunciation_model']=='* e',pair
assert len(read('data/round36_reading_chain.csv'))==5
assert len(read('data/round36_observation_decisions.csv'))==4
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ('src','href') and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round36.html').read_text())
for p in l.urls:assert (R/p).exists(),p
reg={r['local_file']:r for r in read('SOURCE_REGISTER.csv')}
parents=['round32_sources/P229436_photo.jpg','round32_sources/P229436_lineart.jpg','round32_sources/P229436_edition.html','round27_sources/PTS1501_photo.jpg','round27_sources/PTS1502_photo.jpg']
for p in parents:assert sha(R/p)==reg[p]['sha256']
assert (R/'round36_sources/modern_glyph_reference.png').read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
result=dict(status='passed',crop_dimensions_and_hashes=11,original_source_hashes=5,school_model_rows=12,local_links=len(l.urls),conditional_readings=1,new_independently_established_sound_values=0,book_PDF='unchanged')
(R/'data/round36_checkpoint.txt').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False,indent=2))
