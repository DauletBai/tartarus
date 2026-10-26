"""Test a repeated composite against alphabetic witnesses, without assigning sounds to wedges."""
from pathlib import Path
import csv,json,hashlib,re,subprocess,html,xml.etree.ElementTree as E
R=Path(__file__).resolve().parents[1];S=R/'round38_sources'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
base=json.loads((R/'round37_sources/eBL_BM34797.json').read_text())
def records(atf):
 side=None;out=[];last=None
 for line in atf.splitlines():
  if line.startswith('@'):side=line[1:]
  m=re.match(r"(\d+)'?\. (.*)",line)
  if m:
   last=dict(side=side,line=m[1],text=m[2],canonical='');out.append(last)
  if line.startswith('// (Ḫarra II ') and last:last['canonical']=re.search(r'(\d+)\)',line)[1]
 return out
rr=records(base['atf']);a={x['canonical']:x for x in rr if x['side']=='obverse'};b={x['canonical']:x for x in rr if x['side']=='reverse'}
norm=[s for s in base['signs'].splitlines() if s.strip()]
ob=[r for r in rr if r['side']=='obverse'];assert len(norm)==len(ob)==10
norm_by={r['canonical']:s for r,s in zip(ob,norm)}
cases=[]
for canon in ['208','209','210','211','212']:
 c=a[canon]['text'].split(' & ')[0].replace('%sux ','');g=b[canon]['text'].split(' & ')[0].replace('%suxgrc ','')
 status={'208':'exclude: Greek prefix restored','209':'retain prefix: both prefixes unmarked in edition; ending damaged','210':'partial: cuneiform prefix damaged','211':'retain prefix: following corrections do not affect φα','212':'exclude: cuneiform prefix restored'}[canon]
 cases.append(dict(canonical=canon,obverse_line=a[canon]['line'],reverse_line=b[canon]['line'],cuneiform_editorial=c,greek_editorial=g,normalized_sign_model=norm_by[canon],prefix_components='PAP.E / ABZ60 ABZ308',decision=status,scope='edition-guided local prefix check; not a blind photographic decipherment'))
 assert norm_by[canon].split()[:2]==['ABZ60','ABZ308']
write('round38_prefix_alignment.csv',cases)
pap=json.loads((S/'eBL_PAP.json').read_text());ee=json.loads((S/'eBL_E.json').read_text())
assert any(x['name']=='ABZ' and x['number']=='60' for x in pap['lists'])
assert any(x['name']=='ABZ' and x['number']=='308' for x in ee['lists'])
control_log=json.loads((S/'control_fetch_log.json').read_text());controls=[]
for item in control_log:
 if not item['valid']:
  controls.append(dict(museum=item['museum'],ATF_available=False,ABZ308_in_normalized_model=False,E_context='',decision='endpoint unavailable; not evidence of absence'));continue
 d=json.loads((S/item['file']).read_text());atf=d.get('atf','');present='ABZ308' in d.get('signs','').split()
 contexts=[line for line in atf.splitlines() if 'e-lu' in line or '%akkgrc [σουθ ικαβασ' in line]
 decision='No ATF in this snapshot; cannot infer absence of sign on object' if not atf else 'No ABZ308 in supplied normalized sequence; bounded electronic screen only'
 if present:decision='Candidate rejected as preserved E control: e-lu-u2 and corresponding Greek epsilon are restored'
 controls.append(dict(museum=item['museum'],ATF_available=bool(atf),ABZ308_in_normalized_model=present,E_context=' | '.join(contexts),decision=decision))
