"""Photographic collation of Lu-Utu variants; no new phonetic assignment."""
from pathlib import Path
import csv,hashlib,json,subprocess,html,xml.etree.ElementTree as ET
R=Path(__file__).resolve().parents[1];S=R/'round35_sources';S.mkdir(exist_ok=True)
specs=[
 ('CBS7074_reverse_upper.jpg','round32_sources/P229436_photo.jpg',850,4950,1400,1500),
 ('CBS7074_copy_reverse_upper.jpg','round32_sources/P229436_lineart.jpg',0,880,290,620),
 ('PTS1501_row10.jpg','round27_sources/PTS1501_photo.jpg',740,2110,930,310),
 ('PTS1501_rows10_11.jpg','round27_sources/PTS1501_photo.jpg',730,2050,1040,600),
 ('PTS1502_row10.jpg','round27_sources/PTS1502_photo.jpg',730,1790,1000,270),
 ('PTS1502_rows9_10.jpg','round27_sources/PTS1502_photo.jpg',730,1620,1050,420),
 ('BM109930_row10.jpg','round26_sources/LuUtu_view6_text.jpg',60,640,2680,480),
 ('BM109930_final_graph.jpg','round26_sources/LuUtu_view6_text.jpg',2000,640,750,420),
 ('PTS1501_final_graph.jpg','round27_sources/PTS1501_photo.jpg',1300,2120,340,220)]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
