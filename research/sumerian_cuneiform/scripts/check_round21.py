from pathlib import Path
from collections import Counter
from html.parser import HTMLParser
import csv,hashlib,json
B=Path(__file__).resolve().parents[1];D=B/'data'
def load(n):return json.loads((D/n).read_text())
f=load('round21_preheldout_freeze.json');assert f['heldout_pixels_opened'] is False
for n,h in f['records'].items():assert hashlib.sha256((D/n).read_bytes()).hexdigest()==h,n
m=load('round21_prior_sample_metadata.json');assert m['individual_editorial_text_opened'] is False
r=m['records'];assert len(r)==15
assert Counter(x['period'] for x in r)=={'Uruk III (ca. 3200-3000 BC)':10,'Uruk IV (ca. 3350-3200 BC)':5}
assert all(x['language']=='Undetermined' for x in r)
a=load('round21_acquisition_manifest.json');assert len(a)==8
assert [x['label'] for x in a]==[f'U{i:02}' for i in range(1,9)]
assert len({x['sha256'] for x in a})==8
for x in a:
 assert x['period']=='Ur III (ca. 2100-2000 BC)' and x['language']=='Sumerian'
 assert hashlib.sha256((B/x['image_file']).read_bytes()).hexdigest()==x['sha256']
with (D/'round21_heldout_matches.csv').open(encoding='utf-8-sig') as f:r=list(csv.DictReader(f))
assert len(r)==12
assert Counter(x['status'] for x in r)=={'strict_visual_candidate':5,'partial_visual_candidate':5,'unassessable':1,'not_observed':1}
for x in r:assert (B/x['crop']).is_file()
units=load('round21_late_inventory.json');assert len(units)==3
for x in units:
 assert x['sound'] is None and x['syllable'] is None and x['meaning'] is None and x['minimal_graphic_unit_proven'] is False
assert load('round21_crop_review.json')['fixed_criteria_rewritten'] is False
assert len(load('round21_U04_bands.json'))==10
for c in load('round21_crop_manifest.json'):
 assert (B/c['crop']).is_file();x,y,w,h=c['box_xywh'];assert min(x,y)>=0 and min(w,h)>0
 src=next(x for x in a if x['image_file']==c['source']);assert x+w<=src['width'] and y+h<=src['height'],c
with (B/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig') as f:src=[x for x in csv.DictReader(f) if Path(x['local_file']).name.startswith('round21_')]
assert len(src)==26
for x in src:
 raw=(B/x['local_file']).read_bytes();assert len(raw)==int(x['bytes']) and hashlib.sha256(raw).hexdigest()==x['sha256']
class Links(HTMLParser):
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['src','href','data'] and v and not v.startswith(('http:','https:','#','data:')):assert (B/v).is_file(),v
Links().feed((B/'comparison_round21.html').read_text())
msg='OK:15 metadata records;8 stratified photos;12 frozen-criteria assessments;10 mapped bands;all crops within originals;26 source hashes;no sound values;atlas links valid.\n'
(D/'round21_validation.txt').write_text(msg);print(msg)
