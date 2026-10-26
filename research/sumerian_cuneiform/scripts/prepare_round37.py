"""Second-witness search: index leads, material preservation and model kept apart."""
from pathlib import Path
import csv,json,hashlib,re,subprocess,html,xml.etree.ElementTree as E
R=Path(__file__).resolve().parents[1];S=R/'round37_sources';NS={'h':'http://www.w3.org/1999/xhtml'}
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def clean(e):return ' '.join(''.join(e.itertext()).split())
def outcsv(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
leads=[]
for a in E.parse(S/'OSL_UD_DU_lexical.html').findall('.//h:a',NS):
 u=a.get('href','');m=re.search(r'/(P\d+)\.(\d+)\.(\d+)',u)
 if m:leads.append(dict(pid=m[1],editorial_row=m[2],url=u,label=clean(a),status='index lead; not verified physical preservation'))
assert len(leads)==11 and len({r['pid'] for r in leads})==9
outcsv('round37_index_leads.csv',leads)
selection={'P228706':[60,61,62,63,64], 'P227815':[9,10,11], 'P386409':[10,11,12,13]}
entries=[]
for pid,ids in selection.items():
 root=E.parse(S/(pid+'_edition.html'))
 for tr in root.findall('.//h:tr',NS):
  if tr.get('id') not in {pid+'.'+str(i) for i in ids}:continue
  cc=[clean(td) for td in tr.findall('h:td',NS)]
  lab=tr.find('.//h:span[@class="xlabel "]',NS)
  entries.append(dict(pid=pid,row_id=tr.get('id'),line=clean(lab),pronunciation_model=cc[2],main_sign_model=cc[3],all_cells=' | '.join(cc),status='editorial transcription; square brackets and uncertainty preserved'))
outcsv('round37_editorial_rows.csv',entries)
specs=[('UM2916107_reverse.jpg','P228706_photo.jpg',990,4900,1300,2900),
 ('UM2916107_lower_right.jpg','P228706_photo.jpg',1300,6320,900,1400),
 ('UM2916107_pair_context.jpg','P228706_photo.jpg',1300,6760,900,430),
 ('BM34950_copy_target.jpg','P386409_lineart.jpg',20,180,205,160),
 ('BM34950_photo_target.jpg','BM34950_photo.jpg',1350,900,1000,650),
 ('BM34950_three_entries.jpg','BM34950_photo.jpg',1350,930,1000,410),
 ('BM34950_three_entries_copy.jpg','P386409_lineart.jpg',20,220,205,100)]
crops=[]
for name,parent,x,y,w,h in specs:
 subprocess.run(['sips','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(S/parent),'--out',str(S/name)],capture_output=True,check=True)
 crops.append(dict(file='round37_sources/'+name,parent='round37_sources/'+parent,x=x,y=y,width=w,height=h,sha256=sha(S/name),parent_sha256=sha(S/parent)))
outcsv('round37_crops.csv',crops)
decisions=[
 dict(pid='P229436',museum='CBS 7074',editorial_pair='e | UD.DU',photograph='имеется в прежнем архиве; сопоставлялась в блоке 36',preservation='редакция без квадратных скобок в паре',decision='исходный контроль; не новая находка этого блока'),
 dict(pid='P228706',museum='UM 29-16-107',editorial_pair='e | UD.DU; r ii 13',photograph='получена и просмотрена; контекст пары выделен',preservation='пара в редакции без скобок; фото изношено и рассечено трещинами',decision='второй предмет с опубликованной полной парой; поклиновое чтение пары по фото не завершено'),
 dict(pid='P227815',museum='CBS 15310 + CBS 15368',editorial_pair='⸢e⸣ | [UD.DU]; r i 1',photograph='получена и просмотрена; два соединённых фрагмента',preservation='основной комплекс полностью восстановлен редактором',decision='не считать сохранившейся полной парой; два фрагмента не два независимых экземпляра'),
 dict(pid='P386409',museum='BM 34950',editorial_pair='e | UD.DU; a-ra | UD.DU; sag-us2 | UD.DU',photograph='новая фотография получена через публичный eBL API; сверена с CT 12, 30',preservation='основные соответствия в транслитерации без скобок; английские подписи содержат UD.[...] и не должны заменять осмотр фото',decision='добавлен фотографический контроль блока разных подсказок при основном комплексе; полное тождество каждого клина трёх записей не заявляется')]
outcsv('round37_witness_decisions.csv',decisions)
models=[
 dict(model='один знак — одно неизменное чтение',prediction='одинаковому основному комплексу соответствует одна стабильная звуковая запись',data='издания BM 34950 дают e, a-ra, sag-us2 при UD.DU; фотография показывает меняющиеся соседние группы',verdict='в таком общем виде модель не поддержана; не исключает единственного чтения в отдельно взятом контексте'),
 dict(model='древний список чтений многозначного знака',prediction='один комплекс сопровождается разными короткими группами; сходные соответствия повторяются на других учебных предметах',data='поддержано структурой и несколькими опубликованными записями; часть свидетелей повреждена или восстановлена',verdict='рабочая модель; точные фонемы из одной формы знака не выведены'),
 dict(model='древний смысловой список синонимов',prediction='одному комплексу могут соответствовать разные слова; одной парности недостаточно для фонетического равенства',data='одна фотографическая пара сама по себе не исключает такую возможность',verdict='для различения нужны сохранные многоязычные колонки и проверенные слоговые знаки; не принимать автоматически, но сохранять как контроль')]
outcsv('round37_competing_models.csv',models)
urls={
 'OSL_U.html':'https://oracc.museum.upenn.edu/osl/signlist/l0085/index.html',
 'OSL_UD_DU.html':'https://oracc.museum.upenn.edu/osl/signlist/l0085/o0002488/index.html',
 'OSL_UD_DU_lexical.html':'https://oracc.museum.upenn.edu/osl/signlist/selpages/o0002488-lex.html',
 'eBL_BM34950.json':'https://www.ebl.lmu.de/api/fragments/BM.34950',
 'BM34950_photo.jpg':'https://www.ebl.lmu.de/api/fragments/BM.34950/photo'}
log=json.loads((S/'fetch_log.json').read_text())
for item in log:
 if item['valid']:urls[item['file']]=item['url']
assert len(urls)==14
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:reader=csv.DictReader(f);fields=reader.fieldnames;reg=list(reader)
reg=[x for x in reg if not x['local_file'].startswith('round37_sources/')]
for name,url in urls.items():
 p=S/name;raw=p.read_bytes()
 if name.endswith('.jpg'):assert raw[:2]==b'\xff\xd8'
 elif name.endswith('.json'):json.loads(raw)
 elif name.startswith('OSL_') or name.endswith('_edition.html'):E.parse(p)
 rights=('British Museum photograph supplied via public eBL endpoint; reproduction licence not established; not added to book' if name=='BM34950_photo.jpg' else 'Oracc sign data CC0; separate page-template terms' if name.startswith('OSL_') else 'Research snapshot; photograph/handcopy publication rights not established; editorial restorations retained')
 reg.append(dict(local_file='round37_sources/'+name,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type='photograph' if 'photo.jpg' in name else 'modern_handcopy' if 'lineart' in name else 'editorial_reference_or_catalogue',bytes=p.stat().st_size,sha256=sha(p),rights_and_notes=rights))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(reg)
client=Path('/tmp/tartarus-ebl-client.js')
provenance=dict(date='2026-10-26',client_url='https://www.ebl.lmu.de/static/js/main.cffb60e4.js',client_sha256=sha(client) if client.exists() else None,observed_route='/fragments/${encodeURIComponent(e)}/photo',public_photo_url=urls['BM34950_photo.jpg'],authentication='no account tokens supplied; public GET; no website access control bypass',failed_alternative_requests=[dict(url='https://www.penn.museum/collections/object.php?irn=566924',http_status=403),dict(url='https://www.britishmuseum.org/collection/object/W_Sp-II-470',http_status=403)],notes='eBL metadata reports hasPhoto=true; no electronic transliteration in eBL snapshot. Used DCCLT edition separately.')
(S/'photo_acquisition_provenance.json').write_text(json.dumps(provenance,ensure_ascii=False,indent=2)+'\n')
report='''БЛОК 37. НЕЗАВИСИМЫЕ ПРЕДМЕТЫ И ГРАНИЦЫ ЧТЕНИЯ UD.DU
26 октября 2026

ГЛАВНОЕ
Получена ранее отсутствовавшая у нас фотография BM 34950, через открытый интерфейс университетского проекта eBL. CDLI по обычному адресу фотографии возвращал 404, но имел прорисовку CT 12, 30 и ссылку на eBL. В метаданных eBL указано hasPhoto=true; открытый клиент использует маршрут fragments/{number}/photo. По нему получен валидный JPEG 4359×6359, 4 433 689 байт. Это новый фотографический источник для нашего архива, а не открытие ранее неизвестной таблички. На фотографии просмотрен блок, который в редакции содержит три соответствия e | UD.DU, a-ra | UD.DU и sag-us2 | UD.DU. Сопоставлены общая форма фрагмента, маркеры строк, соседние группы и прорисовка. Разные соседние группы видимы; поклиновое тождество трёх основных комплексов не объявляется полностью проверенным.

ВТОРОЙ ПОЛНЫЙ РЕДАКЦИОННЫЙ СВИДЕТЕЛЬ
UM 29-16-107 / P228706: оборот ii 13, e | UD.DU. Получены фотография и редакция MSL 14, 023 Fc. Пара напечатана без квадратных скобок. На полном снимке локализован нижний правый участок оборота, сохранены контекстные фрагменты. Поверхность изношена и покрыта трещинами: полностью независимо прочитанной пары в этом блоке не заявлено. Это отдельный физический предмет, а не ещё одно изображение CBS 7074. Каталог относит UM 29-16-107 к старовавилонскому периоду и Ниппуру; мы не выполняли независимого датирования.

ЛОЖНОЕ УСИЛЕНИЕ, КОТОРОЕ ИСКЛЮЧЕНО
CBS 15310 + CBS 15368 / P227815: оборот i 1, ⸢e⸣ | [UD.DU]. В указателе он находится рядом с остальными примерами, но основной комплекс здесь целиком восстановлен. Поэтому его нельзя засчитать как сохранившуюся полную пару. Фотография двух соединённых фрагментов получена; два музейных номера не превращены в два независимых древних свидетельства. Важно: это не основание обвинять редакторов в подделке. Квадратные скобки как раз открыто показывают восстановление.

ПОЛНЫЙ ПРОСМОТР УКАЗАТЕЛЯ
В OSL для |UD.DU| найдено 11 предметных ссылок на 9 различных P-идентификаторов. Это поисковые кандидаты, а не 9 коллированных сохранившихся пар. Сам указатель также содержит ссылки на составные научные тексты без отдельного P-номера. Помимо e он перечисляет a-ra, sag-us2 и pe2-eš-pe2-eš при соответствующих значениях знака. При этом прямую строку BM 34950 с peš-peš нельзя механически считать четвёртым UD.DU: её транслитерация имеет UD!-[UD]. Все буквальные записи сохранены в таблице.

ЧТО ПОДДЕРЖИВАЮТ ИЗОБРАЖЕНИЯ
Материальная структура учебного списка существует: рядом с основным комплексом помещаются меняющиеся короткие группы. В BM 34950 это теперь можно проверять по фотографии, а не только доверять рисунку. Сходное соответствие e | UD.DU опубликовано на нескольких физических предметах, а не является одним современным переводческим решением CBS 7074. Сохранность разная; восстановленные записи не смешаны с видимыми. Это усиливает кандидат чтения /e/ для конкретного контекста, но одновременно запрещает назначать его всем появлениям UD.DU без проверки.

ПРОВЕРКА КОНКУРИРУЮЩИХ ОБЪЯСНЕНИЙ
Модель «один основной знак всегда имеет одно неизменное чтение» плохо согласуется с рядом разных подсказок. Рабочая модель древнего списка чтений объясняет повторяемость основного комплекса и изменение соседних слоговых групп. Однако одна парная запись без независимых языковых ограничений могла бы быть и смысловым соответствием. Поэтому фонетическую функцию нельзя доказать одним лишь расположением E напротив UD.DU. Нужны дополнительные школьные параллели с сохранной переводной колонкой и независимо проверенные слоговые значения. Это проверка между объяснениями, а не выбор привычного перевода по авторитету.

НЕРЕШЁННАЯ ФОНЕТИЧЕСКАЯ ЗАВИСИМОСТЬ
Два списка с E напротив UD.DU ещё не определяют физический звук /e/: их условные названия можно одновременно заменить, не изменив рисунка. Чтобы независимо закрепить произношение, нужен внешний языковой контроль, например повторяющиеся имена или двуязычные написания, связывающие слоговые знаки с другой известной письменностью. В этом блоке такой контроль для простого E не получен. Следовательно: кандидат /e/ лучше снабжён материальными свидетелями; новых независимо установленных звуков по-прежнему 0. Полные предложения заново не переведены. Агглютинация и родство с казакским языком по этому списку не определялись.

УТОЧНЕНИЕ БЛОКА 36
Его вывод о древней паре сохраняется как графическая и условная фонетическая проверка. Он не означает уже выполненную независимую расшифровку звука простого E. Фотографии новых предметов расширили проверку сохранности и позволили выделить восстановленный пример, который нельзя использовать как дополнительное доказательство целого написания.

ИСТОЧНИКИ, ДОСТУП И ПРАВА
OSL |UD.DU| и его лексический указатель; CDLI P228706, P227815, P386409; индивидуальные DCCLT-издания; открытый JSON и фотография eBL BM.34950. Отдельные запросы к музейным страницам Penn Museum и British Museum получили HTTP 403; это не признак секретности документов. Публичный eBL предоставил фотографию без передачи учётных данных. История получения сохранена. Права воспроизведения новой музейной фотографии для печатной книги не установлены; в книгу она не добавлялась. Все фрагменты — прямоугольные вырезы, без дорисовки и изменения клиньев. Шифры, скобки и сомнения редакторов сохранены. Исследование не слепое: научные чтения нам уже известны.

Итог: 3 новых предмета просмотрены по полученным материалам, 3 новых фотографии (включая eBL), 1 новая прорисовка, 14 новых внешних файлов. 11 поисковых ссылок не объявлены 11 новыми находками. Таблицы и снимки доступны через comparison_round37.html. Книга и PDF не изменялись.
'''
(R/'observations/2026-10-26_round37_independent_witnesses.txt').write_text(report,encoding='utf-8')
def im(p,c):return f'<figure><a href="{p}"><img src="{p}" loading="lazy"></a><figcaption>{html.escape(c)}</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок 37: вторые свидетели и фотография BM 34950</title><style>body{max-width:1100px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f6f3ec;color:#202830}img{max-width:100%;max-height:900px;object-fit:contain}figure{background:white;padding:16px;margin:25px 0}figcaption{font-size:16px}table{border-collapse:collapse;width:100%}td,th{border:1px solid #aaa;padding:12px;vertical-align:top;text-align:left}.result{background:#e4efdf;padding:20px}h1,h2{line-height:1.2}a{color:#194e8c}</style><h1>Вторые свидетели чтения UD.DU</h1><p>Блок 37 · 26 октября 2026. Новые фотографии и проверка того, что действительно сохранилось.</p><div class="result"><b>Получен новый фотографический контроль:</b> BM 34950. Издание сопоставляет основному комплексу разные подсказки e, a-ra, sag-us₂. Второй полный редакционный пример E ↔ UD.DU найден на UM 29-16-107; его поклиновая проверка ещё не закончена. На CBS 15310 + CBS 15368 основной знак восстановлен — эта запись не засчитана как сохранившаяся полная пара.</div><h2>1. BM 34950: фотография, которую не отдавал обычный адрес CDLI</h2>'''
page+=im('round37_sources/BM34950_three_entries.jpg','Фотография British Museum, получена через публичный eBL API. Целевой блок: меняющиеся соседние группы и основной комплекс. Сверка с рисунком поддерживает привязку; все отдельные клинья трёх комплексов полностью не коллированы.')+im('round37_sources/BM34950_three_entries_copy.jpg','CT 12, 30: соответствующий участок старой прорисовки с краями соседних строк. Фото и рисунок относятся к одному предмету.')
page+='<p><a href="round37_sources/BM34950_photo.jpg">Полный фотомонтаж BM 34950</a> · <a href="round37_sources/P386409_lineart.jpg">Полная прорисовка</a> · <a href="round37_sources/P386409_edition.html">Редакция с сохранёнными скобками и комментариями</a> · <a href="round37_sources/photo_acquisition_provenance.json">История получения фотографии</a></p>'
page+='<h2>2. UM 29-16-107: другой физический предмет</h2>'+im('round37_sources/UM2916107_pair_context.jpg','Контекст правой колонки оборота. Изношенные и рассечённые трещинами формы не дорисованы. Пара e | UD.DU стоит в редакции в r ii 13; полная независимая колляция клиньев пока не заявлена.')+'<p><a href="round37_sources/UM2916107_reverse.jpg">Весь оборот</a> · <a href="round37_sources/P228706_photo.jpg">Полный фотомонтаж</a> · <a href="round37_sources/P228706_edition.html">Редакция</a></p>'
page+='<h2>3. Свидетель, который нельзя засчитать как полную пару</h2><p>CBS 15310 + CBS 15368: редакционная запись <b>⸢e⸣ | [UD.DU]</b>. Квадратные скобки означают восстановление основного комплекса. Его нельзя предъявлять как целиком сохранившуюся надпись. Два соединённых фрагмента — один экземпляр.</p>'+im('round37_sources/P227815_photo.jpg','Полученная фотография соединённых фрагментов. Состояние утраченной части не заменено восстановлением на изображении.')
page+='<h2>4. Решения по предметам</h2><table><tr><th>Предмет</th><th>Сохранность и проверка</th><th>Решение</th></tr>'+''.join(f'<tr><td>{html.escape(x["museum"])}</td><td>{html.escape(x["preservation"])}</td><td>{html.escape(x["decision"])}</td></tr>' for x in decisions)+'</table><h2>5. Что это даёт для чтения</h2><p>Фотографии и разные предметы усиливают проверку конкретного соответствия. Они не делают /e/ единственным звуком UD.DU. Звучание простого E в этом блоке независимо не закреплено: для этого ещё нужен внешний фонетический контроль. Новых независимо установленных звуков: <b>0</b>; новых переводов предложений: <b>0</b>.</p>'
page+='<h2>Документы проверки</h2><ul>'+''.join(f'<li><a href="{p}">{t}</a></li>' for p,t in [('observations/2026-10-26_round37_independent_witnesses.txt','Полный отчёт'),('data/round37_editorial_rows.csv','12 выбранных редакционных строк'),('data/round37_witness_decisions.csv','Решения по сохранности'),('data/round37_index_leads.csv','11 ссылок указателя на 9 кандидатов'),('data/round37_competing_models.csv','Конкурирующие объяснения структуры списка'),('data/round37_crops.csv','Координаты и хеши фрагментов')])+'</ul><p>Фотографии не добавлялись в книгу. Их лицензии для печатного воспроизведения не установлены. Книга не изменялась.</p></html>'
(R/'comparison_round37.html').write_text(page,encoding='utf-8')
summary=dict(index_links=11,index_candidate_pids=9,new_artifacts_reviewed=3,new_photographs=3,new_handcopies=1,new_external_files=14,editorial_rows=len(entries),new_complete_editorial_pair_objects=2,restored_main_sign_objects_excluded=1,photographic_microcollation_complete=False,new_independently_established_sound_values=0,new_sentence_translations=0)
(R/'data/round37_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,ensure_ascii=False))