write('round38_E_control_screen.csv',controls)
models=[
 dict(model='A: PAP ↔ φ, E ↔ α separately',fits_selected_prefix_pairs='yes',unresolved='need preserved independent E and PAP contexts in same alphabetic-control tradition',verdict='not excluded by two selected prefixes alone'),
 dict(model='B: composite PAP.E ↔ φα as a unit',fits_selected_prefix_pairs='yes',unresolved='need independent component uses and comparable syllabic/logographic alternations',verdict='consistent with sign-list model; not uniquely proved by this pair'),
 dict(model='C: assign a letter to every individual wedge',fits_selected_prefix_pairs='not operationally specified',unresolved='no stable mapping or segmentation supplied; component identity is not individual wedge identity',verdict='not yet a testable decipherment; cannot claim refuted solely by PAP.E')]
write('round38_competing_segmentations.csv',models)
sources={
 'BM34797_JS_II679c.jpg':('https://www.ebl.lmu.de/api/folios/JS/II%20679c','folio_notebook','Multiple Sp.II entries; only Sp.II.290 concerns BM34797 here; not a second ancient witness'),
 'BM34797_SJL0646.jpg':('https://www.ebl.lmu.de/api/folios/SJL/0646','archival_photograph','Greek face of BM34797'),
 'BM34797_SJL0647.jpg':('https://www.ebl.lmu.de/api/folios/SJL/0647','archival_photograph','Cuneiform face of BM34797'),
 'BM34797_SJL1141.jpg':('https://www.ebl.lmu.de/api/folios/SJL/1141','multi_object_archival_photograph','Left BM90893; right BM34797 cuneiform face. Do not confuse the two objects'),
 'BM34797_SJL1142.jpg':('https://www.ebl.lmu.de/api/folios/SJL/1142','multi_object_archival_photograph','Left BM90893; right BM34797 Greek face. Do not count this as new tablet evidence'),
 'OSL_P.html':('https://oracc.museum.upenn.edu/osl/signlist/l0080/index.html','modern_sign_reference','Oracc sign data CC0; separate page-template notices'),
 'OSL_PAP_E.html':('https://oracc.museum.upenn.edu/osl/signlist/l0080/o0002221/index.html','modern_sign_reference','Composite PAP.E, values pa5/pad5; names are model labels, not new sound discoveries'),
 'eBL_PAP.json':('https://www.ebl.lmu.de/api/signs/PAP','modern_sign_reference','PAP = ABZ60'),
 'eBL_E.json':('https://www.ebl.lmu.de/api/signs/E','modern_sign_reference','E = ABZ308')}
for item in control_log:
 if item['valid']:sources[item['file']]=(item['url'],'catalogue_and_editorial_snapshot','Normalized sign sequence can include restored signs; distinguish from observed clay')
assert len(sources)==16
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:reader=csv.DictReader(f);fields=reader.fieldnames;reg=list(reader)
reg=[r for r in reg if not r['local_file'].startswith('round38_sources/')]
inventory=[]
for name,(url,typ,note) in sources.items():
 p=S/name
 if name.endswith('.jpg'):assert p.read_bytes()[:2]==b'\xff\xd8'
 elif name.endswith('.json'):json.loads(p.read_text())
 else:E.parse(p)
 inventory.append(dict(file='round38_sources/'+name,kind=typ,notes=note,sha256=sha(p)))
 reg.append(dict(local_file='round38_sources/'+name,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type=typ,bytes=p.stat().st_size,sha256=sha(p),rights_and_notes=note+'; image reproduction rights for printed book not established; research archive only' if name.endswith('.jpg') else note))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(reg)
