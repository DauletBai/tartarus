from html.parser import HTMLParser
from pathlib import Path
import json,csv,re
R=Path(__file__).resolve().parents[1]
class N:
 def __init__(self,tag='',a=()):self.tag=tag;self.a=dict(a);self.children=[]
 def text(self):return ''.join(c.text() if isinstance(c,N) else c for c in self.children)
 def all(self,tag=None,cls=None):
  for c in self.children:
   if isinstance(c,N):
    if (tag is None or c.tag==tag) and (cls is None or cls in c.a.get('class','').split()):yield c
    yield from c.all(tag,cls)
class P(HTMLParser):
 def __init__(self):super().__init__(convert_charrefs=True);self.root=N();self.stack=[self.root]
 def handle_starttag(self,t,a):
  n=N(t,a);self.stack[-1].children.append(n)
  if t not in ['meta','link','img','hr','br','input']:self.stack.append(n)
 def handle_startendtag(self,t,a):self.stack[-1].children.append(N(t,a))
 def handle_endtag(self,t):
  for i in range(len(self.stack)-1,0,-1):
   if self.stack[i].tag==t:self.stack=self.stack[:i];break
 def handle_data(self,t):self.stack[-1].children.append(t)
def out(n,rows):
 with (R/'data'/n).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
p=P();p.feed((R/'round23_sources/C1_oracc.html').read_text())
allrows=[];current='';pairs={}
for tr in p.root.all('tr'):
 td=next(tr.all('td','tlit'),None)
 if not td:continue
 x=next(tr.all('span','xlabel'),None)
 if x:current=' '.join(x.text().split())
 words=list(td.all('span','w'));tokens=[w.text() for w in words]
 lang='sux' if any('sux' in w.a.get('class','').split() for w in words) else 'akk_or_logographic'
 if not tokens:continue
 rec=dict(line=current,language=lang,tokens=tokens,editorial_text=' '.join(tokens),html_row_id=tr.a.get('id',''))
 allrows.append(rec);pairs.setdefault(current,{})[lang]=rec
(R/'data/round23_C1_extracted.json').write_text(json.dumps(allrows,ensure_ascii=False,indent=2))
selected=['o 10','o 11','o 12','o 13','o 14','o 15','r 1','r 2','r 3','r 4','r 5','r 6']
meanings=['голова','шея','грудь','правая сторона','левая сторона','кровь','сердце/внутренность','рёбра','позвоночник','верхняя часть бедра','поясница','члены тела']
kaz=['бас','мойын','кеуде','оң жақ','сол жақ','қан','жүрек','қабырға','омыртқа','сан','бел','мүшелер']
def clean(t):return re.sub(r'[\[\]⸢⸣<>]','',t)
rows=[]
for line,meaning,k in zip(selected,meanings,kaz):
 a=pairs[line]['sux'];b=pairs[line]['akk_or_logographic'];ts=a['tokens'];ct=[clean(t) for t in ts]
 pos=ct.index('maš₂'); X=ct[:pos];after=ct[pos+1:]
 ok=(after[:len(X)]==X and after[len(X):]==['lu₂-še₃','ba-an-šum₂'])
 rows.append(dict(line=line,sumerian=a['editorial_text'],akkadian=b['editorial_text'],variable_X=' '.join(X),model='X maš₂ X lu₂-še₃ ba-an-šum₂',matches_after_damage_marker_removal=ok,damage_markers_present=any(any(x in t for x in '[]⸢⸣<>') for t in ts),meaning_from_editorial_bilingual=meaning,kazakh_semantic_comparison=k,phonetic_descent_status='не установлено',source='ORACC blms/P338491; Schramm2008 §4',note='Нормализация убирает лишь скобки; восстановленные/повреждённые знаки не становятся физически сохранившимися'))
out('round23_parallel_formulae.csv',rows)
print('extracted rows',len(allrows),'pairs',len(pairs),'formula matches',sum(r['matches_after_damage_marker_removal'] for r in rows),'of',len(rows),'damage',sum(r['damage_markers_present'] for r in rows))
for r in rows:print(r['line'],r['sumerian'],'||',r['akkadian'])
# Separate lexical evidence; retain all editorial marks and the editorial headword column.
p=P();p.feed((R/'round23_sources/lexical_P333168_oracc.html').read_text())
lex=[]
for tr in p.root.all('tr'):
 cells=[t for t in tr.children if isinstance(t,N) and t.tag=='td']
 if not cells:continue
 xlabel=next(tr.all('span','xlabel'),None)
 label=' '.join(xlabel.text().split()) if xlabel else ''
 if label not in ['r ii 12','r ii 16','r ii 17','r ii 18']:continue
 columns=[' '.join(t.text().split()) for t in cells]
 lex.append(dict(line=label,html_id=tr.a.get('id',''),editorial_headword=columns[1],written_pronunciation=columns[2],sign_column=columns[3],akkadian_equivalent=columns[4],source='AO5400 / P333168; ORACC dcclt/signlists',status='Редакторская передача древнего списка; целевая строка на повреждённом обороте фотографии не прочитана независимо'))
out('round23_ancient_pronunciation_entries.csv',lex)
print('lexical rows extracted',len(lex))
