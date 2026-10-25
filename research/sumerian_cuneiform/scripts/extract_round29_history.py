from pathlib import Path
import re,html,csv,json
R=Path(__file__).resolve().parents[1];records=[]
for pid in ['P201507','P431422','P216745']:
 s=(R/'round29_sources'/f'{pid}_history.html').read_text()
 for i,m in enumerate(re.finditer(r'<pre[^>]*>(.*?)</pre>',s,re.S)):
  before=s[max(0,m.start()-1800):m.start()];dates=re.findall(r'\d{4}-\d{2}-\d{2}(?: at \d{2}:\d{2}:\d{2})?',before)
  raw=html.unescape(re.sub('<[^>]*>','',m[1]));current=html.unescape(re.sub('<[^>]*>','',re.sub(r'<del\b[^>]*>.*?</del>','',m[1],flags=re.S)))
  line10=' / '.join(x for x in current.splitlines() if re.match(r'10\.',x))
  trans=' / '.join(x for x in current.splitlines() if '#tr.en:' in x and ('water' in x or 'front' in x))
  records.append(dict(pid=pid,entry_index=i,date=dates[-1] if dates else '',line10=line10,water_translation=trans,legacy_sheep_active_in_record='gaba[~sheep]' in current,raw_excerpt=raw,current_text_in_record=current))
(R/'data/round29_history_entries.json').write_text(json.dumps(records,ensure_ascii=False,indent=2))
flat=[{k:v for k,v in x.items() if k not in ['raw_excerpt','current_text_in_record']} for x in records]
with (R/'data/round29_history_summary.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=list(flat[0]));w.writeheader();w.writerows(flat)
print('history records',len(records))
