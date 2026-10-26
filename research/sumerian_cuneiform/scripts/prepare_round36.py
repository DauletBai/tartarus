"""Image-grounded school/cone comparison; readings remain conditional."""
from pathlib import Path
import csv,json,hashlib,subprocess,xml.etree.ElementTree as E,html
from crop_exact import crop_exact
R=Path(__file__).resolve().parents[1];S=R/'round36_sources';S.mkdir(exist_ok=True)
specs=[('CBS7074_reverse.jpg','round32_sources/P229436_photo.jpg',820,4600,2200,2650),
 ('CBS7074_copy_12rows.jpg','round32_sources/P229436_lineart.jpg',0,890,280,500),
 ('CBS7074_copy_target.jpg','round32_sources/P229436_lineart.jpg',0,985,280,450),
 ('CBS7074_middle_rows.jpg','round32_sources/P229436_photo.jpg',850,5450,1300,1050),
 ('CBS7074_transition_rows.jpg','round32_sources/P229436_photo.jpg',850,6000,1100,600),
 ('CBS7074_copy_transition.jpg','round32_sources/P229436_lineart.jpg',5,1270,260,165),
 ('CBS7074_row14_photo.jpg','round32_sources/P229436_photo.jpg',870,6300,780,180),
 ('CBS7074_row14_copy.jpg','round32_sources/P229436_lineart.jpg',5,1335,250,70),
 ('CBS7074_UD_upper_context.jpg','round32_sources/P229436_photo.jpg',850,5160,830,470),
 ('CBS7074_UD_middle_context.jpg','round32_sources/P229436_photo.jpg',850,5570,830,470),
 ('CBS7074_neighboring_pair_photo.jpg','round32_sources/P229436_photo.jpg',850,6000,830,220)]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def outcsv(path,rows):
 with (R/path).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
