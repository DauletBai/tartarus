"""Reproduce crops and test a local repetition-marker model of BM35726."""
from pathlib import Path
import csv, hashlib, html, json, re, subprocess
from html.parser import HTMLParser
from urllib.parse import urlsplit, unquote
R=Path(__file__).resolve().parents[1]; S=R/'round41_sources';S.mkdir(exist_ok=True)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(a):return subprocess.run(a,check=True,capture_output=True,text=True)
def csvout(name,rows):
    with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)
spec=[]
for i,y in enumerate((150,350,550,750),1):spec.append((f'Greek_band_{i}', 'Greek',100,y,1500,280,'exploratory band, not exact line boundary'))
for i,y in enumerate((100,300,500,700),1):spec.append((f'Cuneiform_band_{i}','cuneiform',0,y,1600,330,'exploratory band, not exact line boundary'))
spec += [
 ('Greek_row3_end','Greek',1050,380,600,300,'editorial row3 ending context; no secure mu/rho decision'),
 ('Greek_row2_end','Greek',1050,170,600,300,'editorial row2 ending context'),
 ('Greek_ama_full','Greek',150,650,550,230,'refined candidate prefix; shallow/damaged strokes'),
 ('Greek_row5_prefix','Greek',220,800,1200,230,'bottom line context'),
 ('rho_reference_row3','Greek',1010,420,170,200,'candidate reference letter; not certified intact'),
 ('disputed_row3','Greek',1390,410,200,220,'disputed terminal region; shallow and incomplete'),
 ('mu_reference_row3','Greek',650,440,250,190,'candidate reference region; not certified intact'),
 ('mu_candidate_ama','Greek',330,650,220,190,'candidate mu region; localization uncertain'),
 ('AMA_cuneiform_candidate','cuneiform',300,390,500,220,'exploratory crop; superseded by row4_refined context'),
 ('Akkadian_rows_3_4','cuneiform',1500,300,1250,530,'broad context, extends into other rows'),
 ('Greek_Akkadian_rows_3_4','Greek',1700,370,1000,430,'paired-column context; both Greek entries examined'),
 ('AMA_cuneiform_context_refined','cuneiform',130,260,900,280,'exploratory crop; not a secure AMA boundary'),
 ('MIN_candidate_row4','cuneiform',1550,280,400,180,'initial search window misses target; superseded by MIN_row4_refined'),
 ('MIN_candidate_row6','cuneiform',1550,800,400,230,'initial search window misses target; superseded by MIN_row6_refined'),
 ('Akkadian_full_column','cuneiform',1500,0,1100,1100,'column context used to correct row localization'),
 ('MIN_row4_refined','cuneiform',1490,490,330,220,'two adjacent vertical impressions; row4 localization supported by full column and old copy'),
 ('MIN_row6_refined','cuneiform',1530,920,350,180,'bottom repetition-group context; one head and second impression less clear'),
 ('AMA_row4_refined','cuneiform',300,470,470,300,'row4 left context; complete AMA wedge inventory unresolved'),
]
crops=[]
for n,side,x,y,w,h,note in spec:
    parent='round40_sources/BM35726_'+('Greek' if side=='Greek' else 'cuneiform')+'_face.png';out=S/(n+'.png')
    run(['sips','-s','format','png','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(R/parent),'--out',str(out)])
    crops.append(dict(file=str(out.relative_to(R)),parent=parent,x=x,y=y,width=w,height=h,operation='rectangular crop; no reconstruction or reflection',interpretive_status=note,parent_sha256=sha(R/parent),sha256=sha(out)))
csvout('round41_crops.csv',crops)
pdf=R/'round39_sources/PSBA24_1902.pdf';high=S/'PSBA24_PDF134_high.png';rot=S/'PlateI_rotated90.png'
run(['pdftoppm','-f','134','-l','134','-singlefile','-scale-to','4000','-png',str(pdf),str(high.with_suffix(''))])
run(['sips','-s','format','png','-r','90',str(high),'--out',str(rot)])
csvout('round41_transforms.csv',[
 dict(file=str(high.relative_to(R)),parent=str(pdf.relative_to(R)),operation='PDF page134 raster render; scale-to4000; no new source detail',parent_sha256=sha(pdf),sha256=sha(high)),
 dict(file=str(rot.relative_to(R)),parent=str(high.relative_to(R)),operation='clockwise rotation90 degrees; no reflection; no reconstruction',parent_sha256=sha(high),sha256=sha(rot))])

# The repeated form is a comparison of the published strings, not a new sound assignment.
contexts=[
 dict(line='obv3 / rev3',cuneiform_model='ša₂-qin-nu',greek_diplomatic='σακι<ν>',comparison_form='σακιν',physical_status='Greek final nu supplied by edition; not counted as visible',role='antecedent for repetition hypothesis'),
 dict(line='obv4 / rev4',cuneiform_model='MIN',greek_diplomatic='σακιν',comparison_form='σακιν',physical_status='paired vertical impressions located on cuneiform photo; full Greek letter collation not certified',role='local test of repeat-previous-entry model'),
 dict(line='obv5 / rev5',cuneiform_model='la-ab-bu',greek_diplomatic='λ*#[αβ]',comparison_form='λαβ',physical_status='only partial Greek remains; restored ending excluded as observed',role='antecedent for prediction only'),
 dict(line='obv6 / no preserved rev6 in saved edition',cuneiform_model='MIN',greek_diplomatic='not available',comparison_form='',physical_status='bottom pair context partially visible; Greek control unavailable',role='prediction of repetition cannot be tested here'),
]
csvout('round41_repetition_contexts.csv',contexts)
literal='μιν';expanded=contexts[0]['comparison_form'];observed_model=contexts[1]['comparison_form']
tests=[
 dict(model='L_fixed_MIN_sound',predicted=literal,editorial_comparison=observed_model,exact_string_match=literal==observed_model,scope='reject only fixed pronunciation min for this position under the alignment; not every alphabetic account'),
 dict(model='R_repeat_previous_entry',predicted=expanded,editorial_comparison=observed_model,exact_string_match=expanded==observed_model,scope='fits one aligned context; antecedent nu editorial; function remains a supported local hypothesis'),
 dict(model='R_row6_prediction',predicted=contexts[2]['comparison_form'],editorial_comparison='missing',exact_string_match='not_testable',scope='prediction only; not a successful held-out test'),
]
csvout('round41_model_tests.csv',tests)
csvout('round41_access_checks.csv',[
 dict(url='https://cdli.earth/artifacts/566456',result='HTML retrieved; actual image area says No image available; generic og:image is not confirmation of a photo'),
 dict(url='https://cdli.earth/dl/photo/P566456.jpg',result='HTTP404; no image obtained; not a missing ancient object'),
 dict(url='https://www.degruyterbrill.com/document/doi/10.1515/ZA.2007.014/pdf',result='curl exited0 but empty response; no full article obtained or analysed')])
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:rd=csv.DictReader(f);fields=rd.fieldnames;reg=list(rd)
reg=[r for r in reg if not r['local_file'].startswith('round41_sources/')]
p=S/'CDLI_P566456.html'
assert p.exists() and 'BM 035726' in p.read_text() and 'No image available' in p.read_text()
reg.append(dict(local_file='round41_sources/CDLI_P566456.html',source_url='https://cdli.earth/artifacts/566456',retrieved_date_Asia_Qostanay='2026-10-26',type='catalogue_HTML',bytes=p.stat().st_size,sha256=sha(p),rights_and_notes='Catalogue identifies BM035726; image area empty. Not a new photograph or ancient witness.'))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(reg)

report='''БЛОК 41. BM35726: ПРОВЕРКА БУКВ И ЗНАКА ПОВТОРА
26 октября 2026

ОСНОВНОЙ РЕЗУЛЬТАТ
Получена более точная привязка двух клинописных групп, передаваемых редакцией как MIN: в правой колонке строк4 и6. В строке4 видны два соседних вертикальных вдавления. Нижняя группа частично менее ясна. В параллельной греческой строке4 редакция даёт σακιν, а не отдельное μιν. Сопоставление с предыдущей строкой поддерживает функцию «повторить предыдущее соответствие». Это ограниченный структурный результат: не каждый графический элемент в таком тексте обязан непосредственно записывать произносимую букву или слог. Значение всех клиньев и новый полный перевод не получены.

КАК ПРОВЕРЯЛАСЬ ГИПОТЕЗА
В строке3 клинописная модель имеет ša₂-qin-nu, греческая — σακι<ν>. Угловые скобки означают редакционное добавление: последняя ν не выдаётся за сохранившуюся букву. В строке4 клинописная модель заменяет полное слово на MIN, а греческая содержит σακιν. Модель фиксированного произношения MIN как μιν не совпадает с этой строкой. Модель повторения предшествующего соответствия даёт σακιν и совпадает с опубликованной греческой строкой.

Программа проверяет точное совпадение строк этих двух моделей. Это не автоматическая дешифровка и не новая независимая транскрипция: входные строки взяты из уже известной редакции. Фотография служит отдельной проверкой наличия короткой группы вертикальных вдавлений; полная побуквенная колляция греческих соответствий не завершена. Разные условные модели письма могут допускать сокращения и классификаторы. Поэтому один этот пример не опровергает все мыслимые алфавитные объяснения и не доказывает устройство всей клинописи.

В строке6 снова стоит MIN после la-ab-bu в строке5. Модель повторения предсказывает повтор того же соответствия. Однако в сохранённой электронной редакции нет греческой строки6. В строке5 греческое λ*#[αβ] тоже частично восстановлено. Поэтому это лишь будущая проверка, не успешно пройденный контроль на новом материале. Два употребления на одной табличке не считаются двумя независимыми предметами.

ПОКЛИНОВОЕ НАБЛЮДЕНИЕ
Уточнённый вырез MIN_row4_refined имеет координаты x1490,y490,w330,h220 относительно BM35726_cuneiform_face.png. Видны две соседние вертикальные борозды с расширением сверху; контуры отличаются из-за износа и освещения. Уточнённый нижний контекст: x1530,y920,w350,h180; часть краёв менее ясна. Никакие утраченные клинья не достраивались. Число звуков из числа этих вдавлений не выводится.

КОРРЕКЦИЯ ЛОКАЛИЗАЦИИ
Для поиска сделаны перекрывающиеся полосы обеих сторон, затем проверена вся правая колонка. Первые поисковые окна MIN_candidate_row4 и MIN_candidate_row6 попадали выше целевых групп. Они сохранены как история поиска и явно помечены ошибочными поисковыми окнами, а не доказательными вырезами. После осмотра полной колонки выделены MIN_row4_refined и MIN_row6_refined. Похожим образом широкие области AMA не считаются точными границами знака.

AMA / ΑΜΑ
Вырез Greek_ama_full уточняет начало строки относительно широкого окна прошлого блока, но неглубокие штрихи и утраты остаются. Уточнён левый клинописный контекст строки4, однако полная уверенная опись всех клиньев GA2×AN здесь не получена. Следовательно, пара остаётся кандидатом с частичной фотоподдержкой. Сходство начертания или звучания с современным казакским словом не подставляется вместо недостающего прочтения.

DOM / DOR
Выделены спорное окончание, ближайший контекст и кандидаты сравнительных μ/ρ того же писца. Уверенно прочитанные эталонные формы для независимого решения на этих вырезах не установлены. Поэтому нельзя создавать формальный классификатор с заведомо спорными обучающими метками. Расхождение dom в1902 году и dor в электронной редакции остаётся открытым; этот блок его не разрешил.

ПРОВЕРКА ИСТОРИЧЕСКОГО СНИМКА
Из уже сохранённого тома1902 года страница PDF134 (PlateI) заново выведена при scale-to4000 и повёрнута на90 градусов по часовой стрелке. Это фотография, а не прорисовка, и на ней видна та же греческая сторона. Поворот не включает зеркального отражения. Крупный рендер облегчает просмотр, но не восстанавливает детали, потерянные при печати старой фотографии. Его зернистость не позволяет здесь окончательно решить μ/ρ. Это другой фотографический снимок одного предмета, не новый древний свидетель.

ПОИСК ДОПОЛНИТЕЛЬНОГО ИСТОЧНИКА
Сохранена карточка CDLI P566456, обозначающая BM035726. В HTML есть общий og:image со ссылкой P566456.jpg, но в видимой области изображения написано No image available. Прямой запрос фотографии вернул404. Поэтому одна строка метаданных не засчитана как полученная фотография. Проверен адрес PDF Westenholz2007 на сайте издателя: ответ оказался пустым. Полный текст этой статьи не получен и не объявляется прочитанным. В библиографии eBL есть ссылка на его колляцию страницы265; проверка этой колляции остаётся задачей.

ЧТО ЭТО МЕНЯЕТ В МЕТОДЕ
Рабочая единица должна определяться по функции в строке: звуковой знак, запись слова, классификатор, условное сокращение или повтор. Для данного места функция повторения лучше объясняет соответствие, чем буквальное произношение min. Строить заранее единую таблицу «каждый клин = неизменная буква» по этому примеру не обосновано. Прежде требуется отделить графический след от составного знака и от служебной функции.

ГРАНИЦА ИТОГА
Поддержана локальная структурная гипотеза о повторении; получены уточнённые вырезы и воспроизводимое сравнение моделей. Новых полностью независимо установленных звуковых значений:0. Полных новых переводов:0. Генетическое родство с казакским языком и происхождение письма этим блоком не установлены и не опровергнуты. Книга и её PDF не менялись.
'''
reportpath='observations/2026-10-26_round41_repetition_marker_and_letter_controls.txt'
(R/reportpath).write_text(report,encoding='utf-8')
def im(n,c):
    p='round41_sources/'+n+'.png'
    return '<figure><a href="'+p+'"><img src="'+p+'" loading="lazy"></a><figcaption>'+html.escape(c)+'</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок41: знак повтора</title><style>body{max-width:1150px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f7f3eb;color:#202830}figure{background:white;margin:24px 0;padding:16px}img{max-width:100%;max-height:900px;object-fit:contain}figcaption{font-size:16px}table{border-collapse:collapse}td,th{border:1px solid #aaa;padding:12px;text-align:left}.scroll{overflow:auto}.result{padding:20px;background:#e4eee2}h1,h2{line-height:1.2}</style><h1>BM35726: два вертикальных вдавления и функция повторения</h1><p>Блок41 · 26 октября2026</p><div class="result">Главный результат — локальная проверка служебной функции: короткая клинописная группа MIN в строке4 сопоставлена с повторяющимся греческим словом. Это поддерживает модель «повтори предыдущее соответствие», не устанавливая звучание каждого клина. DOM/DOR и полный состав AMA остаются открытыми.</div><h2>Сначала контекст, затем вырез</h2>'''
page+=im('Akkadian_full_column','Правая клинописная колонка. Общий контекст позволил исправить первые поисковые окна.')
page+=im('MIN_row4_refined','Строка4: два соседних вертикальных вдавления. Фото поддерживает наличие короткой группы, называемой в редакции MIN.')
page+=im('MIN_row6_refined','Нижний контекст: второе употребление по модели редакции; часть контуров менее ясна. Греческой строки6 для прямой проверки здесь нет.')
page+=im('Greek_Akkadian_rows_3_4','Греческий контекст соседних соответствий. Последняя ν предыдущей строки добавлена редакцией и не считается сохранной.')
page+='<h2>Сравнение моделей на одном контексте</h2><div class="scroll"><table><tr><th>Модель</th><th>Предсказание</th><th>Строка4 в редакции</th><th>Совпадение</th></tr>'
for t in tests:page+='<tr>'+''.join('<td>'+html.escape(str(t[k]))+'</td>' for k in ['model','predicted','editorial_comparison','exact_string_match'])+'</tr>'
page+='</table></div><p>Это проверка редакционных строк с отдельной фотоколляцией графической группы, а не слепая дешифровка. Для строки6 греческий контроль отсутствует.</p><h2>Спорная буква: решение остаётся открытым</h2>'
page+=im('Greek_row3_end','Контекст старого dom / современного dor. Надёжного решения по этим штрихам пока нет.')
page+=im('disputed_row3','Область спорного окончания. Название файла — рабочая привязка, не установленное чтение.')
page+='<h2>AMA: уточнённые области поиска</h2>'+im('Greek_ama_full','Начало греческой строки, сопоставляемое с αμα. Повреждения сохраняются.')+im('AMA_row4_refined','Левый клинописный контекст строки4. Полная опись клиньев AMA не завершена.')
page+='<h2>Фотография1902 года после поворота</h2>'+im('PlateI_rotated90','PlateI, PDF134, рендер scale-to4000, поворот90° по часовой стрелке. Поворот не восстанавливает потерянные полиграфические детали.')
page+='<h2>Отчёт и проверяемые данные</h2><ul>'
for p,t in [(reportpath,'Полный отчёт'),('data/round41_repetition_contexts.csv','Четыре контекста и состояние сохранности'),('data/round41_model_tests.csv','Предсказания моделей'),('data/round41_crops.csv','Все поисковые и уточнённые вырезы, координаты и статусы'),('data/round41_transforms.csv','Рендер и поворот'),('data/round41_access_checks.csv','Доступность дополнительных источников'),('round41_sources/CDLI_P566456.html','Сохранённая карточка CDLI'),('comparison_round40.html','Предыдущий блок')]:page+='<li><a href="'+p+'">'+html.escape(t)+'</a></li>'
page+='</ul><p>Книга не изменялась. Файлы с названиями candidate и reference — поисковые области, а не гарантированно прочитанные знаки. Источник современной модели: round38_sources/eBL_BM35726.json.</p></html>'
(R/'comparison_round41.html').write_text(page,encoding='utf-8')

# Validation concerns files/provenance and explicit predictions, not historical truth.
for r in crops:
    p=R/r['file'];assert p.read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
    assert sha(p)==r['sha256'] and sha(R/r['parent'])==r['parent_sha256']
    info=run(['sips','-g','pixelWidth','-g','pixelHeight',str(p)]).stdout
    assert int(re.search(r'pixelWidth: (\d+)',info)[1])==r['width']
    assert int(re.search(r'pixelHeight: (\d+)',info)[1])==r['height']
assert literal!=observed_model and expanded==observed_model
assert '<ν>' in contexts[0]['greek_diplomatic'] and '[' in contexts[2]['greek_diplomatic']
class Links(HTMLParser):
    def __init__(self):super().__init__();self.urls=[]
    def handle_starttag(self,t,a):
        for k,v in a:
            if k in ('href','src') and v and not urlsplit(v).scheme:self.urls.append(unquote(v))
l=Links();l.feed(page)
for p in l.urls:assert (R/p).is_file(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
summary=dict(status='passed',crop_records=len(crops),local_links=len(l.urls),external_catalogue_files=1,new_external_photographs=0,local_repetition_model_matches=1,held_out_Greek_test_available=False,dom_dor_resolved=False,AMA_wedge_inventory_complete=False,new_independent_sound_values=0,book_PDF='unchanged')
(R/'data/round41_checkpoint.txt').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
(R/'data/round41_next_targets.txt').write_text('1. Obtain actual Westenholz2007 p265 image or author collation; no inference from unavailable PDF.\n2. Test repetition-marker interpretation on a second physical tablet with an intact Greek counterpart.\n3. Seek stronger local mu/rho controls before classifying the disputed ending.\n4. Complete GA2×AN wedge tracing only with adequate image detail.\n',encoding='utf-8')
print(json.dumps(summary,ensure_ascii=False,indent=2))
