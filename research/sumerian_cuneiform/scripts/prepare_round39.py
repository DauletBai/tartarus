"""Recover the first edition and explicitly normalize image handedness."""
from pathlib import Path
import csv,json,hashlib,subprocess,html,tempfile
R=Path(__file__).resolve().parents[1];S=R/'round39_sources';S.mkdir(exist_ok=True)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(a):return subprocess.run(a,check=True,capture_output=True,text=True)
def write(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
pdf=S/'PSBA24_1902.pdf';assert pdf.read_bytes()[:5]==b'%PDF-'
run(['pdftotext','-layout',str(pdf),str(S/'PSBA24_1902.txt')])
pages=(S/'PSBA24_1902.txt').read_text().split('\f')
(S/'Pinches1902_excerpt.txt').write_text('\n\f\n'.join('PDF PAGE '+str(i+1)+'\n'+pages[i] for i in range(131,149)),encoding='utf-8')
with tempfile.TemporaryDirectory(prefix='tartarus-pinches-',dir='/tmp') as td:
 run(['pdfseparate','-f','132','-l','149',str(pdf),td+'/page-%d.pdf'])
 run(['pdfunite']+[td+'/page-'+str(i)+'.pdf' for i in range(132,150)]+[str(S/'Pinches1902_GreekTranscriptions.pdf')])
for n in (137,139,140):run(['pdftoppm','-f',str(n),'-l',str(n),'-singlefile','-scale-to','2000','-png',str(pdf),str(S/f'PSBA24_PDF{n}')])
orig=R/'round38_sources/BM34797_SJL0646.jpg';mir=S/'BM34797_SJL0646_mirror.png'
run(['sips','-s','format','png','-f','horizontal',str(orig),'--out',str(mir)])
transforms=[dict(file='round39_sources/'+mir.name,parent=str(orig.relative_to(R)),operation='horizontal reflection; output PNG; no glyph reconstruction',parent_sha256=sha(orig),sha256=sha(mir))]
specs=[('BM34797_Greek_209_211_context.png',str(mir.relative_to(R)),90,580,1000,650),
 ('Greek_prefix_209.png',str(mir.relative_to(R)),90,580,650,230),
 ('Greek_prefix_210.png',str(mir.relative_to(R)),90,790,650,240),
 ('Greek_prefix_211.png',str(mir.relative_to(R)),90,1010,650,230),
 ('Pinches1902_Greek_copy.png','round39_sources/PSBA24_PDF140.png',300,145,630,760),
 ('Pinches1902_cuneiform_copy.png','round39_sources/PSBA24_PDF139.png',90,925,820,760)]
crops=[]
for name,parent,x,y,w,h in specs:
 run(['sips','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(R/parent),'--out',str(S/name)])
 crops.append(dict(file='round39_sources/'+name,parent=parent,x=x,y=y,width=w,height=h,parent_sha256=sha(R/parent),sha256=sha(S/name)))
write('round39_image_transforms.csv',transforms);write('round39_crops.csv',crops)
align=[
 dict(canonical='209',greek_side_line='6',greek_prefix_visible='φα',photographic_result='phi loop/stem and neighbouring alpha located after explicit orientation normalization',remainder='λαλ# remains damaged; no complete independent reading claimed',cuneiform_side_line='8',cuneiform_model='PAP.E LAL#',decision='Greek prefix photographically supported; full cuneiform wedge collation still incomplete'),
 dict(canonical='210',greek_side_line='7',greek_prefix_visible='φα',photographic_result='second repeated phi/alpha group located on the same fragment',remainder='σεκ# includes damaged final letter',cuneiform_side_line='9',cuneiform_model='PAP.E# SIG',decision='Greek prefix supported; damaged cuneiform prefix remains a partial control'),
 dict(canonical='211',greek_side_line='8',greek_prefix_visible='φα',photographic_result='third repeated phi/alpha group located; following corrections do not affect prefix',remainder='σε!ι!θ is not treated as uncorrected whole-word evidence',cuneiform_side_line='10',cuneiform_model='PAP.E ŠITA3',decision='Greek prefix photographically supported; full sentence and ending not independently decoded')]
write('round39_greek_prefix_collation.csv',align)
history=[
 dict(item='PAP.E',Pinches1902='pa; text p113 explicitly discusses pha',current_model='pa5 ↔ φα',result='same general correspondence predates the current electronic edition; not a new decipherment by us'),
 dict(item='canonical209',Pinches1902='pa-lal with damaged Greek ending',current_model='pa5-lal# ↔ φα λαλ#',result='prefix is retained; do not fill broken ending merely from the old copy'),
 dict(item='canonical210',Pinches1902='pa-sig ↔ φα σεκ',current_model='pa5#-sig ↔ φα σεκ#',result='modern damage marks retained; old normalized table is not a second clay witness'),
 dict(item='canonical207',Pinches1902='Sumerian e; Greek Sumerian cell dotted/blank, neighbouring ειχ aligns with i-ki',current_model='e# ↔ [ε]; Akkadian i#-ki ↔ ειχ',result='exclude ειχ as direct phonetic proof for Sumerian E'),
 dict(item='pal / bal',Pinches1902='mu-un-pal',current_model='mu-un-bal',result='transcription convention/reading differs; do not infer a physical alteration of the wedges')]
write('round39_historical_comparison.csv',history)
page_map=[dict(pdf_page=132,printed_reference='108',content='article beginning'),dict(pdf_page=137,printed_reference='plate II',content='Greek-face photograph BM34797'),dict(pdf_page=139,printed_reference='111',content='cuneiform drawing'),dict(pdf_page=140,printed_reference='112',content='Greek drawing and parallel transcription'),dict(pdf_page=141,printed_reference='113',content='discussion of group pronounced pha'),dict(pdf_page=149,printed_reference='119',content='end of Pinches article')]
write('round39_page_map.csv',page_map)
sources={
 'PSBA24_commons_search.json':('https://commons.wikimedia.org/w/api.php?action=query&generator=search&gsrsearch=proceedings24soci&gsrnamespace=6&gsrlimit=5&prop=imageinfo&iiprop=url&format=json','archive_locator_JSON','Wikimedia API location of the source scan; not epigraphic evidence'),
 'PSBA24_1902.pdf':('https://upload.wikimedia.org/wikipedia/commons/d/d1/Proceedings_%28IA_proceedings24soci%29.pdf','historical_publication_scan','PSBA24 (1902); scan of historical publication; author T.G.Pinches article pp108–119 and plates. Digitization credited Internet Archive')}
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:reader=csv.DictReader(f);fields=reader.fieldnames;reg=list(reader)
reg=[r for r in reg if not r['local_file'].startswith('round39_sources/')]
for name,(url,typ,note) in sources.items():
 p=S/name
 reg.append(dict(local_file='round39_sources/'+name,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type=typ,bytes=p.stat().st_size,sha256=sha(p),rights_and_notes=note))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(reg)
report='''БЛОК 39. ПЕРВАЯ ПУБЛИКАЦИЯ И ОРИЕНТАЦИЯ ГРЕЧЕСКОЙ СТОРОНЫ
26 октября 2026

ПОЛУЧЕННЫЙ ПЕРВОИСТОЧНИК
Получен полный скан PSBA24 (1902),482 страницы,24 495 918 байт, из Wikimedia Commons. В нём прочитаны выбранные страницы статьи T.G.Pinches, Greek Transcriptions of Babylonian Tablets, печатные108–119, с таблицами и иллюстрациями BM34797. Извлечён отдельный18-страничный PDF статьи (PDF132–149); весь482-страничный том не объявляется целиком проанализированным. PlateII находится на PDF137; клинописная прорисовка — PDF139/печатная111; греческая прорисовка и параллельная таблица — PDF140/печатная112. Общий маршрут через Internet Archive снова был недоступен; обнаружен доступный скан на Commons. Недоступность первого адреса не означает отсутствия самого источника.

ГЛАВНЫЙ НОВЫЙ РЕЗУЛЬТАТ — ОРИЕНТАЦИЯ
Греческая сторона на сохранённом SJL0646 и в цветном фотомонтаже имеет противоположную горизонтальную ориентацию относительно PlateII и рисунка1902 года: боковой дополнительный фрагмент располагается с другой стороны, асимметричные контуры разломов и формы букв зеркально соответствуют. Для сверки создан явный горизонтально отражённый вариант SJL0646 в PNG. Исходный JPEG сохранён без изменений; преобразование и хеши записаны. На отражённом варианте можно соотнести расположение букв с рисунком и повторяющимися началами φα. Причину противоположной ориентации фотографий — этап съёмки, печати, работы с негативом или оцифровки — по этим файлам мы не установили. Из наблюдения не выводится историческое намерение кого-либо.

В предыдущих блоках эта ориентационная проверка не была выполнена. Поэтому прежние попытки точной локализации греческих групп оставались ненадёжными. Теперь исправлена подготовка изображения, а предыдущие отчёты сохранены как история работы. Клинописная сторона автоматически не отражается вместе с греческой: её сверяют отдельно. Устанавливать ориентацию каждой стороны необходимо по её собственным признакам.

ПРОВЕРЕНЫ ТРИ ГРЕЧЕСКИХ НАЧАЛА
На дополнительном боковом фрагменте локализованы три последовательные группы φ и α, соответствующие строкам209,210,211 канонического списка (греческие6,7,8 по редакции). Номера привязаны по рисунку и параллельной таблице, а сами формы букв сверены с фотографией после отмеченного преобразования. Сохранены общий контекст и три отдельных выреза. В двух контекстах 209/211 клинописное начало PAP.E в редакции не помечено повреждённым;210 остаётся частичным из-за pa5#. Эти состояния не переопределены по одному греческому снимку.

Это конкретный шаг от редакционной таблицы к изображению: повторяемое греческое начало видно на фотографии, а не только в латинской передаче. Однако полная независимая поклиновая колляция соответствующих двух клинописных строк ещё не завершена. Поэтому полное независимое чтение PAP.E и всего предложения не объявляется достигнутым. Два контекста одного предмета не считаются двумя независимыми древними предметами.

НЕ ПЕРЕНОСИТЬ ЧТЕНИЕ ИЗ СОСЕДНЕЙ КОЛОНКИ
В таблице1902 года шумерское e сопоставлено с аккадским i-ki. На греческой стороне шумерское место оставлено точками/утрачено, а видимое ειχ принадлежит аккадскому соответствию. В современной редакции это e# и [ε] напротив i#-ki и ειχ. Следовательно, видимое ειχ нельзя использовать как прямое подтверждение греческой epsilon для отдельного шумерского E. Смешение колонок создало бы ложный фонетический аргумент.

СРАВНЕНИЕ СТАРОЙ И СОВРЕМЕННОЙ РЕДАКЦИЙ
Pinches уже в1902 году обсуждал чтение группы как pha (печатная113). Наша работа не открывает это чтение впервые, а проверяет его носитель и состояние. Старые pa, mu-un-pal, sit не следует механически приравнивать современной системе индексов pa5, mu-un-bal, šita3 без сравнения графики и конвенций передачи. Различие транскрипций само по себе не доказывает изменения надписи. Повреждения и современные #/! сохраняются; старая нормализованная таблица не заменяет утраченные штрихи.

ЧТО ОСТАЛОСЬ НЕРЕШЁННЫМ
Из трёх греческих начал φα не следует однозначно, что PAP и E имеют отдельные звуки φ и α, или что PAP.E читается только как неделимый знак. Для различения нужны независимые употребления компонентов. Самостоятельный E по-прежнему не получил прямого сохранного алфавитного подтверждения. Вопрос точного звучания φ и перенос поздней школьной передачи на ранние эпохи здесь не решены. Нет нового полного перевода и нет окончательно нового независимо установленного фонетического значения. Но греческий участок теперь имеет проверенную привязку, которой прежде не было.

СОХРАННОСТЬ И ПРОИСХОЖДЕНИЕ
Преобразования: рендер страниц PDF, выделение страниц статьи, горизонтальное отражение одной греческой фотографии и прямоугольные вырезы. PNG выбран для производного изображения после декодирования JPEG; штрихи не дорисовывались. Исходники, координаты, операции и хеши сохранены. Извлечённая статья и рисунки являются представлениями того же предмета, а не новыми древними свидетелями. Книга и PDF Тартар и Я не изменялись.
'''
(R/'observations/2026-10-26_round39_orientation_and_first_edition.txt').write_text(report,encoding='utf-8')
def im(p,c):return f'<figure><a href="{p}"><img src="{p}" loading="lazy"></a><figcaption>{html.escape(c)}</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок39: ориентация и три греческих начала</title><style>body{max-width:1150px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f6f3ec;color:#202830}img{max-width:100%;max-height:900px;object-fit:contain}figure{padding:16px;background:white;margin:24px 0}figcaption{font-size:16px}.result{background:#e4efdf;padding:20px}.pair{display:grid;grid-template-columns:1fr 1fr;gap:16px}table{border-collapse:collapse;width:100%}td,th{border:1px solid #aaa;padding:10px;vertical-align:top;text-align:left}h1,h2{line-height:1.2}a{color:#194e8c}@media(max-width:700px){.pair{display:block}}</style><h1>Первая публикация помогла проверить ориентацию</h1><p>Блок39 · 26 октября2026. BM34797, греческая сторона.</p><div class="result"><b>Конкретный результат:</b> получены фотография и прорисовки1902 года. Современный сохранённый греческий снимок ориентирован зеркально относительно них. После явного горизонтального отражения локализованы три соседних начала φα. Причина различия ориентаций не установлена. Полная независимая колляция клинописных строк пока не закончена.</div><h2>1. Исходник и преобразованный вариант</h2><div class="pair">'''
page+=im('round38_sources/BM34797_SJL0646.jpg','Исходный SJL0646. Сохранён без изменений.')+im('round39_sources/BM34797_SJL0646_mirror.png','Горизонтальное отражение для сопоставления с публикацией1902 года. Это производное изображение, не новый источник.')+'</div><h2>2. Независимый контроль изображения публикацией1902 года</h2>'
page+=im('round39_sources/PSBA24_PDF137.png','PlateII, PDF137: греческая сторона с музейной подписью SpII290 + SpIII247 (34797).')+im('round39_sources/Pinches1902_Greek_copy.png','Прорисовка греческой стороны на печатной112 / PDF140. Авторский рисунок может интерпретировать штрихи, поэтому используется совместно с фотографией.')
page+='<h2>3. Три повторяющихся начала φα</h2>'+im('round39_sources/BM34797_Greek_209_211_context.png','Общий контекст трёх греческих строк на отражённом варианте. Сравнение показывает петлю/штрих φ и соседнюю α; окончания остаются отдельной проверкой.')
for n in (209,210,211):page+=im(f'round39_sources/Greek_prefix_{n}.png',f'Начало греческой строки, соотнесённой с контекстом{n}. Привязка по рисунку1902 года и параллельной таблице; вырез не означает полного прочтения строки.')
page+='<h2>4. Клинописная сторона: следующий уровень проверки</h2>'+im('round39_sources/Pinches1902_cuneiform_copy.png','Клинописная прорисовка, печатная111 / PDF139. Добавленные из параллельного текста строки отмечены в самом старом издании; их нельзя считать сохранившимися на BM34797.')+'<p>Греческие начала теперь локализованы. Самостоятельное установление границ всех клиньев в соответствующих двух клинописных строках ещё требуется. Соответствие PAP.E ↔ φα остаётся локально проверяемым кандидатом, без объявления полного нового перевода.</p><h2>5. Видимое ειχ не заполняет утраченную шумерскую колонку</h2><p>В таблице отдельно стоят шумерское e и аккадское i-ki. Видимое греческое ειχ относится к аккадской колонке. Для шумерского места современная редакция имеет восстановленное [ε]. Смешивать эти позиции нельзя.</p><h2>Первоисточник и протокол</h2><ul>'+''.join(f'<li><a href="{p}">{t}</a></li>' for p,t in [('round39_sources/Pinches1902_GreekTranscriptions.pdf','Статья Pinches1902, отдельный18-страничный PDF'),('round39_sources/PSBA24_1902.pdf','Полный скан PSBA24 (1902),482 страницы'),('observations/2026-10-26_round39_orientation_and_first_edition.txt','Полный отчёт'),('data/round39_greek_prefix_collation.csv','Привязка трёх греческих начал'),('data/round39_historical_comparison.csv','Сравнение старой и современной редакций'),('data/round39_image_transforms.csv','Операция отражения и хеши'),('data/round39_crops.csv','Координаты вырезов'),('data/round39_page_map.csv','Соответствие PDF-страниц печатным')])+'</ul><p>Новых полных переводов:0. Новых окончательно независимо установленных фонетических значений:0. Исходная книга не изменялась.</p></html>'
(R/'comparison_round39.html').write_text(page,encoding='utf-8')
summary=dict(new_historical_volume_pages=482,article_excerpt_PDF_pages=18,rendered_source_pages=3,new_external_files=2,explicit_horizontal_mirror_images=1,Greek_prefixes_located=3,cuneiform_full_row_collation_complete=False,independent_preserved_E_sound_anchor=False,new_fully_independent_sound_values=0,new_complete_sentence_translations=0)
(R/'data/round39_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n');print(json.dumps(summary,ensure_ascii=False))