crop_rows=[]
for name,parent,x,y,w,h in specs:
 if x==0 or y==0:crop_exact(R/parent,S/name,x,y,w,h)
 else:subprocess.run(['sips','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(R/parent),'--out',str(S/name)],capture_output=True,check=True)
 crop_rows.append(dict(file='round36_sources/'+name,parent=parent,x=x,y=y,width=w,height=h,sha256=sha(S/name),parent_sha256=sha(R/parent)))
outcsv('data/round36_crops.csv',crop_rows)
NS={'h':'http://www.w3.org/1999/xhtml'};root=E.parse(R/'round32_sources/P229436_edition.html')
entries=[]
for tr in root.findall('.//h:tr',NS):
 if tr.get('id') not in {f'P229436.{n}' for n in range(58,70)}:continue
 cells=[' '.join(''.join(td.itertext()).split()) for td in tr.findall('h:td',NS)]
 lab=tr.find('.//h:span[@class="xlabel "]',NS)
 entries.append(dict(row_id=tr.get('id'),line=' '.join(''.join(lab.itertext()).split()),pronunciation_model=cells[2],main_sign_model=cells[3],status='editorial reading; not a new assignment of sound'))
assert len(entries)==12
outcsv('data/round36_school_rows.csv',entries)
assessments=[
 dict(group='r iii 4–5',objects='CBS 7074',observed='изношенные верхние записи; восстановление всех соседних вдавлений ненадёжно',result='Не засчитывать u3 и ta-am3 как независимо прочитанные звуки.'),
 dict(group='r iii 6–12',objects='CBS 7074',observed='повторяющийся основной комплекс и меняющиеся соседние группы видимы; границы отдельных слоговых знаков не везде проверены',result='Структура списка поддержана. Семь полных новых фонетических чтений не заявляются.'),
 dict(group='r iii 14',objects='CBS 7074',observed='простой решётчатый комплекс напротив сочетания треугольного и примыкающего прямолинейного комплекса; соответствует копии',result='Графическое соответствие E ↔ UD.DU поддержано сопоставлением фото и копии. Функция произносительной подсказки остаётся интерпретацией списка.'),
 dict(group='заключение строки 11',objects='PTS 1501; PTS 1502',observed='сходное двухкомпонентное окончание уже выделено как R2–R3',result='Сопоставление с UD.DU учебной таблички поддержано на уровне состава и расположения. Точное звучание по одному контуру не выводится.')]
outcsv('data/round36_observation_decisions.csv',assessments)
chain=[
 dict(step=1,evidence='CBS 7074: простой комплекс F напротив составного Q.D',dependency='фото + прорисовка; сама парность видима',status='графическое наблюдение'),
 dict(step=2,evidence='F соотносится с редакционным E; Q.D — с UD.DU',dependency='палеографическое сопоставление с проверяемой моделью',status='поддержанная идентификация формы'),
 dict(step=3,evidence='если строка содержит подсказку произношения, F и Q.D имеют связанное чтение',dependency='учебная функция колонок; не исключены семантические соответствия только по одной строке',status='интерпретация, требующая серии строк'),
 dict(step=4,evidence='если E независимо имеет звук /e/, для UD.DU получаем кандидат /e/',dependency='внешняя проверка звука простого E ещё не проведена нами',status='условное фонетическое чтение'),
 dict(step=5,evidence='сходный Q.D имеется в окончании конусов',dependency='межэпоховое сопоставление форм, контекст и границы комплекса',status='кандидат переносится для проверки; значение предложения не восстановлено')]
outcsv('data/round36_reading_chain.csv',chain)
report='''БЛОК 36. ОТ УЧЕБНОЙ ТАБЛИЧКИ К НАДПИСИ НА КОНУСЕ
26 октября 2026

РЕЗУЛЬТАТ
На CBS 7074 / P229436 выделена древняя парная запись, соответствующая редакционному E | UD.DU (оборот, колонка iii, строка 14). Сопоставлены фотография и старая прорисовка. Слева наблюдается простой решётчатый комплекс, справа — составной комплекс. Такой же составной тип сопоставляется с окончанием строки 11 на изученных конусах PTS 1501 и PTS 1502: прежние области R2 и R3 принадлежат этой составной записи. Таким образом, к графическому наблюдению на конусах добавлено древнее учебное соответствие. Это один физический учебный предмет, а не два независимых свидетеля, полученных из фото и копии.

УСЛОВНОЕ ЧТЕНИЕ
Запись E | UD.DU позволяет сформулировать проверяемую цепочку: если простому E независимо установлено произношение /e/ и если левая часть данного школьного списка представляет произносительную подсказку, составному UD.DU также соответствует чтение /e/. Следовательно, редакционное e3 перестаёт быть для нас лишь словом из современного перевода: найден конкретный древний материальный контроль. Однако звук E не выведен нами заново из двуязычного имени или другого независимого фонетического материала. Поэтому здесь нет новой независимой расшифровки /e/ с нуля. Индекс 3 в e3 — современный номер различения одинаково читаемых знаков, не древняя цифра в слове.

ЧТО ПОДТВЕРЖДЕНО ПО ИЗОБРАЖЕНИЯМ
1. Парность записи имеется в древнем размещении знаков, а не только в латинской транслитерации.
2. Составной основной комплекс можно отличить от соседнего простого основного комплекса UD по добавленному правому компоненту.
3. Сходное двухкомпонентное окончание встречается на другом типе предметов — конусах. Это даёт проверку сегментации и кандидата чтения между учебной и связной надписью.
4. Один видимый клин не равен автоматически одной букве. Здесь полезная единица сопоставления включает несколько вдавлений, а составная запись включает два компонента.

ЧТО НЕ УСТАНОВЛЕНО
Фотография не доказывает сама по себе ни звук /e/, ни значение «выходить», ни полный перевод заключительной формулы. Парная запись без иных ограничений могла бы передавать и смысловое соответствие; её фонетическая интерпретация проверяется по серии записей. Древняя учебная интерпретация не совпадает автоматически с современной грамматической классификацией. Из неё не следует ни казакское происхождение, ни отсутствие такого родства.

ПРОВЕРКА РЯДА UD
В редакции девять последовательных строк iii 4–12 содержат UD, а соседние группы переданы как u3; ta?-am3?; a; ah; pi-ir; hu-ud; za-la-ag; ba-ab-ba-ar; u2-tu. Все девять строк просмотрены в контексте фотографии и копии; изношенные верхние записи iii 4–5 оставлены нерешёнными. В остальных прослеживается повторяемость основного комплекса и изменение соседних групп, однако полная поклиновая проверка всех слоговых знаков не выполнена. Поэтому девять строк редакции не засчитаны как девять независимо восстановленных произношений. Для ряда u2-tu | UD звуковое чтение остаётся редакционной гипотезой: отдельный вырез соседних пар в атласе не подписан как окончательно прочитанная строка 12, поскольку точная привязка короткого выреза и границы всех знаков недостаточны.

ПОЧЕМУ НУЖЕН ИМЕННО ТАКОЙ РЕЗУЛЬТАТ
Исследование теперь связывает конкретную древнюю пару с конкретным составным окончанием надписи. На следующем этапе проверяется оставшаяся зависимость: как независимо закрепить звук простого E, а затем проверить этим чтением другие строки, не выбирая слово по современному созвучию. Одновременно потребуется второй физический учебный свидетель такой же пары. До этих проверок чтение условно, а не окончательно.

МЕТОД И ИСТОЧНИКИ
Исходные фотографии: round32_sources/P229436_photo.jpg (CDLI, CBS 7074), round27_sources/PTS1501_photo.jpg и PTS1502_photo.jpg. Старая копия: round32_sources/P229436_lineart.jpg, PBS 12, 56. Сохранённая редакция: round32_sources/P229436_edition.html, DCCLT; сведения о восстановленных и сомнительных знаках сохранены. Все источники уже находились в архиве; новых внешних источников в этом блоке нет. Сравнение не слепое: опубликованные названия знаков были известны. Фрагменты получены только кадрированием, без дорисовки. Шрифтовая таблица Noto Sans Cuneiform — современный образец, не прорисовка древнего предмета. Условное фонетическое соответствие: 1; новых независимо установленных звуков: 0; новых переводов предложений: 0. Книга не изменялась.
'''
(R/'observations/2026-10-26_round36_school_to_cone_bridge.txt').write_text(report,encoding='utf-8')
def im(file,caption):return f'<figure><a href="{file}"><img src="{file}" loading="lazy"></a><figcaption>{html.escape(caption)}</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок 36: древняя пара E — UD.DU</title><style>body{max-width:1100px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f6f3ec;color:#202830}img{max-width:100%;max-height:850px;object-fit:contain}figure{margin:25px 0;padding:16px;background:white}figcaption{font-size:16px}h1,h2{line-height:1.2}.result{padding:20px;background:#e4efdf}table{border-collapse:collapse}td,th{border:1px solid #aaa;padding:12px;text-align:left;vertical-align:top}a{color:#194e8c}</style><h1>Древний учебный контроль составного знака</h1><p>Блок 36 · 26 октября 2026 · CBS 7074 и конусы PTS 1501 / 1502.</p><div class="result"><b>Материальная связь:</b> простому комплексу E в древнем списке сопоставлен составной UD.DU. Сходный составной тип находится в окончании надписи на конусах. Получен конкретный контроль сегментации и условного чтения, но звук простого E независимо ещё не восстановлен.</div><h2>1. Древняя парная запись</h2>'''
page+=im('round36_sources/CBS7074_row14_photo.jpg','CBS 7074: исследуемая пара в контексте фотографии. Простой решётчатый комплекс слева, составной справа. Номер строки 14 относится к редакционной разметке всей колонки.')
page+=im('round36_sources/CBS7074_row14_copy.jpg','Старая прорисовка той же пары с краями соседних строк; это дополнительное представление того же предмета, не второй древний оригинал.')
page+=im('round36_sources/CBS7074_transition_rows.jpg','Более широкий контекст соседних пар помогает проверять привязку и отличать простой основной знак от составного.')
page+='<h2>2. Проверка на конусах</h2>'+im('round34_sources/PTS1501_lower_context.jpg','PTS 1501: в нижней части фрагмента после группы мелких косых вдавлений и треугольной формы следует двухкомпонентное окончание. Редакционная идентификация: IN, затем UD.DU (= e3).')+im('round34_sources/PTS1502_bottom_rows.jpg','PTS 1502: соответствующее двухкомпонентное окончание видно в средней полосе. Области изображения не следует считать отдельными буквами.')
page+='<h2>3. Что требуется для звука</h2><table><tr><th>Шаг</th><th>Основание</th><th>Статус</th></tr>'+''.join(f'<tr><td>{r["step"]}</td><td>{html.escape(r["evidence"])}<br><small>{html.escape(r["dependency"])}</small></td><td>{html.escape(r["status"])}</td></tr>' for r in chain)+'</table>'
page+='<h2>4. Весь проверяемый участок</h2>'+im('round36_sources/CBS7074_copy_target.jpg','Копия повторяющегося ряда и перехода к составным комплексам; чтения в латинице помещены отдельно в таблицу модели.')+im('round36_sources/CBS7074_UD_upper_context.jpg','Фотография верхней/средней части исследуемого ряда. Изношенные места не восстановлены дорисовкой.')+im('round36_sources/CBS7074_UD_middle_context.jpg','Фотография средней части ряда: повторяется основной комплекс, меняются соседние группы. Этот короткий вырез не является отдельным полным прочтением всех строк.')
page+='<h2>5. Шрифтовая справка</h2>'+im('round36_sources/modern_glyph_reference.png','Современные шрифтовые E, UD, DU, U2, TU, SHU. Это образцы для сравнения модели, не древняя палеография и не независимое доказательство звучания.')
page+='<h2>Отчёт и данные</h2><ul><li><a href="observations/2026-10-26_round36_school_to_cone_bridge.txt">Полный отчёт</a></li><li><a href="data/round36_reading_chain.csv">Зависимости условного чтения</a></li><li><a href="data/round36_school_rows.csv">12 редакционных строк, включая соседние составные записи</a></li><li><a href="data/round36_observation_decisions.csv">Наблюдения и решения</a></li><li><a href="data/round36_crops.csv">Координаты и хеши</a></li><li><a href="round32_sources/P229436_photo.jpg">Полная фотография CBS 7074</a></li><li><a href="round32_sources/P229436_lineart.jpg">Полная старая копия</a></li><li><a href="round32_sources/P229436_edition.html">Сохранённая редакция DCCLT</a></li></ul><p>Новых независимо установленных звуков: 0. Новых переводов предложений: 0. Книга не изменялась.</p></html>'
(R/'comparison_round36.html').write_text(page,encoding='utf-8')
summary=dict(school_physical_objects=1,cone_physical_objects=2,editorial_rows_examined=12,repeated_UD_editorial_rows=9,conditional_phonetic_correspondences=1,new_independently_established_sound_values=0,new_sentence_translations=0,crops=len(crop_rows),new_external_sources=0)
(R/'data/round36_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,ensure_ascii=False))
