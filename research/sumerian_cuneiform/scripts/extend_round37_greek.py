"""Add an external-alphabet control; restored epsilon is explicitly excluded."""
from pathlib import Path
import csv,json,hashlib,subprocess,html
R=Path(__file__).resolve().parents[1];S=R/'round37_sources'
subprocess.run(['python3',str(R/'scripts/prepare_round37.py')],check=True,capture_output=True)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def writecsv(p,rows):
 with p.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
d=json.loads((S/'eBL_BM34797.json').read_text());assert d['museumNumber']['number']=='34797'
assert d['cdliImages']==['dl/photo/P370916_d.jpg']
(S/'BM34797_editorial.atf').write_text(d['atf'],encoding='utf-8')
pairs=[
 dict(canonical_line='205',cuneiform_model='mu-un-bal',greek_model='[μονο]βαλ',preserved_greek='βαλ',limit='начало греческого слова восстановлено',decision='частичный внешний контроль; не полное чтение mu-un-bal'),
 dict(canonical_line='206',cuneiform_model='mu-un-du',greek_model='[μον]δ',preserved_greek='δ',limit='сохранилась лишь малая часть греческой группы',decision='слабый локальный контроль; не самостоятельное полное чтение'),
 dict(canonical_line='207',cuneiform_model='e#',greek_model='[ε]',preserved_greek='',limit='греческая epsilon полностью восстановлена, клинописный знак повреждён',decision='ИСКЛЮЧИТЬ как непосредственное подтверждение звука E'),
 dict(canonical_line='208',cuneiform_model='pa5',greek_model='[φα]',preserved_greek='',limit='греческая передача шумерской группы восстановлена; соседнее Akkadian φαλαγ сохранено',decision='не считать непосредственно сохранившимся шумерским pa5 ↔ φα'),
 dict(canonical_line='209',cuneiform_model='pa5-lal#',greek_model='φα λαλ#',preserved_greek='φα λα + следы конечной λ',limit='конечная часть обеих записей повреждена',decision='кандидат для внешнего контроля; требуется точная поклиновая и побуквенная колляция'),
 dict(canonical_line='210',cuneiform_model='pa5#-sig',greek_model='φα σεκ#',preserved_greek='φα σε + следы κ',limit='первый клинописный комплекс и конечная греческая буква повреждены',decision='кандидат внешнего контроля SIG ↔ σε(κ); не переносить автоматически на ранний язык'),
 dict(canonical_line='211',cuneiform_model='pa5-šita3',greek_model='φα σε!ι!θ',preserved_greek='φα и группа с редакционными поправками',limit='! обозначает редакционную оценку знака, не уверенную неповреждённую фонетическую цепь',decision='не использовать как чистую независимую фонетическую опору до сверки поправок')]
writecsv(R/'data/round37_greek_controls.csv',pairs)
with (R/'data/round37_crops.csv').open(encoding='utf-8-sig') as f:crops=list(csv.DictReader(f))
for name,x,y,w,h in [('BM34797_Greek_side.jpg',480,2260,1080,1180),('BM34797_cuneiform_side.jpg',460,450,1080,1300)]:
 parent=S/'P370916_photo.jpg'
 subprocess.run(['sips','-c',str(h),str(w),'--cropOffset',str(y),str(x),str(parent),'--out',str(S/name)],check=True,capture_output=True)
 crops.append(dict(file='round37_sources/'+name,parent='round37_sources/P370916_photo.jpg',x=x,y=y,width=w,height=h,sha256=sha(S/name),parent_sha256=sha(parent)))
writecsv(R/'data/round37_crops.csv',crops)
urls={
 'Lang2023_GraecoBabyloniaca.pdf':'https://journal.fi/store/article/download/129810/78663',
 'P370916_card.html':'https://cdli.earth/artifacts/370916',
 'eBL_BM34797.json':'https://www.ebl.lmu.de/api/fragments/BM.34797',
 'P370916_photo.jpg':'https://cdli.earth/dl/photo/P370916_d.jpg'}
assert (S/'Lang2023_GraecoBabyloniaca.pdf').read_bytes()[:5]==b'%PDF-'
reg=R/'SOURCE_REGISTER.csv'
with reg.open(encoding='utf-8-sig') as f:reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
for name,url in urls.items():
 p=S/name
 rows.append(dict(local_file='round37_sources/'+name,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type='research_article_PDF' if name.endswith('.pdf') else 'photograph' if name.endswith('.jpg') else 'catalogue_or_editorial_text',bytes=p.stat().st_size,sha256=sha(p),rights_and_notes='Martin Lang 2023, Studia Orientalia Electronica 11(2),130–137, CC BY-NC-SA 4.0' if name.endswith('.pdf') else 'British Museum / CDLI / eBL; publication rights to photographs not established; bracketed restorations are not preserved letters'))
