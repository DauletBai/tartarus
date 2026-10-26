from pathlib import Path
import csv,json,hashlib,re,subprocess
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
R=Path(__file__).resolve().parents[1]
def read(p):
 with (R/p).open(encoding='utf-8-sig') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
reg=[r for r in read('SOURCE_REGISTER.csv') if r['local_file'].startswith('round38_sources/')]
assert len(reg)==16 and len({r['local_file'] for r in reg})==16
for r in reg:assert sha(R/r['local_file'])==r['sha256']
for r in read('data/round38_crops.csv'):
 assert sha(R/r['file'])==r['sha256'];assert sha(R/r['parent'])==r['parent_sha256']
 s=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(R/r['file'])],text=True)
 assert int(re.search(r'pixelWidth: (\d+)',s)[1])==int(r['width'])
 assert int(re.search(r'pixelHeight: (\d+)',s)[1])==int(r['height'])
p=read('data/round38_prefix_alignment.csv');assert len(p)==5
assert sum(r['decision'].startswith('retain') for r in p)==2
assert sum(r['decision'].startswith('exclude') for r in p)==2
assert sum(r['decision'].startswith('partial') for r in p)==1
assert next(r for r in p if r['canonical']=='209')['obverse_line']=='8'
assert next(r for r in p if r['canonical']=='209')['reverse_line']=='6'
assert all(r['normalized_sign_model'].split()[:2]==['ABZ60','ABZ308'] for r in p)
controls=read('data/round38_E_control_screen.csv');assert len(controls)==8
raw=json.loads((R/'round38_sources/eBL_BM33769.json').read_text())
assert 'ABZ308' in raw['signs'].split()
assert 's[u KUR-i e-lu-u₂-ti]' in raw['atf']
assert '[σουθ ικαβασ σαδη ε]λουθ' in raw['atf']
assert len(read('data/round38_competing_segmentations.csv'))==3
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,a):
  for k,v in a:
   if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round38.html').read_text())
for p in l.urls:assert (R/p).exists(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
out=dict(status='passed',source_hashes=16,crop_dimensions_and_hashes=4,aligned_contexts=5,retained_prefix_candidates=2,partial_prefix_controls=1,excluded_prefix_cases=2,additional_control_endpoints=8,restored_E_excluded=True,local_links=len(l.urls),new_fully_independent_sound_values=0,book_PDF='unchanged')
(R/'data/round38_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n');print(json.dumps(out,ensure_ascii=False,indent=2))
