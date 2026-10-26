"""Record a bounded image/edition collation; no automatic decipherment."""
from pathlib import Path
import csv, hashlib, html, json, subprocess

R = Path(__file__).resolve().parents[1]
S = R / 'round40_sources'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def table(name, rows):
    with (R/'data'/name).open('w', encoding='utf-8-sig', newline='') as f:
        w=csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)

# Coordinates refer to the immediate parent, not always the original photograph.
specs = [
 ('BM34797_lower_rows.jpg','round38_sources/BM34797_SJL0647.jpg',310,1870,1250,850),
 ('BM35726_Greek_face.png','round40_sources/BM35726_photo.jpg',800,2250,2900,1250),
 ('BM35726_cuneiform_face.png','round40_sources/BM35726_photo.jpg',800,150,2900,1100),
 ('BM35726_Greek_middle.png','round40_sources/BM35726_Greek_face.png',800,250,1000,600),
 ('BM35726_Greek_ama_context.png','round40_sources/BM35726_Greek_face.png',40,700,1700,330),
 ('BM35726_Greek_ama_prefix.png','round40_sources/BM35726_Greek_face.png',100,550,650,280),
 ('BM35726_cuneiform_left_context.png','round40_sources/BM35726_photo.jpg',850,420,1250,760),
 ('BM35726_1902_cuneiform_rows.png','round40_sources/PSBA24_PDF135.png',20,980,1050,390),
]
table('round40_crops.csv', [dict(file='round40_sources/'+n,parent=p,x=x,y=y,width=w,height=h,
    parent_sha256=sha(R/p),sha256=sha(S/n),operation='rectangular crop; no reflection or reconstructed strokes')
    for n,p,x,y,w,h in specs])
table('round40_page_renders.csv', [dict(file=f'round40_sources/PSBA24_PDF{n}.png',
    parent='round39_sources/PSBA24_1902.pdf',pdf_page=n,scale_to=scale,
    parent_sha256=sha(R/'round39_sources/PSBA24_1902.pdf'),sha256=sha(S/f'PSBA24_PDF{n}.png'))
    for n,scale in [(134,2000),(135,2200),(136,2200)]])

rows = [
 dict(obverse_line='2',reverse_line='2',canonical='Hh III 291',cuneiform_model='{geš}gišimmar du13-[du13]',greek_model='γισιμαρ δοδ[ο]',old_1902='gishimmar dumu-dumu; domu-domu in transcription table',status='second du13 and final Greek vowel restored; exclude as two preserved signs',physical_result='no complete independent reading'),
 dict(obverse_line='3',reverse_line='3',canonical='Hh III 292',cuneiform_model='{geš}gišimmar TUR',greek_model='γισιμαρ δορ',old_1902='gishimmar dumu; dom in transcription table',status='old dom versus modern dor requires direct mu/rho collation',physical_result='terminal letter not independently resolved on photograph'),
 dict(obverse_line='4',reverse_line='4',canonical='Hh III 293',cuneiform_model='{geš}ama gišimmar#',greek_model='αμα γισιμαρ#',old_1902='ama gishimmar; ama gisimar in transcription table',status='candidate GA2×AN ↔ αμα; following group damaged',physical_result='Greek prefix candidate located using old drawing; complete cuneiform wedge inventory not secured'),
 dict(obverse_line='5',reverse_line='5',canonical='Hh III 294',cuneiform_model='[{geš}]gišimmar lipiš bur-ra',greek_model='γισιμ!αρ λε#φεσ βωρ',old_1902='gishimmar libish burra',status='classifier restored; Greek contains editorial correction and damage',physical_result='context only; do not count corrected letters as independently read'),
]
table('round40_paired_rows.csv', rows)
tests = [
 dict(test='T1',question='Does a repeated group keep its position?',observation='In editorial rows 2/3 GIŠIMMAR precedes TUR; in row4 AMA precedes GIŠIMMAR; Greek group changes position correspondingly.',result='Useful segmentation constraint; model-guided, not a blind transcription.'),
 dict(test='T2',question='Does one graphic identity have one fixed reading?',observation='OSL canonical GA2×AN includes ama and dagal; BM35726 row4 edition selects ama.',result='Do not equate sign identifier with a single sound or semantic word.'),
 dict(test='T3',question='Can normalized sign arrays prove preservation?',observation='Row2 model has two ABZ144 entries, while ATF restores the second du13.',result='No. Brackets must override naive counts of normalized signs.'),
 dict(test='T4',question='Does αμα alone establish meaning mother?',observation='Greek transcription supplies a proposed phonetic correspondence, not a definition.',result='No independent semantic identification obtained in this block.'),
 dict(test='T5',question='Has dom versus dor been decided?',observation='1902 transcription and current edition differ; photo terminal stroke not securely resolved here.',result='Open; neither edition wins merely by authority or date.'),
 dict(test='T6',question='Has a letter for every wedge been recovered?',observation='No repeatable wedge-to-letter inventory established from these images.',result='No. A graphic-group to sound-sequence candidate is the narrower test.'),
 dict(test='T7',question='Does BM34797 prove PAP.E wedge boundaries?',observation='Lower-field crop shows substantial surface loss.',result='Full photographic collation remains incomplete; no invented strokes.'),
]
table('round40_hypothesis_tests.csv', tests)
sources = {
 'BM35726_photo.jpg':('https://www.ebl.lmu.de/api/fragments/BM.35726/photo','museum_photo_via_eBL','British Museum object; public eBL photograph endpoint. Publication/reuse rights not established; private research copy, no book insertion.'),
 'eBL_TUR.json':('https://www.ebl.lmu.de/api/signs/TUR','sign_dictionary_JSON','Modern reference model; ABZ144. Not an independently deciphered ancient text.'),
 'OSL_GA2xAN.html':('https://oracc.museum.upenn.edu/osl/signlist/l0071/o0001286/index.html','sign_list_HTML','Modern OSL reference: GA2×AN U+120BC, SLLHA237; values include ama and dagal. Not a second ancient witness.'),
 }
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:
    rd=csv.DictReader(f);fields=rd.fieldnames;reg=list(rd)