write('round38_source_inventory.csv',inventory)
specs=[('BM34797_cuneiform_lower_left.jpg','BM34797_SJL0647.jpg',300,1400,1000,1270),('BM34797_Greek_lower_left.jpg','BM34797_SJL0646.jpg',270,1450,1300,1150),('BM34797_1141_right_only.jpg','BM34797_SJL1141.jpg',3250,700,2250,2750),('BM34797_1142_right_only.jpg','BM34797_SJL1142.jpg',3250,900,2350,2950)]
crops=[]
for name,parent,x,y,w,h in specs:
 subprocess.run(['sips','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(S/parent),'--out',str(S/name)],check=True,capture_output=True)
 crops.append(dict(file='round38_sources/'+name,parent='round38_sources/'+parent,x=x,y=y,width=w,height=h,sha256=sha(S/name),parent_sha256=sha(S/parent)))
write('round38_crops.csv',crops)
report='''БЛОК 38. ПОВТОРЯЮЩИЙСЯ КОМПЛЕКС PAP.E И ГРЕЧЕСКОЕ φα
26 октября 2026

КОНКРЕТНЫЙ РЕЗУЛЬТАТ
Уточнён состав графической группы, передаваемой в редакции как pa5: PAP.E, а не один простой знак. В нормализованной последовательности eBL это ABZ60 ABZ308. Справочники eBL для PAP и E подтверждают соответствие номеров, OSL показывает состав |PAP.E| и значения pa5/pad5. Это проверка графической идентификации внутри модели; справочники не считаются двумя независимыми древними свидетелями.

ПРОВЕРКА ПОВТОРОВ
Построена таблица пяти выровненных контекстов 208–212 по редакционным номерам канонического списка. Номер физической строки на одной стороне не равен номеру на другой: например, редакционные 8/6 соответствуют канонической строке 209; 9/7 — строке 210. Эта привязка пока опирается на редакционные комментарии, а не на независимо завершённую разметку всех знаков на фотографии.
208: pa5 ↔ [φα] — греческая группа восстановлена; исключено как полное прямое подтверждение.
209: pa5-lal# ↔ φα λαλ# — начальные группы оставлены для проверки; повреждение относится к окончанию.
210: pa5#-sig ↔ φα σεκ# — первый клинописный комплекс повреждён; только частичный контроль.
211: pa5-šita3 ↔ φα σε!ι!θ — поправки в следующей группе не затрагивают само начальное φα. Для проверки начала это допустимый кандидат, хотя всё окончание нельзя считать бесспорным.
212: [pa5]-šita3 ↔ φα σ#ειθ — клинописное начало восстановлено; исключено как целиком сохранившийся начальный комплекс.
Итого в редакционно выровненной выборке: два основных кандидата начального соответствия PAP.E ↔ φα, один частичный, два исключённых. Это не две независимо расшифрованные строки и не статистическая оценка достоверности всего языка. На уровне позднего алфавитного контроля получена частичная опора для чтения всей группы; точный фонетический вариант φ и реконструкция более раннего произношения остаются отдельными вопросами.

ДОПОЛНИТЕЛЬНЫЕ ФОТОГРАФИИ
Получены все пять открытых листов, перечисленных в карточке BM34797: JS II 679c, SJL0646, SJL0647, SJL1141, SJL1142. SJL0646 и 0647 — архивные фотографии двух сторон; SJL1141 и 1142 содержат по два предмета. Слева на последних находится BM90893, справа BM34797; подпись и форма предметов сверены. Для просмотра вырезаны только правые предметы. JS II679c — страница с несколькими музейными записями Sp.II.; не вся страница относится к BM34797. Эти пять файлов не являются пятью независимыми древними свидетельствами. Они улучшают видимость отдельных штрихов, но полная побуквенная и поклиновая колляция строк 209–210 ещё не завершена.

ПРОВЕРКА САМОСТОЯТЕЛЬНОГО E
Чтобы различить чтение компонентов по отдельности и чтение составной группы, просмотрены восемь дополнительных музейных записей eBL. Семь доступны, HSM1137 по проверенному адресу возвращает 404. В четырёх доступных карточках есть ATF, в трёх ATF отсутствует: это не означает отсутствия текста на древнем предмете.
Один нормализованный список знаков, BM33769, содержит ABZ308. Но соответствующий участок ATF имеет восстановленное e-lu-u2 внутри [...], а нужная греческая epsilon также находится в восстановлении. Поэтому кандидат НЕ засчитан как сохранившийся независимый звук E. Получен важный контроль источника данных: нормализованная последовательность знаков может включать знаки, восстановленные редактором, а не наблюдаемые на глине. Положительный результат электронного поиска сам по себе не даёт материального доказательства.

ДВА КОНКУРИРУЮЩИХ ОБЪЯСНЕНИЯ
A. PAP соответствует φ, E соответствует α в выбранных сочетаниях.
B. PAP.E целиком является составным знаком с передачей φα.
Два выделенных начала совместимы с обоими объяснениями. Поэтому одна эта пара НЕ доказывает невозможность буквенного анализа компонентов и НЕ доказывает его правильность. Современная модель составного знака согласуется с данными, но независимый выбор требует сохранных употреблений компонентов вне PAP.E и других алфавитных параллелей. Назначать каждому отдельному клину букву тоже нельзя без определённой таблицы соответствий и проверок на других строках: такая таблица здесь ещё не получена.

ГРАНИЦА ФОНЕТИЧЕСКОГО ВЫВОДА
Имеется локальный кандидат позднего звучания группы PAP.E, связанный с реально существующей греческой передачей. Он сильнее простого созвучия современному слову, потому что опирается на две системы письма одного предмета. Однако независимое полное чтение этих двух строк по фотографиям ещё не закончено; речь не идёт об открытии нового неизвестного науке значения. Мы проверяем ранее опубликованное соответствие и его материальные ограничения. Звук простого E и язык всей надписи по нему не определены. Новых полностью независимо закреплённых значений: 0; полных новых переводов: 0. Вопрос происхождения и связи с казакским языком здесь не решается.

ДОСТУП И ПРОИСХОЖДЕНИЕ
Адреса открытых листов взяты из карточки eBL и маршрута публичного клиента; запросы выполнены без учётных данных. Попытка получить метаданные скана PSBA24 на archive.org завершилась тайм-аутом; PDF Geller1997 с проверенного адреса издателя пришёл пустым. Эти попытки не объявлены полученными источниками и не включены в реестр как валидные файлы. Сами статьи не считаются проанализированными по отсутствующим полным текстам. Книга и PDF не изменялись. Все вырезы — кадрирование без дорисовки.
'''
(R/'observations/2026-10-26_round38_composite_prefix_controls.txt').write_text(report,encoding='utf-8')
def im(p,c):return f'<figure><a href="{p}"><img src="{p}" loading="lazy"></a><figcaption>{html.escape(c)}</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок 38: PAP.E и φα — проверка состава</title><style>body{max-width:1150px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f6f3ec;color:#202830}img{max-width:100%;max-height:850px;object-fit:contain}figure{background:white;padding:15px;margin:24px 0}figcaption{font-size:16px}table{width:100%;border-collapse:collapse}td,th{border:1px solid #aaa;padding:10px;text-align:left;vertical-align:top}.result{padding:20px;background:#e4efdf}h1,h2{line-height:1.2}a{color:#194e8c}</style><h1>Два компонента PAP.E и греческая группа φα</h1><p>Блок 38 · 26 октября 2026. Проверка повторов и восстановленных знаков.</p><div class="result"><b>Уточнение:</b> редакционное pa₅ записано составной группой PAP.E. В пяти выровненных контекстах выделены два основных кандидата соответствия этой группы греческому φα, один частичный и два исключённых. Это пока локальная проверка модели: полная независимая колляция обеих строк по фотографии не завершена.</div><h2>1. Контроль по отдельным начальным группам</h2>'''
page+='<table><tr><th>Контекст</th><th>Клинописная модель</th><th>Греческая модель</th><th>Решение</th></tr>'+''.join(f'<tr><td>{x["canonical"]}<br>строки {x["obverse_line"]}/{x["reverse_line"]}</td><td>{html.escape(x["cuneiform_editorial"])}</td><td>{html.escape(x["greek_editorial"])}</td><td>{html.escape(x["decision"])}</td></tr>' for x in cases)+'</table><p>Номера и чтения взяты из редакции для проверки. Неповреждённое начало не исключается только потому, что соседнее окончание имеет поправки; восстановленное начало не засчитывается как наблюдаемое.</p><h2>2. Дополнительные фотографии того же предмета</h2>'
page+=im('round38_sources/BM34797_SJL0646.jpg','SJL0646: греческая сторона. Другой снимок BM34797, не другой древний экземпляр.')+im('round38_sources/BM34797_SJL0647.jpg','SJL0647: клинописная сторона. Сохраняются утраты и материал соединения.')+im('round38_sources/BM34797_cuneiform_lower_left.jpg','Фрагмент клинописной стороны для проверки повторяющихся комплексов. Не подписан как окончательно прочитанная строка.')+im('round38_sources/BM34797_Greek_lower_left.jpg','Фрагмент греческой стороны. Отдельные штрихи видны лучше, но чтение всей строки независимо не объявляется завершённым.')
page+='<h2>3. Не смешивать предметы на групповых снимках</h2>'+im('round38_sources/BM34797_1141_right_only.jpg','SJL1141: из группового снимка вырезан только правый предмет BM34797. Оставшийся слева на оригинале BM90893 не является частью этого текста.')+im('round38_sources/BM34797_1142_right_only.jpg','SJL1142: только BM34797 с греческой стороной.')
page+='<p><a href="round38_sources/BM34797_SJL1141.jpg">Групповой оригинал SJL1141</a> · <a href="round38_sources/BM34797_SJL1142.jpg">Групповой оригинал SJL1142</a> · <a href="round38_sources/BM34797_JS_II679c.jpg">Лист JS II679c с несколькими музейными записями</a></p><h2>4. Самостоятельное E остаётся нерешённым</h2><p>Электронный поиск нашёл ABZ308 в BM33769. Проверка ATF показала: нужный E и греческая epsilon относятся к восстановленному участку. Кандидат исключён как сохранное фонетическое подтверждение. Нормализованный список знаков включает восстановления.</p><h2>5. Чего пока нельзя заключить</h2><p>Соответствие PAP.E ↔ φα совместимо и с чтением всей группы, и с гипотезой буквенного распределения PAP↔φ, E↔α в этих контекстах. Два начала сами по себе не выбирают между объяснениями. Для выбора нужны сохранные независимые употребления компонентов. Поздняя греческая передача не устанавливает автоматически произношение ранних эпох.</p><h2>Материалы</h2><ul>'+''.join(f'<li><a href="{p}">{t}</a></li>' for p,t in [('observations/2026-10-26_round38_composite_prefix_controls.txt','Полный отчёт'),('data/round38_prefix_alignment.csv','Пять выровненных контекстов'),('data/round38_E_control_screen.csv','Восемь проверенных адресов внешнего контроля'),('data/round38_competing_segmentations.csv','Конкурирующие варианты сегментации'),('data/round38_source_inventory.csv','Реестр полученных источников'),('data/round38_crops.csv','Координаты и хеши вырезов'),('round38_sources/OSL_PAP_E.html','Сохранённая статья OSL о PAP.E')])+'</ul><p>Новых полностью независимо установленных значений: 0. Новых полных переводов: 0. Книга не изменялась.</p></html>'
(R/'comparison_round38.html').write_text(page,encoding='utf-8')
summary=dict(external_source_files=16,new_folio_images=5,main_ancient_objects=1,additional_object_on_group_photos='BM90893 excluded from target',aligned_prefix_contexts=5,retained_prefix_candidates=2,partial_prefix_controls=1,excluded_prefix_cases=2,additional_control_endpoints=8,valid_control_snapshots=7,control_snapshots_with_ATF=4,preserved_independent_E_controls_found=0,new_fully_independent_sound_values=0,new_sentence_translations=0)
(R/'data/round38_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,ensure_ascii=False))
