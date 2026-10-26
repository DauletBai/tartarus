from pathlib import Path
import csv,html,hashlib
R=Path(__file__).resolve().parents[1]
def read(n):
 with (R/'data'/n).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def table(rows,cols):return '<div class="scroll"><table><tr>'+''.join('<th>'+html.escape(v)+'</th>' for k,v in cols)+'</tr>'+''.join('<tr>'+''.join('<td>'+html.escape(str(x[k]))+'</td>' for k,v in cols)+'</tr>' for x in rows)+'</table></div>'
lex=read('round32_lexical_target_rows.csv')
doc='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Цикл32 — UD и второй корень</title><style>body{max-width:1100px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#faf9f5;color:#25353e}h1,h2{line-height:1.2}.note{padding:18px;background:#e5ecee;border-left:5px solid #477786}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:14px}td,th{border:1px solid #cbd1d0;padding:9px;text-align:left;vertical-align:top}img{max-width:100%;max-height:760px;object-fit:contain}figure{margin:25px 0;padding:15px;background:white}figcaption{font-size:15px}a{color:#205d83}</style>
<p><a href="comparison_round31.html">Цикл31</a> · <a href="observations/2026-10-26_round32_UD_school_and_second_root.txt">Полный отчёт</a> · <a href="data/round32_summary.json">Итоги JSON</a></p>
<h1>UD: школьный ряд. Pad₃: второй сравнительный корень</h1><p>26 октября2026. Работа непосредственно в tartarus.</p>
<p class="note">Девять разных подсказок для одного UD не поддерживают его трактовку как единственного слова «родился». У pad₃ проверены147 редакторских употреблений и древние двуязычные соответствия. Историческая связь с тап-/таб- остаётся проверяемой гипотезой.</p>
<h2>Девять строк CBS7074</h2><p>Старовавилонская учебная табличка, Ниппур. Подсказки и основной знак — две древние колонки; значения лемм в заголовках добавляет редакция. Повреждения сохранены.</p>'''
doc+=table([x for x in lex if x['pid']=='P229436'],[('line','Место'),('pronunciation','Подсказка редакции'),('main_sign','Знак')])
doc+='''<figure><a href="round32_sources/P229436_photo.jpg"><img src="round32_sources/CBS7074_reverse_context.jpg" alt="Оборот CBS7074"></a><figcaption>Фотографический вырез: x820 y4600 w2200 h2650. Повторения графического комплекса сопоставлены с копией; полной проверки каждого клина нет. Нажатие открывает фотокомпозит.</figcaption></figure>
<p><a href="round32_sources/P229436_lineart.jpg">Прорисовка</a> · <a href="https://oracc.museum.upenn.edu/dcclt/P229436">Редакция CBS7074</a></p>
<h2>Различить PAD и IGI.RU</h2><p>На одном предмете P250504 запись pa-ad сопоставлена с PAD, другая pa-a — с IGI.RU и аккадским a-tu-u₂. Первая строка лишилась аккадского пояснения. Общая латинская похожесть не делает разные графические комплексы одним корнем.</p>'''
doc+=table([x for x in lex if x['pid']=='P250504'],[('row_id','Строка'),('all_cells','Ячейки редакции')])
doc+='''<h2>Восстановленный pa-ad и древние значения</h2><p>У P527017 [pa-ad] восстановлено редактором. У K197 представлены соответствия называнию и клятве. Нельзя считать все147 словоупотреблений буквальным нахождением только по единой корпусной глоссе find.</p>'''
doc+=table([x for x in lex if x['pid'] in ['P527017','P365268']],[('pid','Предмет'),('line','Место'),('all_cells','Ячейки редакции')])
doc+='''<p><a href="round32_sources/P250504_photo.jpg">Фото P250504</a> · <a href="round32_sources/P365268_photo.jpg">Фото K197 с соединениями</a> · <a href="data/round32_visual_audit.csv">Пределы осмотра</a></p>
<h2>Казакское сопоставление</h2><p>Современное тауып имеет употребление «найдя», подтверждённое грамматическим примером. Между pad и tap нужно объяснить порядок согласных: p-a-d и t-a-p. Гипотеза перестановки с оглушением требует других независимых примеров; переход d/g/w из пары utud/tuw- этого не объясняет.</p>
<figure><img src="round32_sources/Kazakh_PDF69.png" alt="Казакское нахождение денег, грамматический пример341"><figcaption>Dotton/Wagner, печатнаяс63/PDF69. Контроль современного значения, не древняя этимологическая цепочка.</figcaption></figure>'''
doc+=table(read('round32_comparative_tests.csv'),[('pair','Пара'),('positive','Положительное наблюдение'),('required','Что проверить'),('result','Статус')])
doc+='''<p><a href="data/round32_pad_corpus.csv">147 корпусных позиций</a> · <a href="data/round32_lexical_target_rows.csv">19 лексических строк</a> · <a href="data/round32_file_manifest.csv">Файлы и SHA-256</a> · <a href="data/round32_checkpoint.txt">Проверка</a></p><p>Книга/PDF не менялись. Новых независимо установленных фонетических значений нет. Права на публикацию новых изображений не установлены.</p></html>'''
(R/'comparison_round32.html').write_text(doc)
files=sorted(set(list((R/'round32_sources').glob('*'))+list((R/'data').glob('round32_*'))+list((R/'scripts').glob('*round32.py'))+[R/'comparison_round32.html',R/'observations/2026-10-26_round32_UD_school_and_second_root.txt']))
with (R/'data/round32_file_manifest.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=['file','bytes','sha256']);w.writeheader()
 for p in files:
  if p.is_file() and p.name not in ['round32_file_manifest.csv','round32_checkpoint.txt']:w.writerow(dict(file=str(p.relative_to(R)),bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
print('HTML and manifest written')