reg=[r for r in reg if not r['local_file'].startswith('round40_sources/')]
for n,(url,typ,note) in sources.items():
    p=S/n
    reg.append(dict(local_file='round40_sources/'+n,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type=typ,bytes=p.stat().st_size,sha256=sha(p),rights_and_notes=note))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:
    w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(reg)

report='''БЛОК 40. BM35726: ГРАФИЧЕСКИЙ ЗНАК, ЗВУЧАНИЕ И ПОВТОРЯЮЩАЯСЯ ГРУППА
26 октября 2026

ЗАДАЧА И ИТОГ
Продолжена проверка чтений по фотографиям, с использованием греческой стороны как внешнего алфавитного контроля. Получена цветная фотография BM35726 размером 4323×5021. Она позволяет изучать обе стороны одного физического предмета. Проверены четыре контекста редакционной таблицы и семь вопросов к ним. Выделен новый для нашего рабочего ряда кандидат GA2×AN ↔ αμα. Это локальная проверка традиционного чтения, а не новая полная дешифровка. Поклиновой инвентаризации этого знака на фотографии пока нет, поэтому окончательно независимое чтение не объявляется.

ПОЧЕМУ ДОБАВЛЕН ВТОРОЙ ПРЕДМЕТ
Нижнее поле BM34797, где проверяется PAP.E, сильно повреждено. Вырез сохранён: поверхность не позволяет уверенно подтвердить все детали старой прорисовки. Изношенность нельзя компенсировать дорисовыванием. BM35726 добавлен как второй объект с клинописью и греческими строками; результаты BM34797 не заменены и не объявлены завершёнными.

КАКИЕ СЛОИ ДАННЫХ РАЗДЕЛЕНЫ
1. Фотография BM35726 — изображение предмета. Стороны показывают одну табличку, а не два независимых древних свидетельства.
2. Pinches1902, печатная109/PDF135 — историческая прорисовка. Автор прямо пишет, что использовал варианты и дополнения из Louvre AO2131. Поэтому рисунок не равен чистой факсимильной копии всех сохранившихся клиньев BM35726.
3. Современный ATF из сохранённого round38_sources/eBL_BM35726.json — модель чтения с повреждениями, восстановлением и исправлениями. Это проверяемая гипотеза, не исходный голос писца.
4. OSL и eBL signs — справочники идентичности знаков и возможных чтений. Они не служат самостоятельным древним доказательством.

ПАРА AMA / ΑΜΑ
В модели четвёртой строки клинописной стороны GEŠ + AMA + GIŠIMMAR# соответствует αμα γισιμαρ# на греческой стороне. В модели соседних строк GIŠIMMAR стоит раньше другого компонента; на греческой стороне повторяется γισιμαρ, а в четвёртой строке перед ним появляется αμα. Это даёт ограничение на сегментацию: повторяемую группу надо узнавать и при смене её позиции, а не подбирать каждый раз новый перевод.

На фотографии выделен кандидат начала αμα по привязке к рисунку1902 года. Штрихи неглубоки, часть поверхности утрачена. Фото совместимо с предложенной привязкой, но не даёт основания объявить полностью независимую слепую транскрипцию. В клинописной стороне выделен общий левый контекст. Надёжно отделить каждый клин AMA от повреждений и соседних групп пока не удалось. Область поиска не выдаётся за точную обводку знака.

Если соответствие будет подтверждено полнее, оно свяжет целую графическую единицу со звуковой последовательностью из трёх греческих букв. Оно само по себе не назначает отдельную букву каждому клину. Также оно не устанавливает значение «мать»: транскрипция звука и перевод смысла — разные задачи. GEŠ не получает произвольного греческого эквивалента из-за отсутствия отдельного слова в этой модели: возможность непроизносимого классификатора здесь остаётся объяснением модели, которое требует других контролей.

ОДНО НАЧЕРТАНИЕ — НЕ ОБЯЗАТЕЛЬНО ОДНО ЧТЕНИЕ
Проверка OSL выявила: канонический GA2×AN (U+120BC, SLLHA237) имеет среди перечисленных чтений и ama, и dagal. Современная модель BM35726 использует идентификатор237 в строке с ama. Поэтому нельзя трактовать совпадение номера с DAGAL в другом тексте как доказанное совпадение звучания. Мы установили устройство справочного соответствия, а не независимо доказали все чтения знака.

Запросы API по /signs/AMA и /signs/DAGAL не дали карточек, но это не отсутствие соответствующих чтений: чтение и каноническое имя графического знака различаются. Валидная карточка GA2×AN сохранена. Карточка TUR/ABZ144 отдельно содержит несколько значений, включая du13, dum, dumu, tur. Из одного названия TUR нельзя заранее извлечь любое желательное значение.

DOM ИЛИ DOR
Старая таблица передаёт компонент как dom, современная третья строка — δορ. Это действительное расхождение двух редакционных передач. Конечную букву на современной фотографии мы пока не отделили достаточно уверенно от повреждений. Следовательно, в этой проверке нельзя объявить ни старое dom ошибкой, ни современное dor фальсификацией. Полезный следующий контроль — сравнить спорный штрих с бесспорными μ и ρ этого же писца, а затем с отдельной архивной фотографией при ином освещении. Результат контроля остаётся открытым.

ОТБРАКОВАН ЛОЖНЫЙ ПОДСЧЁТ
Во второй строке современный массив signs содержит два ABZ144, но ATF пишет du13-[du13]. Второй компонент восстановлен. Поэтому массив нормализованных знаков нельзя использовать как доказательство двух физически сохранившихся начертаний. В других строках сохранены #, !, угловые и квадратные скобки. Старые дополнения из AO2131 не считаются новыми целыми знаками BM35726.

ГРАНИЦА ВЫВОДА
Установлена проверяемая пара графического комплекса и алфавитной передачи; получено новое фото и уточнены условия проверки. Не получены: полный алфавит клиньев, полностью независимая фонетика AMA, новый перевод предложения, доказательство казакского происхождения шумерского или его опровержение. Однотипные внешние сходства слов пока не определяют направление родства. Сравнение уже видело существующие издания, поэтому не называется слепым независимым экспериментом.

СОХРАННОСТЬ ФАЙЛОВ
Исходная фотография сохранена без изменений. Производные — прямоугольные вырезы и рендеры исторического PDF; зеркального отражения BM35726, улучшения нейросетью и дорисовки нет. Координаты и хеши записаны. Старые вырезы с рабочими именами middle и ama_context сохраняют историю поиска; их имена не удостоверяют точность привязки. Права на перепечатку современной фотографии не установлены: в книгу она не добавлена. Книга и её PDF не изменялись.
'''
reportpath='observations/2026-10-26_round40_BM35726_sign_and_sound.txt'
(R/reportpath).write_text(report,encoding='utf-8')
def im(p,c): return '<figure><a href="'+p+'"><img loading="lazy" src="'+p+'"></a><figcaption>'+html.escape(c)+'</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок40 · BM35726</title><style>body{max-width:1150px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f7f3eb;color:#202830}img{max-width:100%;max-height:1000px;object-fit:contain}figure{margin:24px 0;padding:16px;background:white}figcaption{font-size:16px}table{border-collapse:collapse;font-size:15px}td,th{border:1px solid #aaa;padding:10px;vertical-align:top;text-align:left}.result{padding:20px;background:#e5eee1}.scroll{overflow:auto}h1,h2{line-height:1.2}</style><h1>BM35726: от изображения к проверке звучания</h1><p>Блок40 · 26 октября2026</p><div class="result">Добавлена фотография обеих сторон. Проверяется соответствие GA₂×AN ↔ αμα и перемещение повторяющейся группы γισιμαρ. Полного независимого чтения знака по всем клиньям пока нет. Разница старого dom и современного dor остаётся открытой.</div>'''
page+='<h2>Исходный предмет</h2>'+im('round40_sources/BM35726_photo.jpg','Цветной фотомонтаж BM35726. Один предмет; современное изображение не является новым древним текстом. Нажмите для оригинального размера.')
page+='<h2>Старая прорисовка и её ограничения</h2>'+im('round40_sources/PSBA24_PDF135.png','Pinches1902, печатная109. Вводная строка прямо указывает варианты и дополнения из Louvre AO2131. Их нельзя автоматически приписывать сохранившейся поверхности BM35726.')
page+='<h2>Две стороны: общий контекст</h2>'+im('round40_sources/BM35726_cuneiform_face.png','Клинописная сторона без отражения и дорисовки. Полная поклиновая разметка не завершена.')+im('round40_sources/BM35726_Greek_face.png','Греческая сторона без отражения. Ряд повторяющихся групп используется для проверки сегментации.')+im('round40_sources/BM35726_Greek_ama_prefix.png','Кандидат начала αμα, локализованный с помощью старого рисунка. Частичная фотографическая поддержка; не слепое прочтение.')+im('round40_sources/BM35726_cuneiform_left_context.png','Область поиска на левой части клинописной стороны; не точная обводка AMA.')
page+='<h2>Четыре сопоставляемых контекста</h2><div class="scroll"><table><tr><th>Строка</th><th>Клинописная модель</th><th>Греческая модель</th><th>Результат проверки</th></tr>'
for r in rows: page+='<tr>'+''.join('<td>'+html.escape(r[k])+'</td>' for k in ['canonical','cuneiform_model','greek_model','physical_result'])+'</tr>'
page+='</table></div><h2>Что проверено</h2><ul>'
for t in tests: page+='<li>'+html.escape(t['question']+' '+t['result'])+'</li>'
page+='</ul><h2>Рабочие данные</h2><ul>'
links=[(reportpath,'Полный отчёт'),('data/round40_paired_rows.csv','Строки, повреждения и различия редакций'),('data/round40_hypothesis_tests.csv','Семь проверок'),('data/round40_crops.csv','Координаты и хеши вырезов'),('round40_sources/PSBA24_PDF136.png','Таблица и обсуждение1902 года'),('round40_sources/OSL_GA2xAN.html','Сохранённая карточка GA₂×AN'),('round40_sources/eBL_TUR.json','Сохранённая карточка TUR'),('round40_sources/BM34797_lower_rows.jpg','BM34797: изношенное нижнее поле'),('comparison_round39.html','Предыдущий блок')]
for p,t in links: page+='<li><a href="'+p+'">'+html.escape(t)+'</a></li>'
page+='</ul><p>Фото получено через <a href="https://www.ebl.lmu.de/api/fragments/BM.35726/photo">eBL</a>; графический справочник — <a href="https://oracc.museum.upenn.edu/osl/signlist/l0071/o0001286/index.html">OSL</a>. Современная редакция — сохранённый файл round38_sources/eBL_BM35726.json. Новых полных переводов:0. Книга не изменялась.</p></html>'
(R/'comparison_round40.html').write_text(page,encoding='utf-8')
summary=dict(new_external_files=3,new_ancient_objects_photographed=1,paired_contexts_reviewed=4,hypothesis_tests=7,crop_records=len(specs),new_candidate_group='GA2×AN ↔ αμα',complete_wedge_collation=False,dom_dor_resolved=False,new_fully_independent_sound_values=0,new_complete_translations=0)
(R/'data/round40_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
(R/'data/round40_next_targets.txt').write_text('1. BM35726: exact mu/rho comparison within the same Greek hand; independent lighting needed if photograph remains ambiguous.\n2. Full wedge-level tracing of GA2×AN in row4; distinguish breaks from incisions before assigning sounds.\n3. Obtain AO2131 original image separately; do not treat Pinches composite copy as two independently checked tablets.\n',encoding='utf-8')
print(json.dumps(summary,ensure_ascii=False,indent=2))
