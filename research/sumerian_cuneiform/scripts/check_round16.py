from pathlib import Path
from collections import Counter
import csv,hashlib,json
b=Path(__file__).resolve().parents[1]
r=list(csv.DictReader((b/'data/round16_collation.csv').open(encoding='utf-8-sig')))
assert [int(x['CS_row']) for x in r]==list(range(79,92))
assert len({x['editorial_left_normalized'] for x in r[:7]})==6
assert sum(bool(x['duplicate_row']) for x in r[:8])==7
assert sum(bool(x['duplicate_row']) for x in r[8:])==5
assert all(x['independently_recovered_sound']=='False' for x in r)
for name,h in json.loads((b/'data/round16_integrity.json').read_text()).items():
 assert hashlib.sha256((b/name).read_bytes()).hexdigest()==h,name
# Relabel every distinct editorial reading bijectively. Equality/incidence cannot identify sound.
a=[x['editorial_left_normalized'] for x in r]
labels=list(dict.fromkeys(a)); mapping={v:labels[(i+1)%len(labels)] for i,v in enumerate(labels)}
z=[mapping[v] for v in a]
assert a!=z
assert [[v==w for w in a] for v in a]==[[v==w for w in z] for v in z]
assert Counter(Counter(a).values())==Counter(Counter(z).values())
with (b/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig') as f:
 registered=[x for x in csv.DictReader(f) if Path(x['local_file']).name.startswith('round16_')]
assert len(registered)==12
for x in registered:
 p=b/x['local_file'];raw=p.read_bytes()
 assert len(raw)==int(x['bytes']) and hashlib.sha256(raw).hexdigest()==x['sha256']
e=json.loads((b/'sources/round16_eBL_BM93042.json').read_text())
assert e['hasPhoto'] is False and e['accession']['prefix']=='1885,0430' and e['accession']['number']=='6'
assert e['cdliImages']==['dl/lineart/P365263_l.jpg']
print('OK:13 rows;7+5 editorial overlaps;6 complete G1 readings;12 source hashes; sound-label permutation preserves repetition.')