rows=[]
for name,parent,x,y,w,h in specs:
 subprocess.run(['sips','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(R/parent),'--out',str(S/name)],capture_output=True,check=True)
 rows.append(dict(file='round35_sources/'+name,parent=parent,x=x,y=y,width=w,height=h,parent_sha256=sha(R/parent),sha256=sha(S/name)))
def csvout(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
csvout('round35_crops.csv',rows)
collation=[
 dict(witness='BM 109930',pid='P216745',line='10',neutral_end='X',observed='длинный горизонтальный элемент и ряд крупных однонаправленных косых элементов; без отчётливой двойной ёлочки',editorial_sign='GI',result='фотография согласуется с редакционной идентификацией; поддержано',confidence='средняя: знак у края ракурса; проверен также соседний ракурс',image='round35_sources/BM109930_row10.jpg'),
 dict(witness='PTS 1501',pid='P201507',line='10',neutral_end='Y',observed='отчётливая двойная ёлочка из противоположно наклонённых рядов, с горизонтальным элементом',editorial_sign='ZI',result='фотография согласуется с редакционной идентификацией; поддержано',confidence='средняя/высокая для различия форм; звуковое чтение отдельно не установлено',image='round35_sources/PTS1501_row10.jpg'),
 dict(witness='PTS 1502',pid='P431422',line='10',neutral_end='?',observed='нижний край снимка и кривизна поверхности затрудняют отделение последнего знака',editorial_sign='ZI',result='редакционное чтение в этом блоке не подтверждено и не опровергнуто по фотографии',confidence='недостаточная',image='round35_sources/PTS1502_rows9_10.jpg')]
csvout('round35_photographic_collation.csv',collation)
checks=[
 dict(test='T1',question='Равны ли R1–R3 из блока 34 трём установленным знакам?',result='Нет: это области. При сопоставлении заключительной строки R1 соответствует редакционному IN, R2–R3 вместе относятся к составному окончанию E3. Отдельных звуков областям не назначено.'),
 dict(test='T2',question='Можно ли назначить один номер строки одной горизонтальной полосе любого снимка?',result='Нет: нужны соседние строки и перекрывающиеся ракурсы. В PTS 1501 строки 10 и 11 занимают по две полосы внутри своего текстового поля.'),
 dict(test='T3',question='Есть ли физическое основание для различия GI / ZI?',result='Сопоставление BM 109930 / PTS 1501 поддерживает разные графические формы. PTS 1502 оставлен нерешённым.'),
 dict(test='T4',question='Получены ли новые звуковые значения?',result='Нет. Названия GI, ZI, IN, E3 заимствованы из сравниваемой модели; по одной форме клиньев звуки не выведены.'),
 dict(test='T5',question='Содержит ли древняя табличка CBS 7074 повторяющийся знак рядом с меняющимися группами?',result='Видимая повторяемость проверена на фото; девять строк и их слоговые чтения перечислены по редакции отдельно. Фотографическая проверка всех девяти строк и независимое восстановление звуков ещё не завершены.')]
csvout('round35_tests.csv',checks)
# Preserve exactly the editorial lexical hypotheses, not a new reading of the clay.
with (R/'data/round32_lexical_target_rows.csv').open(encoding='utf-8-sig') as f:
 school=[r for r in csv.DictReader(f) if r['pid']=='P229436']
csvout('round35_school_model_rows.csv',school)
external={
 'OSL_index.html':'https://oracc.museum.upenn.edu/osl/signlist/index.html',
 'OSL_G.html':'https://oracc.museum.upenn.edu/osl/signlist/l0071/index.html',
 'OSL_Z.html':'https://oracc.museum.upenn.edu/osl/signlist/l0090/index.html',
 'OSL_I.html':'https://oracc.museum.upenn.edu/osl/signlist/l0073/index.html',
 'OSL_GI.html':'https://oracc.museum.upenn.edu/osl/signlist/l0071/o0000210/index.html',
 'OSL_ZI.html':'https://oracc.museum.upenn.edu/osl/signlist/l0090/o0000609/index.html',
 'OSL_IN.html':'https://oracc.museum.upenn.edu/osl/signlist/l0073/o0000256/index.html',
 'eBL_GI.html':'https://www.ebl.lmu.de/signs/GI',
 'eBL_ZI.html':'https://www.ebl.lmu.de/signs/ZI',
 'eBL_GI.json':'https://www.ebl.lmu.de/api/signs/GI',
 'eBL_ZI.json':'https://www.ebl.lmu.de/api/signs/ZI'}
for name in ('GI','ZI'):
 d=json.loads((S/f'eBL_{name}.json').read_text());assert d['name']==name
for name in external:
 if name.startswith('OSL_'):ET.parse(S/name)
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:reader=csv.DictReader(f);fields=reader.fieldnames;reg=list(reader)
seen={r['local_file'] for r in reg}
for name,url in external.items():
 local='round35_sources/'+name
 if local not in seen:
  reg.append(dict(local_file=local,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type='modern_sign_reference_JSON' if name.endswith('.json') else 'modern_sign_reference_HTML',bytes=(S/name).stat().st_size,sha256=sha(S/name),rights_and_notes='Oracc sign data footer CC0; page template has separate notices' if name.startswith('OSL_') else 'eBL LMU; research snapshot, image reuse rights not verified; HTML is an application shell, not sign evidence'))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(reg)
report='''БЛОК 35. ПРОВЕРКА ВАРИАНТОВ ПО ФОТОГРАФИЯМ
26 октября 2026

КОНКРЕТНЫЙ РЕЗУЛЬТАТ
Сопоставление BM 109930 и PTS 1501 поддерживает различие последних графических форм редакционной строки 10, обозначаемых в изданиях GI и ZI. На PTS 1501 ясно видны два ряда противоположно наклонённых элементов — двойная «ёлочка». На BM 109930 в соответствующей позиции видна другая конфигурация с длинным горизонтальным элементом и рядом крупных однонаправленных косых элементов. Последовательность и место сверены с соседними строками и доступным соседним ракурсом BM. Это локальная палеографическая проверка, а не новое восстановление звуков. У PTS 1502 состояние и положение участка пока не позволяют вынести такое же заключение: его редакционное ZI в результат проверки не засчитывается.

ПРИВЯЗКА
Строку 9 узнаём по повторяемой последовательности четырёх комплексов, обозначаемых в редакции E2 MU NA DU3. Затем следует поле строки 10, после него заключительное поле строки 11. На PTS 1501 длинные строки 10 и 11 продолжаются нижней полосой внутри поля; поэтому подсчёт горизонтальных полос фотомонтажа не равен подсчёту редакционных строк. На BM проверены LuUtu_view6_text и LuUtu_view7_text; фрагмент view6 является производным от исходного фотомонтажа BM109930, сохранённого в round26_sources. Его координаты в этой таблице относятся к производному view6, а не к полному оригиналу. Полная прорисовка Gadd используется только как дополнительный контроль расположения: она сведена по двум конусам и не считается независимой фотографией BM 109930.

ИСПРАВЛЕНИЕ РЕЗУЛЬТАТА БЛОКА 34
Тогда мы правильно выделили визуально сходный фрагмент на двух предметах, но не привязали его к строке. Теперь он соотнесён с продолжением заключительной строки 11. R1 соответствует комплексу, который редакция обозначает IN, а R2 и R3 — частям составного окончания, обозначенного E3. Следовательно, R1–R3 нельзя превращать в три буквы или три самостоятельных слова. Это непосредственно полезный результат для построения инвентаря: граница произвольно выделенной области фотографии не равна границе знака. Прежние файлы сохранены как история наблюдений; приоритет имеет эта уточнённая привязка.

НЕЙТРАЛЬНАЯ И РЕДАКЦИОННАЯ ЗАПИСИ
Нейтрально: в соответствующей конечной позиции BM 109930 = X, PTS 1501 = Y; у третьего конуса = ?. Отображение X→GI, Y→ZI принадлежит проверяемой научной модели и согласуется с наблюдаемой формой. Оно не было независимо выведено из звучания. Совпадение контекста не доказывает ни равенство звуков, ни равенство значения X и Y. Два писца могли выбирать разные слова, близкие по значению знаки или варианты формулы; решение требует дополнительных текстов.

ПРОВЕРКА ЗВУКОВОГО ОСНОВАНИЯ
Дополнительно просмотрена фотография CBS 7074 / P229436 и её старая прорисовка PBS 12, 56. В правой части соответствующего участка повторяется один графический комплекс рядом с меняющимися группами слева. Редакция соотносит девять строк с одним знаком UD и чтениями u3, ta?-am3?, a, ah, pi-ir, hu-ud, za-la-ag, ba-ab-ba-ar, u2-tu. Это именно древний список, который можно использовать для проверки гипотезы нескольких чтений одного знака; сами латинские значения пока остаются редакционными. Из девяти опубликованных строк нельзя честно получить девять независимо восстановленных нами звуков. В этом блоке мы проверили видимость повторения, но не завершили поклиновую колляцию всех девяти строк.

ЧТО ЭТО МЕНЯЕТ В ИССЛЕДОВАНИИ
Получено физически проверяемое основание поддержать одно конкретное различие в существующей транслитерации вместо общего недоверия или общего доверия к переводам. Кроме того, установлен случай, где три выделенные области следует объединить в два редакционных знаковых комплекса. Эти результаты запрещают строить «алфавит» механическим приписыванием буквы каждому клину или каждой произвольной области снимка. Но они не устанавливают окончательно тип всей письменности: логографическая, слоговая и смешанная функции должны проверяться в серии контекстов.

ГРАНИЦА ВЫВОДА
Новых фонетических значений: 0. Новых переводов предложений: 0. Вопрос родства с казакским языком здесь не решён. Агглютинативность, происхождение извне, приоритет любого современного языка из этих форм не выводятся. На основании двух конусов нельзя ни подтвердить, ни опровергнуть весь шумерский переводческий корпус. Произношение по форме клина без внешнего языкового ограничения не восстанавливается однозначно.

ИСТОЧНИКИ И ОБРАБОТКА
Фотографии CDLI из ранее собранного архива; отдельная обработка — только кадрирование. Все координаты и хеши сохранены. Для проверки различий GI / ZI получены страницы Oracc Sign List и JSON электронного проекта eBL Университета Людвига-Максимилиана. Это современные справочники для сопоставления модели с глиной, не первичная древняя расшифровка. Иллюстрация modern_glyph_reference.png — современный шрифтовой образец Noto Sans Cuneiform: его нельзя выдавать за точную прорисовку древней формы. Права фотографий наследуются от оригиналов; новые музейные лицензии не заявляются. Книга и PDF не изменялись.

Просмотр: comparison_round35.html.
'''
(R/'observations/2026-10-26_round35_photographic_collation.txt').write_text(report,encoding='utf-8')
def im(path,caption):return '<figure><a href="'+path+'"><img src="'+path+'" loading="lazy"></a><figcaption>'+html.escape(caption)+'</figcaption></figure>'
table='<table><tr><th>Предмет</th><th>Наблюдение</th><th>Вывод проверки</th></tr>'+''.join('<tr><td>'+html.escape(r['witness'])+'</td><td>'+html.escape(r['observed'])+'</td><td>'+html.escape(r['result'])+'</td></tr>' for r in collation)+'</table>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок 35: проверка знаков по фотографиям</title><style>body{max-width:1100px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;color:#202830;background:#f6f3ec}img{max-width:100%;max-height:720px;object-fit:contain;background:#111}figure{margin:25px 0;padding:16px;background:white}figcaption{font-size:16px}table{border-collapse:collapse;width:100%}th,td{border:1px solid #b5bbc0;padding:12px;text-align:left;vertical-align:top}h1,h2{line-height:1.2}.result{padding:20px;background:#e4efdf}a{color:#194e8c}</style><h1>Два конуса: различие знаков видно на глине</h1><p>Блок 35 · 26 октября 2026 · Проверка изображений, без нового назначения звуков.</p><div class="result"><b>Результат:</b> формы в конечной позиции соответствующей строки BM 109930 и PTS 1501 различаются; наблюдение поддерживает редакционные GI / ZI. Третий конус PTS 1502 остаётся нерешённым. Это проверка конкретного места, не новый перевод надписи.</div>'''+table
page+='<h2>1. Сравниваемое место</h2>'+im('round35_sources/BM109930_row10.jpg','BM 109930. Полоса строки 10; последний знак у правого края. Источник — фотография, не прорисовка.')+im('round35_sources/PTS1501_row10.jpg','PTS 1501. Поле строки 10 с переносом; последняя двойная ёлочка расположена внизу справа.')+im('round35_sources/PTS1501_final_graph.jpg','PTS 1501: увеличенный фрагмент окончания. Видны два разнонаправленных ряда косых вдавлений.')+im('round35_sources/BM109930_final_graph.jpg','BM 109930: фрагмент окончания с прилегающими полосами. Последний знак строки 10 расположен в верхней части фрагмента; ниже начинается следующая строка.')
page+='<h2>2. Контекст и нерешённый экземпляр</h2>'+im('round35_sources/PTS1501_rows10_11.jpg','PTS 1501: соседние поля помогают различить строку 10 и заключительную строку 11. Ранее выделенные R1–R3 находятся в продолжении строки 11.')+im('round35_sources/PTS1502_rows9_10.jpg','PTS 1502: строка 9 и нижележащий участок строки 10. Конечная графема по этому ракурсу не определена надёжно.')
page+='<h2>3. Современная модель — отдельно от фотографии</h2>'+im('round35_sources/modern_glyph_reference.png','Шрифтовые GI, ZI, IN: современный Noto Sans Cuneiform. Это справочная схема, не древний оригинал.')+'<p>Имена знаков взяты из проверяемой модели: <a href="https://oracc.museum.upenn.edu/osl/signlist/l0071/o0000210/">Oracc GI</a>, <a href="https://oracc.museum.upenn.edu/osl/signlist/l0090/o0000609/">Oracc ZI</a>, <a href="https://www.ebl.lmu.de/signs/GI">eBL GI</a>, <a href="https://www.ebl.lmu.de/signs/ZI">eBL ZI</a>. Графическое соответствие не означает, что мы независимо восстановили их звучание.</p>'
page+='<h2>4. Древний материал для проверки звуков</h2>'+im('round35_sources/CBS7074_reverse_upper.jpg','CBS 7074: фрагмент фотографии. Повторяющийся комплекс в правой части исследуемой колонки сопоставляется с меняющимися группами слева.')+im('round35_sources/CBS7074_copy_reverse_upper.jpg','Старая прорисовка того же участка. Она помогает находить строки, но не заменяет фотографию.')+'<p>Девять редакционных строк списка UD вынесены в <a href="data/round35_school_model_rows.csv">отдельную таблицу гипотез чтения</a>. Их латинские слоговые значения в этом блоке независимо не восстановлены.</p>'
page+='<h2>Материалы проверки</h2><ul><li><a href="observations/2026-10-26_round35_photographic_collation.txt">Полный отчёт и исправление привязки блока 34</a></li><li><a href="data/round35_photographic_collation.csv">Решения по трём конусам</a></li><li><a href="data/round35_crops.csv">Координаты и хеши фрагментов</a></li><li><a href="data/round35_tests.csv">Пять проверок и их результаты</a></li><li><a href="round27_sources/PTS1501_photo.jpg">Полная фотография PTS 1501</a></li><li><a href="round27_sources/PTS1502_photo.jpg">Полная фотография PTS 1502</a></li><li><a href="round26_sources/LuUtu_BM109930_photo.jpg">Полный фотомонтаж BM 109930</a></li><li><a href="round32_sources/P229436_photo.jpg">Полная фотография CBS 7074</a></li></ul><p>Новых звуковых значений: 0. Новых переводов предложений: 0. Книга не изменялась.</p></html>'
(R/'comparison_round35.html').write_text(page,encoding='utf-8')
summary=dict(physical_cones_reviewed=3,graphic_difference_supported_for=2,unresolved_cones=1,school_tablets_reviewed=1,new_phonetic_values=0,new_sentence_translations=0,derived_crops=len(specs),new_external_reference_files=len(external))
(R/'data/round35_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,ensure_ascii=False))
