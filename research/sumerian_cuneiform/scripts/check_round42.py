from pathlib import Path
import csv,hashlib,json,re,subprocess
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
R=Path(__file__).resolve().parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def rows(p):
 with (R/p).open(encoding='utf-8-sig') as f:return list(csv.DictReader(f))
reg=[x for x in rows('SOURCE_REGISTER.csv') if x['local_file'].startswith('round42_sources/')]
assert len(reg)==6
for x in reg:
 p=R/x['local_file'];assert sha(p)==x['sha256'] and p.stat().st_size==int(x['bytes'])
screen=rows('data/round42_MIN_screen.csv');assert len(screen)==13
for row in screen:
 x=json.loads((R/row['source']).read_text());lines=[l for l in x.get('text',{}).get('lines',[]) if l.get('type')=='TextLine']
 hits=sum(any(z.get('value')=='MIN' for z in l.get('content',[])) for l in lines)
 assert len(lines)==int(row['editorial_text_lines']) and hits==int(row['explicit_MIN_hits'])
assert sum(int(x['explicit_MIN_hits']) for x in screen)==2
for x in rows('data/round42_crops.csv'):
 p=R/x['file'];assert sha(p)==x['sha256'] and sha(R/x['parent'])==x['parent_sha256']
 assert p.read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
 info=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(p)],text=True)
 assert int(re.search(r'pixelWidth: (\d+)',info)[1])==int(x['width'])
 assert int(re.search(r'pixelHeight: (\d+)',info)[1])==int(x['height'])
for x in rows('data/round42_previews.csv'):
 assert sha(R/x['file'])==x['sha256'] and sha(R/x['parent'])==x['parent_sha256']
assert len(rows('data/round42_preservation_tests.csv'))==3
for x in rows('synthesis/evidence_for_book.csv'):assert (R/x['local_report']).is_file()
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,a):
  for k,v in a:
   if k in ('href','src') and v and not urlsplit(v).scheme:self.urls.append(unquote(v))
l=Links();l.feed((R/'comparison_round42.html').read_text())
for p in l.urls:assert (R/p).is_file(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
print('Passed:6 original source hashes,13 editorial screens,5 crops,2 previews,3 preservation tests,cumulative synthesis references,16 local links; book unchanged.')
