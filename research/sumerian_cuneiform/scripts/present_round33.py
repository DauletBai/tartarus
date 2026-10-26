from pathlib import Path
import csv,json,html,hashlib
R=Path(__file__).resolve().parents[1]
def read(n):
 with (R/'data'/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def table(rows,cols):return '<div class="scroll"><table><tr>'+''.join('<th>'+html.escape(v)+'</th>' for k,v in cols)+'</tr>'+''.join('<tr>'+''.join('<td>'+html.escape(str(x[k]))+'</td>' for k,v in cols)+'</tr>' for x in rows)+'</table></div>'
s=json.loads((R/'data/round33_summary.json').read_text())
doc='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Цикл33 — весь архив перед чтением</title><style>body{max-width:1160px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#faf9f5;color:#25353c}h1,h2{line-height:1.2}.note{padding:18px;background:#e4edef;border-left:5px solid #447681}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:14px}td,th{border:1px solid #cbd2d1;padding:8px;text-align:left;vertical-align:top}th{background:#e8edeb}img{max-width:100%;max-height:950px;object-fit:contain}figure{margin:25px 0;padding:15px;background:white}figcaption{font-size:15px}a{color:#205d83}.sheets{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:15px}.sheets img{max-height:600px}</style>
<p><a href="comparison_round32.html">Цикл32</a> · <a href="observations/2026-10-26_round33_archive_wide_image_first_review.txt">Полный отчёт</a> · <a href="data/round33_summary.json">Итоги JSON</a></p>
<h1>От следа инструмента к знаку: сплошной обзор архива</h1><p>26 октября2026. Без назначения новых звуков по латинским леммам.</p>
<p class="note"><b>Все95 отобранных первичных изображений просмотрены обзорно.</b> Это не полная колляция всех клиньев. Из23 доступных PDF получено число страниц5229; полное прочтение всех страниц не заявлено. Полной новой расшифровки пока нет.</p>
<h2>Объём и охват</h2><p>В исходном срезе282 медиафайла:256 растровых изображений,2 схемы,24 PDF. Один PDF пустой. Число файлов не равно числу древних предметов. Есть повторные ракурсы, копии и монтажи нескольких объектов. Идентификаторы V001–V095 — локальные номера изображений.</p>
<p><a href="data/round33_media_inventory.csv">Весь реестр282 файлов</a> · <a href="data/round33_primary_review.csv">Отбор95 изображений</a> · <a href="data/round33_document_pages.csv">Все PDF и число страниц</a></p>
<h2>Шесть обзорных листов</h2><p>Нажмите изображение для открытия полного листа. Для чтения мелких деталей используйте оригинал по ссылке в таблице ниже, а не уменьшенную миниатюру.</p><div class="sheets">'''
for i in range(1,7):
 p=f'round33_sources/contact_{i:02d}.jpg';doc+=f'<figure><a href="{p}"><img src="{p}" alt="Обзорный лист{i}"></a><figcaption>Лист{i}: V{(i-1)*16+1:03d}–V{min(i*16,95):03d}. Производный монтаж, не новый древний свидетель.</figcaption></figure>'
doc+='''</div><h2>Детальное наблюдение: след и комплекс</h2><p>Два музейных снимка открыты в полном разрешении. На левом комплексе различимы контур, внутренние поперечины и примыкающие элементы; в центральном поле — округлые участки, наклонные вдавления и границы полей. Ни букв, ни слов этим формам пока не назначено.</p>
<figure><img src="round33_sources/Met327385_complex_left.jpg" alt="Контурный комплекс без звукового назначения"><img src="round33_sources/Met327385_impressions_mid.jpg" alt="Округлые участки, оттиски и границы"><figcaption>Met327385, исходный снимок DP293245. ВырезыM01/M02. Подробные координаты и альтернативные членения сохранены в CSV. Восемь кандидатов на округлые участки не равны восьми доказанным отдельным нажатиям или числовому значению8.</figcaption></figure>
<p><a href="round33_sources/Met327385_face_crop.jpg">Полное поле первого предмета</a> · <a href="round33_sources/Met327384_face_crop.jpg">Другой предмет для сравнения</a> · <a href="data/round33_micro_observations.csv">Четыре наблюдения с координатами</a></p>
<h2>Буква, слог, слово или более длинная запись?</h2><p>Сначала проверяется граница графической единицы, затем её языковая функция. Повторяемость не назначает единственное звучание: замена условных названий сохраняет рисунок повторов. Для звукового ключа нужны независимые ограничения и успешные предсказания на новых строках.</p>'''
doc+=table(read('round33_unit_role_tests.csv'),[('unit','Единица'),('observed','Наблюдение'),('candidates','Возможные функции'),('needed','Что различит варианты'),('status','Статус')])
doc+='''<h2>Что это говорит о происхождении системы?</h2>'''+table(read('round33_origin_hypotheses.csv'),[('observation','Наблюдение'),('human_development','Человеческое развитие'),('external_origin','Внешняя передача'),('discrimination','Различающий результат')])
doc+='''<p>Стройность письма, происхождение устного языка и происхождение разума — разные вопросы. Доказательство одного не заменяет два других.</p>
<h2>Все95 записей обзорного осмотра</h2><p>Каждая строка имеет одинаковое ограничение: обзор выполнен, каждый клин полностью не коллирован. Звуковое назначение —0.</p>'''
review=read('round33_visual_overview.csv');doc+='<div class="scroll"><table><tr><th>Номер</th><th>Оригинал/копия</th><th>Наблюдение</th></tr>'
for x in review:doc+=f'<tr><td>{x["review_id"]}</td><td><a href="{html.escape(x["file"],quote=True)}">{html.escape(x["file"])}</a></td><td>{html.escape(x["observation"])}</td></tr>'
doc+='''</table></div><p><a href="data/round33_visual_overview.csv">CSV обзора</a> · <a href="data/round33_file_manifest.csv">Файлы и SHA-256</a> · <a href="data/round33_checkpoint.txt">Проверка</a></p><p>Книга/PDF не менялись. Новые внешние источники в этом цикле не загружались. Созданы производные исследовательские материалы из сохранённых оригиналов.</p></html>'''
(R/'comparison_round33.html').write_text(doc)
files=sorted(set(list((R/'round33_sources').glob('*'))+list((R/'data').glob('round33_*'))+list((R/'scripts').glob('*round33.py'))+list((R/'scripts').glob('*round33.swift'))+[R/'comparison_round33.html',R/'observations/2026-10-26_round33_archive_wide_image_first_review.txt']))
with (R/'data/round33_file_manifest.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=['file','bytes','sha256']);w.writeheader()
 for p in files:
  if p.is_file() and p.name not in ['round33_file_manifest.csv','round33_checkpoint.txt']:w.writerow(dict(file=str(p.relative_to(R)),bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
print('HTML and manifest written')
