from pathlib import Path
import csv,json,hashlib,re
R=Path(__file__).resolve().parents[1];D=R/'data'
def rows(n):return list(csv.DictReader((D/n).open(encoding='utf-8-sig')))
manifest=rows('round25_file_manifest.csv')
for r in manifest:
 p=R/r['file'];assert p.stat().st_size==int(r['bytes']);assert hashlib.sha256(p.read_bytes()).hexdigest()==r['sha256']
with (R/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig') as f:reg=list(csv.DictReader(f))
assert len(reg)==len({r['local_file'] for r in reg})
new=[r for r in reg if r['local_file'].startswith('round25_sources/')]
for r in new:assert hashlib.sha256((R/r['local_file']).read_bytes()).hexdigest()==r['sha256']
a=rows('round25_nine_pronunciation_entries.csv');assert len(a)==9;assert sum(r['has_editorial_correction']=='True' for r in a)==1
assert next(r for r in a if r['row_id']=='P227708.26')['pronunciation']=='* ga-ba!'
assert all(not r['akkadian'] for r in a)
assert len(rows('round25_control_readings.csv'))==8
controls=rows('round25_control_readings.csv');rm=next(r for r in controls if r['source']=='P240227_oracc.html' and r['line']=="i 9'");assert rm['pronunciation']=='[...]' and rm['main_sign']=='[GU₂]'
summary=json.loads((D/'round25_extraction_summary.json').read_text());assert 'bullet' in summary['notes']
for l in re.findall(r'(?:href|src)="([^"]+)"',(R/'comparison_round25.html').read_text()):
 if not l.startswith(('https:','#')):assert (R/l).exists(),l
expected={'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5','Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}
for n,h in expected.items():assert hashlib.sha256((R.parents[1]/'book'/n).read_bytes()).hexdigest()==h
print(json.dumps(dict(status='PASS',sources=len(new),file_hashes=len(manifest),selected_entries=9,corrections=1,damaged_control_not_counted_as_complete=True,book_unchanged=True),ensure_ascii=False))
