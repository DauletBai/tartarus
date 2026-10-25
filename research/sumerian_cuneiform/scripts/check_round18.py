from pathlib import Path
from collections import Counter
from html.parser import HTMLParser
import csv,json,hashlib
b=Path(__file__).resolve().parents[1]
r=json.loads((b/'data/round18_target_nodes.json').read_text())
s=json.loads((b/'data/round18_corpus_summary.json').read_text())
assert len(r)==92 and len({x['qid'] for x in r})==57
assert len(s['forms'])==14 and sum(s['forms'].values())==92
assert s['files_matched']==1457 and s['files_parsed']==1456 and len(s['failed_files'])==1
assert all(x['f']['cf']=='utud' and x['f']['gw']=='bear' for x in r)
assert len({(x['qid'],x['ref'],x['f']['form']) for x in r})==92
assert r[0]['ref']=='Q000377.45.4'
a=[x for x in r if x['qid']=='Q001025']
assert [x['line_label'] for x in a]==['ii 2','iii 2','v 2']
assert all([g['gdl_sign'] for g in x['f']['gdl']]==['MU','TU'] for x in a)
with (b/'data/round18_control_tokens.csv').open(encoding='utf-8-sig') as f:c=list(csv.DictReader(f))
assert Counter(x['cf'] for x in c)=={'pad':147,'gid':11,'sud':26,'nud':22}
for name,h in json.loads((b/'data/round18_integrity.json').read_text()).items():assert hashlib.sha256((b/name).read_bytes()).hexdigest()==h,name
with (b/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig') as f:src=[x for x in csv.DictReader(f) if Path(x['local_file']).name.startswith('round18_')]
assert len(src)==12
for x in src:
 raw=(b/x['local_file']).read_bytes();assert len(raw)==int(x['bytes']);assert hashlib.sha256(raw).hexdigest()==x['sha256']
class Links(HTMLParser):
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['href','src'] and v and not v.startswith(('http:','https:','#')):assert (b/v).is_file(),v
Links().feed((b/'comparison_round18.html').read_text())
print('OK:92 unique target tokens,57 compositions,14 forms;206 controls;12 source hashes;local viewer links;3 Ur-Nanse loci. No independent sound readings asserted.')
