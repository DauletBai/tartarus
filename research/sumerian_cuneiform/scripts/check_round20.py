from pathlib import Path
from collections import Counter
from html.parser import HTMLParser
import csv,hashlib,json
B=Path(__file__).resolve().parents[1];D=B/'data'
def load(n):return json.loads((D/n).read_text())
p=load('round20_protocol.json');a=load('round20_acquisition_manifest.json');freeze=load('round20_preheldout_freeze.json')
assert len(a)==12 and p['sample_ids']==list(range(4,16))
assert freeze['heldout_pixels_opened'] is False
for n,h in freeze['records'].items():assert hashlib.sha256((D/n).read_bytes()).hexdigest()==h,n
for r in a:
 assert r['status']=='acquired_not_yet_visually_inspected' # acquisition-time state, not current state
 assert hashlib.sha256((B/r['image_file']).read_bytes()).hexdigest()==r['image_sha256']
assert len({r['image_sha256'] for r in a})==12
pilot=load('round20_pilot_regions.json');parts=load('round20_alternative_partitions.json')
assert len(pilot['regions'])==22 and sum(r['boundary_status']=='clear' for r in pilot['regions'])==11
assert len(parts['fine'])==22 and len(parts['coarse'])==19
assert sorted(x for g in parts['fine'] for x in g)==sorted(x for g in parts['coarse'] for x in g)
for r in pilot['regions']:
 x,y,w,h=r['box_xywh'];assert 0<=x<x+w<=978 and 0<=y<y+h<=1312
 assert r['sound'] is None and r['meaning'] is None
inv=load('round20_graphic_inventory.json')
assert len(inv['inventory'])==13 and len(inv['geometric_trace_categories'])==5
assert sum(r['status']=='provisional_graphic_type' for r in inv['inventory'])==12
for r in inv['inventory']:
 assert all(r[k] is None for k in ['sound','syllable','word_meaning','canonical_identity'])
 assert (B/r['image_or_crop']).is_file()
with (D/'round20_heldout_matches.csv').open(encoding='utf-8-sig') as f:r=list(csv.DictReader(f))
assert len(r)==24 and Counter(x['status'] for x in r)=={'strict_visual_candidate':1,'partial_visual_candidate':10,'not_observed_in_visible_area':7,'unassessable':6}
obs=load('round20_heldout_observations.json');assert obs['criteria_changed_after_viewing'] is False and obs['individual_readings_opened'] is False
assert Counter(x['status'] for x in obs['layout'])=={'positive':3,'unassessable':1}
models=load('round20_topology_models.json');assert [m['cycle_rank'] for m in models]==[0,2,2,3]
for m in models:assert sum(m['degree_multiset'])==2*len(m['edges'])
# Repetition cannot distinguish a global permutation of arbitrary sound labels.
x=[g for r in pilot['regions'] for g in r['candidate_groups']];u=list(dict.fromkeys(x));perm={v:u[(i+1)%len(u)] for i,v in enumerate(u)};y=[perm[v] for v in x]
assert x!=y and [[v==w for w in x] for v in x]==[[v==w for w in y] for v in y]
with (B/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig') as f:reg=[x for x in csv.DictReader(f) if Path(x['local_file']).name.startswith('round20_')]
assert len(reg)==36
for x in reg:
 raw=(B/x['local_file']).read_bytes();assert len(raw)==int(x['bytes']) and hashlib.sha256(raw).hexdigest()==x['sha256']
class Links(HTMLParser):
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['src','href','data'] and v and not v.startswith(('https:','http:','#','data:')):assert (B/v).is_file(),v
Links().feed((B/'comparison_round20.html').read_text())
msg='OK:12 photos;36 original source hashes;22/19 alternative regions;12 graphic types+1 unresolved;24 fixed-criteria assessments;4 topology models;phonetic relabel invariance;all local atlas links.\n'
(D/'round20_validation.txt').write_text(msg);print(msg)
