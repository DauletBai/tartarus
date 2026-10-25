from pathlib import Path
import csv,hashlib,re,json
R=Path(__file__).resolve().parents[1]
def rows(name):return list(csv.DictReader((R/'data'/name).open(encoding='utf-8-sig')))
manifest=rows('round23_file_manifest.csv')
for r in manifest:
 p=R/r['file'];assert p.stat().st_size==int(r['bytes']);assert hashlib.sha256(p.read_bytes()).hexdigest()==r['sha256']
with (R/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig') as f:register=list(csv.DictReader(f))
assert len(register)==len({r['local_file'] for r in register})
new=[r for r in register if r['local_file'].startswith('round23_sources/')]
for r in new:assert hashlib.sha256((R/r['local_file']).read_bytes()).hexdigest()==r['sha256']
f=rows('round23_parallel_formulae.csv');assert len(f)==12;assert all(r['matches_after_damage_marker_removal']=='True' for r in f);assert sum(r['damage_markers_present']=='True' for r in f)==3
lex=rows('round23_ancient_pronunciation_entries.csv');assert len(lex)==4
r=next(r for r in lex if r['line']=='r ii 16');assert r['written_pronunciation']=='ga-⸢ab?⸣';assert r['editorial_headword'].startswith('[[')
assert len(rows('round23_visual_collation.csv'))==8
for l in re.findall(r'(?:src|href)="([^"]+)"',(R/'comparison_round23.html').read_text()):
 if not l.startswith('https:'):assert (R/l).exists(),l
expected={'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5','Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}
for n,h in expected.items():assert hashlib.sha256((R.parents[1]/'book'/n).read_bytes()).hexdigest()==h
print(json.dumps(dict(status='PASS',new_sources=len(new),file_hashes=len(manifest),formulae=len(f),marked_damage=3,visual_observations=8,lexical_entries=len(lex),book_unchanged=True),ensure_ascii=False))
