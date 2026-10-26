from pathlib import Path
import csv,json,hashlib
from html.parser import HTMLParser
from urllib.parse import urlsplit,unquote
R=Path(__file__).resolve().parents[1]
def read(n):
 with (R/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((R/'data/round31_summary.json').read_text())
assert (s['concordance_contexts'],s['concordance_objects'],s['context_word_occurrences'])==(24,17,25)
rs=read('data/round31_praxis_contexts.csv');assert len(rs)==24 and sum(int(x['same_form_tokens_in_context']) for x in rs)==25
assert [x['ref'] for x in rs if int(x['same_form_tokens_in_context'])>1]==['P343032.3']
assert len(read('data/round31_target_rows.csv'))==8 and len(read('data/round31_early_complete_texts.csv'))==30
assert len(read('data/round31_visual_audit.csv'))==11 and len(read('data/round31_hypothesis_tests.csv'))==7
ctl=read('data/round31_UD_tense_control.csv');assert len(ctl)==2 and all(x['signs'].startswith('UD') for x in ctl)
src=[x for x in read('SOURCE_REGISTER.csv') if x['local_file'].startswith('round31_sources/')]
assert len(src)==22==len({x['local_file'] for x in src})
for x in src:
 p=R/x['local_file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
 if p.suffix=='.jpg':assert p.read_bytes()[:2]==b'\xff\xd8',p
mf=read('data/round31_file_manifest.csv')
for x in mf:
 p=R/x['file'];assert p.stat().st_size==int(x['bytes']) and sha(p)==x['sha256'],p
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,attrs):
  for k,v in attrs:
   if k in ['href','src'] and v and not urlsplit(v).scheme and not v.startswith('#'):self.urls.append(unquote(v.split('#')[0]))
l=Links();l.feed((R/'comparison_round31.html').read_text())
for p in l.urls:
 if p!='data/round31_checkpoint.txt':assert (R/p).exists(),p
for file,digest in {
 'Тартар_и_Я_рабочая_редакция.md':'a82e8a91f0e8d40738dfb7eda1ff40dcb38a0f49067b5556d6588e11344d3fc5',
 'Tartar_i_Ya_print_6x9.pdf':'ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'}.items():assert sha(R.parents[1]/'book'/file)==digest,file
out=dict(status='passed',contexts=24,objects_in_concordance=17,word_occurrences=25,target_rows=8,complete_early_lines=30,source_hashes=len(src),manifest_hashes=len(mf),local_links=len(l.urls),book_hashes='unchanged')
(R/'data/round31_checkpoint.txt').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n');assert all((R/p).exists() for p in l.urls)
print(json.dumps(out,ensure_ascii=False,indent=2))
