from pathlib import Path
from html.parser import HTMLParser
import json,re,subprocess,hashlib
B=Path(__file__).resolve().parents[1]
class P(HTMLParser):
 def __init__(self):super().__init__();self.t=[]
 def handle_data(self,s):
  if s.strip():self.t.append(s.strip())
def get(u,p):
 if p.exists() and p.stat().st_size:return
 subprocess.run(['curl','-sSL','--fail','--max-time','35',u,'-o',str(p)],check=True,capture_output=True)
def main():
 protocol=json.loads((B/'data/round21_selection_protocol.json').read_text());rows=[];audit=[]
 for nid in protocol['candidate_ids']:
  r={'pid':f'P{nid:06d}','card_url':f'https://cdli.earth/artifacts/{nid}'}
  try:
   card=B/f'sources/round21_P{nid:06d}_card_withheld.html';get(r['card_url'],card);r['card_file']=str(card.relative_to(B));s=card.read_text();p=P();p.feed(s)
   for field,key in [('Period','period'),('Language(s)','language')]:
    i=p.t.index(field);r[key]=p.t[i+1]
   if r['period']!='Ur III (ca. 2100-2000 BC)' or r['language']!='Sumerian':r['status']='metadata_not_eligible';audit.append(r);continue
   i=s.index('/tn_photo/');start=s.rfind('<a ',0,i);m=re.search(r'href="([^\"]*/reader/\d+)"',s[start:i]);assert m
   r['label']=f'U{len(rows)+1:02d}';r['reader_url']='https://cdli.earth'+m[1];rp=B/f'sources/round21_{r["label"]}_reader_withheld.html';get(r['reader_url'],rp);r['reader_file']=str(rp.relative_to(B));m=re.search(r'<img src="([^\"]+/photo/[^\"]+)"',rp.read_text());assert m
   r['image_url']='https://cdli.earth'+m[1];ip=B/f'sources/round21_{r["label"]}.jpg';get(r['image_url'],ip);r['image_file']=str(ip.relative_to(B));r['sha256']=hashlib.sha256(ip.read_bytes()).hexdigest();r['status']='selected_pixels_unopened';rows.append(r)
   print(r['label'],'selected; catalogue language/period eligible; pixels unopened')
  except Exception as e:r['status']='acquisition_failure';r['error']=str(e)
  audit.append(r)
  if len(rows)==8:break
 (B/'data/round21_acquisition_manifest.json').write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n');(B/'data/round21_candidate_audit.json').write_text(json.dumps(audit,ensure_ascii=False,indent=2)+'\n')
 assert len(rows)==8,'Keep partial outcomes; sample not complete'
if __name__=='__main__':main()