with reg.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
extra='''
ДОПОЛНЕНИЕ 37Б. ВНЕШНИЙ АЛФАВИТНЫЙ КОНТРОЛЬ

Найдено более подходящее направление для независимого закрепления звуков: Graeco-Babyloniaca. Получена фотография BM 34797 / P370916: одна сторона написана клинописью, другая — греческими буквами. Адрес обычной фотографии CDLI возвращал 404, но JSON eBL указал существующий вариант P370916_d.jpg. Он получен и просмотрен. Текст eBL содержит редакционное сопоставление сторон. Дополнительно сохранена статья Martin Lang, 2023, Studia Orientalia Electronica 11(2),130–137, DOI 10.23993/store.129810; её таблица помогает выбирать предметы с обеими сохранившимися сторонами, отделяя их от одноалфавитных фрагментов. Даты выдачи результатов поиска не приняты за год статьи: на самом PDF указан 2023.

Существенное различие: здесь фонетическая подсказка передана другим алфавитом на самом древнем предмете, а не только вторым клинописным знаком. Поэтому материал может разорвать круг взаимного переименования клинописных символов. Однако это не означает, что каждая греческая строка сохранилась полностью или что поздний писец точно воспроизводил произношение за две тысячи лет до него.

В выбранной строке 207 редакция имеет клинописное e# и греческое [ε]. Квадратные скобки показывают, что epsilon восстановлена. ЭТОТ ПРИМЕР НЕ ЗАКРЕПЛЯЕТ ЗВУК ПРОСТОГО E НЕЗАВИСИМО. В строке 208 греческое [φα] шумерской части также восстановлено. Эти строки нельзя предъявлять как сохранившиеся алфавитные подтверждения.

Есть другие, более содержательные кандидаты: строка 209 pa5-lal# ↔ φα λαλ# и строка 210 pa5#-sig ↔ φα σεκ#. На фотографии греческой стороны видны повторяющиеся алфавитные группы; в таблице сохранены все повреждения. Они позволяют целенаправленно проверять частичные соответствия, в частности SIG ↔ σε(κ), вместо произвольного назначения звука по современной ассоциации. Полная независимая построчная колляция обеих сторон ещё не завершена; поэтому здесь не заявлено окончательное новое фонетическое значение. Древнегреческое φ также нельзя автоматически приравнивать современному русскому ф или раннему шумерскому p без учёта времени и практики передачи звуков.

В итог блока добавлены четвёртый новый предмет, четвёртая новая фотография и четыре внешних файла. Всего в блоке теперь 18 внешних файлов, а не 14. Фотографические кандидаты контроля произношения выделены в data/round37_greek_controls.csv; звук E пока остаётся нерешённым. Эта граница теперь определена по конкретной утраченной букве, а не общим рассуждением о неизвестности древнего языка.
'''
report=R/'observations/2026-10-26_round37_independent_witnesses.txt';report.write_text(report.read_text()+extra,encoding='utf-8')
paragraph='''<h2>6. Существенное расширение: клинопись и греческий алфавит на одном предмете</h2><p>BM 34797 / P370916: получены обе стороны. Древняя греческая запись даёт внешний контроль для звуковых гипотез. Проверка опирается на саму фотографию и отдельно сохранённую редакционную привязку. Позднюю учебную запись нельзя автоматически переносить на произношение раннего периода.</p>'''
paragraph+='<figure><a href="round37_sources/BM34797_Greek_side.jpg"><img src="round37_sources/BM34797_Greek_side.jpg" loading="lazy"></a><figcaption>Древняя греческая сторона BM 34797. Восстановленные буквы на фотографии не дорисованы.</figcaption></figure><figure><a href="round37_sources/BM34797_cuneiform_side.jpg"><img src="round37_sources/BM34797_cuneiform_side.jpg" loading="lazy"></a><figcaption>Клинописная сторона того же предмета. Материал соединения и утраты видимы.</figcaption></figure>'
paragraph+='<p><b>Контроль E пока не получен:</b> в редакции строки 207 греческое <b>[ε]</b> восстановлено. В других строках сохранились группы <b>φα λαλ#</b> и <b>φα σεκ#</b>, сопоставленные с клинописными <b>pa5-lal#</b> и <b>pa5#-sig</b>. Это кандидаты внешней фонетической проверки; # обозначает повреждение, а не уверенное полное чтение.</p><p><a href="data/round37_greek_controls.csv">Семь выбранных соответствий и причины ограничений</a> · <a href="round37_sources/BM34797_editorial.atf">Редакционный текст обеих сторон</a> · <a href="round37_sources/P370916_photo.jpg">Полный фотомонтаж</a> · <a href="round37_sources/Lang2023_GraecoBabyloniaca.pdf">Статья Martin Lang (2023)</a> · <a href="https://www.ebl.lmu.de/fragmentarium/BM.34797">Карточка eBL</a></p>'
page=R/'comparison_round37.html';page.write_text(page.read_text().replace('</html>',paragraph+'</html>'),encoding='utf-8')
p=R/'data/round37_summary.json';summary=json.loads(p.read_text());summary.update(new_artifacts_reviewed=4,new_photographs=4,new_external_files=18,external_alphabet_control_candidates=7,preserved_epsilon_for_E=False)
p.write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,ensure_ascii=False))
