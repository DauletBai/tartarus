from pathlib import Path
import csv, hashlib, json, re, subprocess
from html.parser import HTMLParser
from urllib.parse import urlsplit, unquote
R=Path(__file__).resolve().parents[1]
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def rows(p):
    with (R/p).open(encoding='utf-8-sig') as f: return list(csv.DictReader(f))
reg=[r for r in rows('SOURCE_REGISTER.csv') if r['local_file'].startswith('round40_sources/')]
assert len(reg)==3
for r in reg:
    p=R/r['local_file']; assert sha(p)==r['sha256']; assert p.stat().st_size==int(r['bytes'])
assert (R/'round40_sources/BM35726_photo.jpg').read_bytes()[:3]==b'\xff\xd8\xff'
tur=json.loads((R/'round40_sources/eBL_TUR.json').read_text())
assert any(x['name']=='ABZ' and x['number']=='144' for x in tur['lists'])
osl=(R/'round40_sources/OSL_GA2xAN.html').read_text()
assert '>ama<' in osl and '>dagal<' in osl
for r in rows('data/round40_crops.csv'):
    p=R/r['file']; assert sha(p)==r['sha256']; assert sha(R/r['parent'])==r['parent_sha256']
    if p.suffix=='.png': assert p.read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
    s=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(p)],text=True)
    assert int(re.search(r'pixelWidth: (\d+)',s)[1])==int(r['width'])
    assert int(re.search(r'pixelHeight: (\d+)',s)[1])==int(r['height'])
for r in rows('data/round40_page_renders.csv'):
    assert sha(R/r['file'])==r['sha256']; assert sha(R/r['parent'])==r['parent_sha256']
assert len(rows('data/round40_paired_rows.csv'))==4
assert len(rows('data/round40_hypothesis_tests.csv'))==7
class Links(HTMLParser):
    def __init__(self): super().__init__();self.urls=[]
    def handle_starttag(self,t,a):
        for k,v in a:
            if k in ('href','src') and v and not urlsplit(v).scheme and not v.startswith('#'):
                self.urls.append(unquote(v.split('#')[0]))
l=Links(); l.feed((R/'comparison_round40.html').read_text())
for p in l.urls: assert (R/p).is_file(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
out=dict(status='passed',external_source_hashes=3,crop_dimensions_and_hashes=8,page_render_hashes=3,paired_context_records=4,hypothesis_tests=7,local_links=len(l.urls),book_PDF='unchanged',new_independent_complete_readings=0)
(R/'data/round40_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(out,ensure_ascii=False,indent=2))
