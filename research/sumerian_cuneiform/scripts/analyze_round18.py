"""Audit a fixed editorial corpus; does not decode photographs or prove ancestry."""
from pathlib import Path
import collections,csv,hashlib,json,re,zipfile
from census_possessives import nodes
from check_round11 import line_labels
B=Path(__file__).resolve().parents[1]
def main():
 out=[];control_rows=[];controls=collections.Counter();failed=[];seen=set();files=0
 archive=B/'sources/ETCSRI_2026-10-24_json.zip'
 assert hashlib.sha256(archive.read_bytes()).hexdigest()==json.loads((B/'data/round18_protocol.json').read_text())['archive_sha256']
 with zipfile.ZipFile(archive) as z:
  cat=json.loads(z.read('etcsri/catalogue.json'))['members']
  for fn in sorted(z.namelist()):
   if not re.search(r'/corpusjson/Q\d+\.json$',fn):continue
   files+=1
   try:doc=json.loads(z.read(fn))
   except (ValueError,UnicodeError) as e:failed.append({'file':fn,'error':str(e)});continue
   ls=line_labels(doc);ns=list(nodes(doc));qid=Path(fn).stem
   for i,n in enumerate(ns):
    f=n['f'];cf=f.get('cf','');gw=f.get('gw','');ref=n.get('ref','');key=(qid,ref,f.get('form',''))
    if key in seen:continue
    seen.add(key)
    if not f.get('lang','').startswith('sux'):continue
    if cf.endswith('d') and f.get('pos','').startswith('V'):controls[(cf,gw)]+=1
    if cf in ['pad','gid','sud','nud']:
     control_rows.append({'qid':qid,'ref':ref,'line_label':ls.get(ref.rsplit('.',1)[0],''),**{k:f.get(k,'') for k in ['cf','gw','form','base','norm','morph','morph2']}})
    if cf not in ['utud','tud']:continue
    out.append({'qid':qid,'ref':ref,'line_label':ls.get(ref.rsplit('.',1)[0],''),'f':f,'context_before':[v['f'].get('form','') for v in ns[max(0,i-4):i]],'context_after':[v['f'].get('form','') for v in ns[i+1:i+5]],'metadata':cat.get(qid,{})})
 (B/'data/round18_target_nodes.json').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
 (B/'data/round18_d_final_verb_inventory.json').write_text(json.dumps([{'cf':c,'gw':g,'tokens':n} for (c,g),n in sorted(controls.items())],ensure_ascii=False,indent=2)+'\n')
 summary={'files_matched':files,'files_parsed':files-len(failed),'failed_files':failed,'target_tokens':len(out),'target_compositions':len({r['qid'] for r in out}),'forms':dict(collections.Counter(r['f'].get('form','') for r in out)),'not_physical_witness_counts':True}
 (B/'data/round18_corpus_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
 with (B/'data/round18_control_tokens.csv').open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(control_rows[0]));w.writeheader();w.writerows(control_rows)
 fields=['qid','ref','line_label','form','cf','gw','sense','norm','morph','morph2','before','after','designation']
 with (B/'data/round18_target_tokens.csv').open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader()
  for r in out:
   row={k:r['f'].get(k,'') for k in fields};row.update(qid=r['qid'],ref=r['ref'],line_label=r['line_label'],before=' '.join(r['context_before']),after=' '.join(r['context_after']),designation=r['metadata'].get('designation',''));w.writerow(row)
 print(json.dumps(summary,ensure_ascii=False))
 for r in out[:8]:print(r['qid'],r['line_label'],r['f'].get('form'),r['f'].get('sense'),r['f'].get('norm'))
 print('Controls:',controls.most_common(12))
 if out:print('First metadata:',json.dumps(out[0]['metadata'],ensure_ascii=False))
if __name__=='__main__':main()
