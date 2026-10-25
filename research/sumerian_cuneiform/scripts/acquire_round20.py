from pathlib import Path
import concurrent.futures,json,re,subprocess,hashlib
B=Path(__file__).resolve().parents[1]
def get(u,p):
 if p.exists() and p.stat().st_size:return
 subprocess.run(['curl','-sSL','--fail','--max-time','45',u,'-o',str(p)],check=True,capture_output=True)
def one(nid):
 label=f'A{nid:02d}';r={'label':label,'pid':f'P{nid:06d}','card_url':f'https://cdli.earth/artifacts/{nid}'}
 try:
  card=B/f'sources/round20_{label}_card_withheld.html';get(r['card_url'],card);r['card_file']=str(card.relative_to(B));s=card.read_text();i=s.index('/tn_photo/');start=s.rfind('<a ',0,i);m=re.search(r'href="([^\"]*/reader/\d+)"',s[start:i]);assert m,'No photo reader'
  r['reader_url']='https://cdli.earth'+m[1];reader=B/f'sources/round20_{label}_reader_withheld.html';get(r['reader_url'],reader);r['reader_file']=str(reader.relative_to(B));m=re.search(r'<img src="([^\"]+/photo/[^\"]+)"',reader.read_text());assert m,'No full photograph'
  r['image_url']='https://cdli.earth'+m[1];im=B/f'sources/round20_{label}.jpg';get(r['image_url'],im);r['image_file']=str(im.relative_to(B));r['image_bytes']=im.stat().st_size;r['image_sha256']=hashlib.sha256(im.read_bytes()).hexdigest();r['status']='acquired_not_yet_visually_inspected'
 except Exception as e:r['status']='failed';r['error']=str(e)
 return r
if __name__=='__main__':
 p=json.loads((B/'data/round20_protocol.json').read_text());rows=[]
 with concurrent.futures.ThreadPoolExecutor(max_workers=3) as ex:
  for r in ex.map(one,p['sample_ids']):rows.append(r);print(r['label'],r['status'],r.get('image_bytes',''))
 (B/'data/round20_acquisition_manifest.json').write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n')
