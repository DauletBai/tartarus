from pathlib import Path
import csv,json,hashlib,html
R=Path(__file__).resolve().parents[1]
def read(name):
    with (R/'data'/name).open(encoding='utf-8-sig',newline='') as f:return list(csv.DictReader(f))
def table(rows,cols):
    return '<div class="scroll"><table><tr>'+''.join('<th>'+html.escape(v)+'</th>' for k,v in cols)+'</tr>'+''.join('<tr>'+''.join('<td>'+html.escape(str(r[k]))+'</td>' for k,v in cols)+'</tr>' for r in rows)+'</table></div>'
lex=read('round30_TU_witness_rows.csv');verbs=read('round30_UrNanse11_verb_contrast.csv')
doc='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Цикл 30 — Туу / utud</title>
<style>body{max-width:1100px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;color:#202e39;background:#faf9f5}h1,h2{line-height:1.2}h1{font-size:35px}.note{background:#e8eef1;padding:18px;border-left:5px solid #476b77}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:15px}td,th{border:1px solid #c5cbd0;padding:10px;text-align:left;vertical-align:top}th{background:#e5e9e7}a{color:#205e87}img{max-width:100%;max-height:850px;object-fit:contain}figure{margin:25px 0;padding:15px;background:white}figcaption{font-size:15px}nav{display:flex;gap:18px;flex-wrap:wrap}</style>
<nav><a href="comparison_round29.html">Предыдущий цикл</a><a href="observations/2026-10-25_round30_TU_birth_and_creation.txt">Полный отчёт</a><a href="data/round30_summary.json">Итоги JSON</a></nav>
<h1>Туу / utud: древний ключ и проверка контекстом</h1><p>25 октября 2026 года. Цикл 30. Исследовательские материалы; книга не изменялась.</p>
<p class="note">Получена древняя двуязычная опора для значения рождения: <b>du-u | TU | wa-la-dam</b> в редакции MS 3178. Историческое родство с казакским <b>туу</b> остаётся открытым. Самостоятельное чтение всех клиньев целевой строки по фотографии не выполнено.</p>
<h2>Один знак — несколько употреблений</h2><p>Семь соседних строк одного свидетеля и одна повреждённая параллель другого. Звёздочка в произносительной колонке — элемент представления списка, не знак исправления. Скобки повреждений сохранены. Составная редакция Ea 07 не посчитана третьим предметом.</p>'''
doc+=table(lex,[('pid','Предмет'),('line','Строка'),('pronunciation','Подсказка в редакции'),('main_sign','Основной знак'),('akkadian','Аккадское пояснение')])
doc+='''<p><a href="data/round30_TU_witness_rows.csv">Таблица с исходными ячейками и статусами</a> · <a href="https://oracc.museum.upenn.edu/dcclt/signlists/P342645">Редакция MS 3178</a> · <a href="https://oracc.museum.upenn.edu/dcclt/signlists/P257722">Редакция CBS 00157+</a></p>
<figure><a href="round30_sources/P342645_photo.jpg"><img src="round30_sources/MS3178_reverse_context.jpg" alt="Фотография оборота MS 3178, общий участок"></a><figcaption>MS 3178 / P342645. Вырез оборота: x590, y3450, ширина 1700, высота 2280. Видны колонки и утраты. Точная фотоколляция строки ii26 остаётся открытой. Нажатие открывает полную фотографию CDLI.</figcaption></figure>
<p><a href="round30_sources/P257722_photo.jpg">Фотография CBS 00157+ с соединёнными фрагментами</a> · <a href="data/round30_visual_audit.csv">Протокол осмотра девяти изображений</a></p>
<h2>Рождение и создание изображения</h2><p>Вся надпись Ur-Nanše 11 содержит 36 строк. В ней семь <i>mu-tud</i> и девять <i>mu-du₃</i>. В редакции первая группа относится к созданию образов божеств, вторая — к постройкам. Для первой интерпретации есть отдельный контроль: другие надписи с явно записанным <i>alan</i> «статуя».</p>'''
doc+=table(verbs,[('line','Место'),('form','Форма редакции'),('signs','Названия знаков GDL')])
doc+='''<figure><img src="round30_sources/P222364_lineart.jpg" alt="Прорисовка надписи Ur-Nanše 11"><figcaption>AO 3179 / P222364. Прорисовка позволяет сопоставить повторяющийся MU с двумя разными конечными комплексами. Это современная копия графики; фотография по стандартному адресу недоступна. Она не заменена молча прорисовкой.</figcaption></figure>
<p><a href="data/round30_UrNanse11_complete.csv">Полный 36-строчный текст редакции</a> · <a href="https://oracc.museum.upenn.edu/etcsri/Q001026">ETCSRI</a></p>
<h2>Где проходит граница основы и окончания</h2><p>В модели Ягерсмы <i>dú-da</i> разбирается как основа <i>dú.d</i> и показатель <i>-a</i>. Знак DA пересекает границу морфем. Поэтому его нельзя автоматически приравнять к казакскому прошедшему <i>-ды/-ді</i>. Сравнение с <i>tuw-GAn</i> полезно по значению и функции, но окончания различаются.</p>'''
doc+=table(read('round30_tuu_grammar_comparison.csv'),[('item','Проверка'),('sumerian','Шумерская модель'),('kazakh','Казакский контроль'),('result','Результат')])
doc+='''<figure><img src="round30_sources/Jagersma_PDF666.png" alt="Грамматический разбор, с.639"><figcaption>Ягерсма 2010, печатная с.639 / PDF 666, пример51: разбор причастной формы. Это исследовательская модель, а не фотография древней надписи.</figcaption></figure>
<h2>Что проверять дальше</h2><p>Слоговые написания вне одиночного TU; конечный согласный по парадигме; различия человеческого, животного и переносного рождения; повторяемость предполагаемых переходов d/g/w на других заранее выбранных корнях. Одной пары для установления направления преемственности недостаточно.</p>
<p>Новые изображения сохранены для исследования. Права на их публикацию в книге не установлены.</p>
<nav><a href="data/round30_file_manifest.csv">Файлы и SHA-256</a><a href="data/round30_checkpoint.txt">Проверка целостности</a></nav></html>'''
(R/'comparison_round30.html').write_text(doc,encoding='utf-8')
files=sorted(set(list((R/'round30_sources').glob('*'))+list((R/'data').glob('round30_*'))+list((R/'scripts').glob('*round30.py'))+[R/'comparison_round30.html',R/'observations/2026-10-25_round30_TU_birth_and_creation.txt']))
files=[p for p in files if p.is_file() and p.name not in ['round30_file_manifest.csv','round30_checkpoint.txt']]
with (R/'data/round30_file_manifest.csv').open('w',encoding='utf-8-sig',newline='') as f:
    w=csv.DictWriter(f,fieldnames=['file','bytes','sha256']);w.writeheader()
    for p in files:w.writerow(dict(file=str(p.relative_to(R)),bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
print('Wrote HTML and manifest:',len(files),'files')
