"""Frozen scope: inventory pre-existing media; exclude this cycle's own outputs."""
from pathlib import Path
import csv,json,hashlib,re,subprocess,concurrent.futures
R=Path(__file__).resolve().parents[1]
reg={x['local_file']:x for x in csv.DictReader((R/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig'))}
rows=[];primary=[];docs=[]
for p in sorted(R.rglob('*')):
 if not p.is_file():continue
 rel=str(p.relative_to(R))
 if '__pycache__' in p.parts or rel.startswith('round33_sources/'):continue
 suf=p.suffix.lower()
 if suf not in ['.jpg','.jpeg','.png','.tif','.tiff','.svg','.pdf']:continue
 m=reg.get(rel,{});kind=m.get('type','unregistered_derived');url=m.get('source_url','')
 typ='pdf_document' if suf=='.pdf' else 'vector_figure' if suf=='.svg' else 'raster_image'
 isprimary=typ=='raster_image' and any(t in kind for t in ['photo','drawing','handcopy','publication_plate']) and not any(t in kind for t in ['metadata','viewer','reader','incomplete','storyboard'])
 row=dict(file=rel,media=typ,source_type=kind,source_url=url,bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),catalogue_id_candidates=';'.join(sorted(set(re.findall(r'P\d{6}',rel+' '+url)))),primary_review_candidate=isprimary,pdf_header_valid=p.read_bytes()[:5]==b'%PDF-' if suf=='.pdf' else '')
 rows.append(row)
 if isprimary:primary.append(dict(review_id='V'+str(len(primary)+1).zfill(3),**row))
 if suf=='.pdf':docs.append(row)
def write(name,rs):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=list(rs[0]));w.writeheader();w.writerows(rs)
write('round33_media_inventory.csv',rows);write('round33_primary_review.csv',primary);write('round33_document_inventory.csv',docs)
(R/'round33_sources/contact_input.json').write_text(json.dumps([dict(id=x['review_id'],path=str((R/x['file']).resolve()),label=x['file']) for x in primary],ensure_ascii=False,indent=2))
summary=dict(media_files=len(rows),raster_files=sum(x['media']=='raster_image' for x in rows),vector_files=sum(x['media']=='vector_figure' for x in rows),pdf_files=len(docs),valid_pdf_files=sum(x['pdf_header_valid'] is True for x in docs),unique_byte_hashes=len({x['sha256'] for x in rows}),primary_review_candidates=len(primary),source_url_groups=len({x['source_url'] for x in primary}),candidate_P_ids=len({t for x in primary for t in x['catalogue_id_candidates'].split(';') if t}))
(R/'data/round33_inventory_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2))
def get(x):
 r=subprocess.run(['pdfinfo',str(R/x['file'])],capture_output=True,text=True);m=re.search(r'^Pages:\s+(\d+)',r.stdout,re.M)
 return dict(file=x['file'],bytes=x['bytes'],pdfinfo_ok=r.returncode==0,pages=int(m.group(1)) if m else '',error=r.stderr[:130] if r.returncode else '',coverage='catalogued; full visual/textual reading not claimed')
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:pages=list(ex.map(get,docs))
write('round33_document_pages.csv',pages)
print(json.dumps(summary,indent=2))
