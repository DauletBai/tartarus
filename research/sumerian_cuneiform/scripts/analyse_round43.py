"""A bounded historical chain for туу; keep cuneiform reading separate."""
from pathlib import Path
import csv,json,hashlib,re,subprocess,html
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
R=Path(__file__).resolve().parents[1];S=R/'round43_sources'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def out(name,rows):
 with (R/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)

class Tables(HTMLParser):
 def __init__(self):super().__init__();self.tables=[];self.depth=0;self.rows=[];self.row=[];self.cell=None
 def handle_starttag(self,t,a):
  if t=='table':
   self.depth+=1
   if self.depth==1:self.rows=[]
  if self.depth==1:
   if t=='tr':self.row=[]
   elif t in ('td','th'):self.cell=[]
 def handle_data(self,d):
  if self.cell is not None:self.cell.append(d)
 def handle_endtag(self,t):
  if self.depth==1:
   if t in ('td','th') and self.cell is not None:self.row.append(' '.join(''.join(self.cell).split()));self.cell=None
   elif t=='tr':self.rows.append(self.row)
  if t=='table':
   if self.depth==1:self.tables.append(self.rows)
   self.depth-=1
t=Tables();t.feed((S/'Karakalpak_comparative_article.html').read_text())
selected=[v for v in t.tables if any('naissance' in c for row in v for c in row)]
assert len(selected)==1
comparison=[]
for row in selected[0][2:]:
 assert len(row)==5
 label,*forms=row
 comparison.append(dict(source_label=label,cord=forms[0],birth=forms[1],raw=forms[2],touch=forms[3],evidence_type='reconstruction' if label=='Clauson' else ('historical forms reported by author' if label in ['Codex cuman.','vieux türk'] else 'modern comparative forms reported by author'),source='Jacquesson2002 printed111 / PDF20; one comparative study, not seventeen independent decipherments'))
assert len(comparison)==17
out('round43_regional_comparison.csv',comparison)

paradigm=[
 dict(source_form='tuarmen / togarmen',Latin_column='nasco (sic!)',Kazakh_comparandum='туамын',function='first-person birth form in edition; alternate hand noted by Kuun',analysis_status='printed forms checked visually; no autograph manuscript image obtained'),
 dict(source_form='togdum',Latin_column='nasci (sic!)',Kazakh_comparandum='тудым',function='past first-person reading supported by related paradigm and finite-past controls',analysis_status='Latin label irregular; do not treat nasci alone as an unambiguous past-tense translation'),
 dict(source_form='toggil',Latin_column='nascere (sic!)',Kazakh_comparandum='ту',function='imperative; Kuun index printed288 explicitly says imp.',analysis_status='same lexical field, different imperative morphology; not exact modern Kazakh spelling'),
 dict(source_form='togmac',Latin_column='natura',Kazakh_comparandum='туу / тумақ',function='nominal/infinitival comparison',analysis_status='Latin natura not silently replaced by nasci; suffix history requires further comparison'),
 dict(source_form='tougam',Latin_column='natus',Kazakh_comparandum='туған',function='born; participial comparison',analysis_status='printed tougam preserved; final m not silently rewritten to n; manuscript check still needed')]
out('round43_birth_paradigm.csv',paradigm)
controls=[
 dict(present='atagirmen',past='atagdim',Latin_present='nomino alium',Latin_past='nominauj',modern_past='атадым',past_tail='dim',role='finite first-person past control'),
 dict(present='jarastururmen',past='jarastirdum',Latin_present='ordino',Latin_past='ordinauj',modern_past='жарастырдым',past_tail='dum',role='finite first-person past control'),
 dict(present='togarmen',past='togdum',Latin_present='nasco (sic!)',Latin_past='nasci (sic!)',modern_past='тудым',past_tail='dum',role='birth case; already seen, not blind held-out test'),
 dict(present='kar yayar',past='kar yagdi',Latin_present='ningit',Latin_past='ninguit',modern_past='қар жауды',past_tail='di',role='third-person contrast: no final m; lexical and phonetic control')]
out('round43_inflection_controls.csv',controls)
for c in controls[:3]:assert re.search(r'd[ui]m$',c['past'])
assert controls[3]['past'].endswith('di') and not controls[3]['past'].endswith('m')
phonetic=[
 dict(older='tag',Kazakh='taw / тау',feature='back-vowel environment; g versus w',source='Jacquesson2002 sectionIII.1.2 and printed111',scope='comparison reported by author; not a sound recovered from a Sumerian wedge'),
 dict(older='baag / bag-',Kazakh='baw / бау',feature='back-vowel environment; g versus w',source='Jacquesson2002 printed111',scope='cord/bond; do not conflate unrelated garden homonyms'),
 dict(older='tog- / tug-',Kazakh='tuw- / туу',feature='back-vowel environment; g versus w',source='Jacquesson2002 printed111; Asimkhan2021 PDF20',scope='old inscribed form reported through editions; medieval printed variants checked separately'),
 dict(older='teg-',Kazakh='tij-üw / тию',feature='front-vowel contrast; g versus j',source='Jacquesson2002 printed111',scope='citation includes verbal suffix; not a free universal substitution g→w')]
out('round43_sound_controls.csv',phonetic)
chain=[
 dict(link='Sumerian sign TU to birth reading tud/du2',status='traditional model under partial visual audit',support='ancient Ea row du-u | TU | wa-la-dam; MS3178 photograph',gap='full independent wedge collation not completed; exact d/t phonetics not newly established'),
 dict(link='Sumerian tud/dud to early tog/tug',status='not established',support='shared broad semantic field and limited sound resemblance',gap='no demonstrated repeated d→g correspondence or dated transmission chain'),
 dict(link='early tog/tug to medieval birth forms',status='documented in linguistic publications; selected medieval edition checked',support='Asimkhan2021 E29 citation; Kuun1880 printed40 and288; Jacquesson2002',gap='not a new direct decipherment of E29; manuscript autograph not inspected'),
 dict(link='medieval steppe birth forms to modern Kazakh tuw/туу',status='supported historical comparison',support='multi-form paradigm, related inflection, regional g/w controls',gap='does not establish Kazakh as sole ancestor of all compared languages')]
out('round43_chain_status.csv',chain)

# The sips bug is restricted to (0,0), not to each zero axis separately.
regression=json.loads((S/'coordinate_regression.json').read_text())
assert len(regression)==8
assert all(v['passed'] for v in regression if v['method']=='strict')
assert [v['passed'] for v in regression if v['method']=='sips']==[False,True,True,True]
edge=[]
for p in sorted((R/'data').glob('round*_crops.csv')):
 if p.name.startswith('round43'):continue
 with p.open(encoding='utf-8-sig') as f:rows=list(csv.DictReader(f))
 for row in rows:
  if row.get('x')=='0' or row.get('y')=='0':
   edge.append(dict(register=p.name,file=row['file'],x=row['x'],y=row['y'],both_zero=row['x']=='0' and row['y']=='0',decision='single zero axis is not affected in the regression; no historical image replaced'))
assert len(edge)==9 and not any(x['both_zero'] for x in edge)
out('round43_previous_edge_crop_audit.csv',edge)
out('round43_current_crops.csv',[
 dict(file='round43_sources/MS3178_left_strip.png',parent='round30_sources/MS3178_reverse_context.jpg',x=0,y=0,width=300,height=1800,operation='CoreGraphics raw-top-left; corrects current exploratory sips(0,0) crop',sha256=sha(S/'MS3178_left_strip.png')),
 dict(file='round43_sources/MS3178_central_strip.png',parent='round30_sources/MS3178_reverse_context.jpg',x=260,y=0,width=430,height=1500,operation='CoreGraphics raw-top-left; exploratory context, not a confirmed row26 crop',sha256=sha(S/'MS3178_central_strip.png'))])
# Image-first observation: preserve exploratory strips without calling them exact lines.
photo_crops=[]
for name,x,y,w,h,status in [
 ('CBS12709_TU_block',80,650,1120,620,'overview; lower context cut off'),
 ('CBS12709_row5_candidate',80,700,1120,220,'exploratory strip; intersects neighbouring row; not exact row5'),
 ('CBS12709_row6_candidate',80,900,1120,220,'exploratory strip; not exact row6'),
 ('CBS12709_row7_candidate',80,1100,1120,220,'exploratory strip; intersects changed lower complex; not exact row7'),
 ('CBS12709_extended_context',80,650,1120,850,'visually checked context; repeated right complexes and changing left groups')]:
 photo_crops.append(dict(file='round43_sources/'+name+'.png',parent='round29_sources/CBS12709_written_face.jpg',parent_sha256=sha(R/'round29_sources/CBS12709_written_face.jpg'),x=x,y=y,width=w,height=h,sha256=sha(S/(name+'.png')),status=status))
out('round43_CBS12709_crops.csv',photo_crops)
out('round43_CBS12709_observations.csv',[
 dict(level='photograph',finding='Similar right-hand complexes recur below the horizontal fracture; left-hand groups vary.',confidence='structural visual observation; no exact wedge inventory',inference='repetition with contrasting contexts; no new sound assigned'),
 dict(level='published model',finding='Edition assigns o i5 tu-u2 | TU; o i6 du-u2 | TU; o i7 hu-du-us | TU.',confidence='inherited line identification and phonetics',inference='compatible with visual layout; not an independently deciphered syllable sequence'),
 dict(level='limit',finding='Narrow exploratory crops do not isolate exact lines; no separate Akkadian meaning column on this item.',confidence='explicit restriction',inference='Do not use this photograph alone to equate TU with Kazakh туу or derive birth meaning.')])
out('round43_page_map.csv',[
 dict(file='Asimkhan2021_verbs.pdf',PDF_page=20,printed_page='260',content='birth and E29 comparison; catalogue/header gives article range261–291 but PDF footer runs241–271'),
 dict(file='Asimkhan2021_verbs.pdf',PDF_page=25,printed_page='265',content='author conclusion: selected ninety shared verbs; not percentage of all vocabulary'),
 dict(file='Jacquesson2002.pdf',PDF_page=20,printed_page='111',content='four-meaning comparative table and conditioned consonant discussion'),
 dict(file='Kuun1880_CodexCumanicus.pdf',PDF_page=188,printed_page='40',content='five birth entries, snowfall and two naming/ordering finite paradigms'),
 dict(file='Kuun1880_CodexCumanicus.pdf',PDF_page=436,printed_page='288',content='birth index and explicit imperative label'),
 dict(file='Kuun1880_CodexCumanicus.pdf',PDF_page=437,printed_page='289',content='additional birth-related index forms')])

urls={
 'eBL_TU.json':('https://www.ebl.lmu.de/api/signs/TU','modern_sign_reference','TU ABZ58,UnicodeU+12305; sign identity not independently deciphered sound.'),
 'Asimkhan2021_verbs.pdf':('https://dergipark.org.tr/en/download/article-file/1836305','author_article_PDF','Selected pages and target sections read; full article downloaded. Printed pagination differs from bibliographic range; no original E29 photograph obtained.'),
 'Yildiz2020_E29.pdf':('https://dergipark.org.tr/tr/download/article-file/1501290','author_collation_PDF','Read source-identification and image-reference sections; not an independent check of the whole stele or all proposed readings.'),
 'Karakalpak_comparative_article.html':('https://journals.openedition.org/asiecentrale/658','author_article_HTML','Actual full article retrieved, not bot-challenge page; comparative table extracted. One study, not seventeen independent witnesses.'),
 'Jacquesson2002.pdf':('https://journals.openedition.org/asiecentrale/pdf/658','author_article_PDF','Same study as HTML;46 PDF pages; printed111 target checked visually. HTML and PDF not independent authorities.'),
 'Kuun1880_commons_locator.json':('https://commons.wikimedia.org/w/api.php?action=query&generator=search&gsrsearch=codexcumanicusbi00kuunuoft&gsrnamespace=6&gsrlimit=3&prop=imageinfo&iiprop=url&format=json','archive_locator_JSON','Location of an1880 edition scan; not ancient evidence itself.'),
 'Kuun1880_CodexCumanicus.pdf':((S/'Kuun1880_download_url.txt').read_text().strip(),'historical_edition_scan','1880 edition,552 PDF pages; selected printed40,288,289 checked. Not a photograph/facsimile of medieval handwriting. Commons source labels scan public domain; modern articles have separate rights.'),
 'Bitig_E29.html':('https://www.bitig.org/?lang=e&mod=1&tid=2&oid=116&m=2','access_response_HTML','Redirect-to-lander response145 bytes, not a usable photograph or text; excluded from historical evidence.'),
 'Schoyen_MS3178.html':('https://www.schoyencollection.com/19-dictionaries-lexical-texts/ms-3178','collection_owner_catalogue_HTML','Owner description,2+2 columns; catalogue date1400–1100BC not independently measured here.')}
regpath=R/'SOURCE_REGISTER.csv'
with regpath.open(encoding='utf-8-sig') as f:rd=csv.DictReader(f);fields=rd.fieldnames;register=list(rd)
register=[v for v in register if not v['local_file'].startswith('round43_sources/')]
for n,(url,kind,note) in urls.items():
 p=S/n;assert p.is_file() and p.stat().st_size>0
 register.append(dict(local_file='round43_sources/'+n,source_url=url,retrieved_date_Asia_Qostanay='2026-10-26',type=kind,bytes=p.stat().st_size,sha256=sha(p),rights_and_notes=note))
with regpath.open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(register)

report='''БЛОК43. ТУУ: ИСТОРИЧЕСКАЯ ПАРАДИГМА И НЕУСТАНОВЛЕННЫЙ ШУМЕРСКИЙ ПЕРЕХОД
26 октября2026

СУЩЕСТВЕННЫЙ РЕЗУЛЬТАТ
Проверено историческое издание Codex Cumanicus1880 года. На печатной40/PDF188 виден ряд tuarmen/togarmen, togdum, toggil, togmac, tougam. Рядом записаны kar yagdi, atagdim, jarastirdum. Получена предметная сравнительная опора: несколько форм рождения и параллельные грамматические окончания, сопоставимые с казакскими тудым, қар жауды, атадым, жарастырдым. Это подтверждает сохраняющийся в казакском исторический степной языковой слой на уровне слов и грамматических построений. Это проверка известного памятника и опубликованных сравнений, не объявление неизвестной науке находки.

В цепочке выделены разные по надёжности участки. Ранние tog-/tug-, средневековые формы и современные tuw-/туу документированы в языковых публикациях; выбранные формы средневекового текста проверены по скану печатного издания. Переход шумерского tud/dud к tog/tug этим не установлен. Нельзя переносить доказательность одного участка на всю цепь. Первый источник казакского или всех агглютинативных языков не определён.

1. ЧТО ИМЕННО ПРОЧИТАНО
Получен полный552-страничный скан издания Géza Kuun1880; предметно просмотрены печатные40,288,289. Вся книга не объявляется полностью проанализированной. На печатной40 слова читаются непосредственно на изображении печати, а не только в OCR. Это издание средневекового текста, не фотография средневековой рукописи. Текущая попытка получить E29 через указанный в научной статье bitig.org дала страницу переадресации, а не фотографии. Новая независимая расшифровка рун E29 не выполнена.

Издание сохраняет неправильные латинские формы с пометами sic. Особенно nasci нельзя само по себе переводить как однозначное первое лицо прошедшего времени. Форма togdum рассматривается вместе с другими глаголами и традиционным разбором. Индекс на печатной288 прямо отмечает toggil как повелительную форму. Tougam в таблице сохраняется именно так, как напечатано; окончание не исправлено нами в -gan. Автор1880 года связывает вариант tuarmen с другой рукой; его промежуточное tuvarmen в примечании — объяснение редактора, не автоматически отдельная засвидетельствованная запись писца.

2. ГРАММАТИЧЕСКИЙ КОНТРОЛЬ
Две соседние пары снабжены латинскими формами первого лица: nomino/nominauj при atagirmen/atagdim и ordino/ordinauj при jarastururmen/jarastirdum. Переход от первой формы ко второй сопровождается окончаниями -dim/-dum. В ряду рождения togdum имеет тот же тип окончания. В погодной строке kar yagdi конечного m нет. Это содержательная опора для сопоставления показателя прошедшего и лица с современными казакскими формами. Границы tog-dum и дальнейшее членение -dV-m — наш сопоставительный анализ, а не дефисы древнего писца. Все строки уже были видены, поэтому проверка не называется слепым предсказанием. Три формы на одной странице — не три независимых памятника.

Смысловые поля тоже проверяются: atagdim относится к называнию, jarastirdum — к упорядочению/согласованию. Современное жарастырдым не обязано совпадать со всем диапазоном латинского ordino. Сопоставление смысла требует контекста. Само наличие m не даёт права одинаково переводить все похожие окончания во всех языках.

3. ПОВТОРЯЕМОСТЬ ЗВУКОВОГО СООТВЕТСТВИЯ
В статье François Jacquesson2002 проверена печатная111/PDF20. Таблица сопоставляет четыре смысловых поля,17 строк-обозначений, из которых одна — реконструкции, две — исторические наборы,14 — современные языковые нормы. Таблица сохранена дословно на уровне форм. Это один сравнительный источник; его HTML и PDF не посчитаны двумя независимыми работами. Из неё нельзя считать процент казакского происхождения языков.

Для задних гласных сравниваются tag/taw, bag/baw, tog/tuw; для переднего окружения teg/tij даёт другой исход. Тем самым g/ğ→w рассматривается в определённых условиях и на нескольких основах, а не как свободная замена любой согласной. В таблице Codex отмечены tow-/tog-, но оригинальные рукописные вхождения каждого варианта здесь отдельно не осмотрены. На печатной40 действительно видны варианты и глагольный ряд; точную фонетическую реконструкцию их латинского написания нельзя вывести одним OCR.

В статье Janarbyek Asimkhan2021 (PDF20, нижний колонтитул260) отдельно сопоставляются ранние tog-/tug- и казакское tuw-. Ссылка на Е29 остаётся ссылкой через издание автора, не нашим прочтением стелы. У статьи есть несоответствие пагинации: титульные/библиографические сведения дают261–291, а колонтитулы файла идут241–271. Поэтому зафиксированы PDF-страницы и видимые колонтитулы. Автор анализирует90 специально выбранных общих глаголов; это не процент всего древнего или современного словаря.

4. ЧТО ОСТАЛОСЬ ОТ ШУМЕРСКОГО КАНДИДАТА
Сохранённая древняя учебная модель MS3178 даёт du-u | TU | wa-la-dam и соседние строки с иными подсказками к тому же знаку. Карточка TU у eBL различает графическую единицу ABZ58/U+12305 и несколько значений. Повторно просмотрены обе фотографии MS3178 и уточнённые полосы оборота. Полная поклиновая локализация и независимое чтение строки r ii26 не закончены. Новых шумерских звуков этот блок не установил.

Дополнительно проверена фотография CBS12709(+)CBS12709a/P227701. На расширенном вырезе под горизонтальным разломом повторяются сходные комплексы справа, тогда как левые группы меняются. Это самостоятельное наблюдение графического устройства. Издание относит к этому участку три строки tu-u₂ | TU, du-u₂ | TU, ḫu-du-uš | TU; их звуки и номера строк унаследованы из редакции. Полная опись каждого клина не завершена. Узкие поисковые полосы пересекают соседние строки и не названы точными вырезами строк5–7. На этом предмете нет отдельной аккадской смысловой колонки; значение «рождение» нельзя извлечь только из показанной фотографии. Сохраняются5 вырезов с координатами, хешем родительского изображения и статусами, а также3 наблюдения разных уровней.

Важное уточнение сравнения: обоснованный для степного материала переход g/ğ→w ещё не даёт перехода шумерского d→g. Конечное d в tud нельзя автоматически считать казакским прошедшим -ды. Для этого нужны собственные повторяемые соответствия и независимые контексты. Сходство области рождения остаётся полезным кандидатом, но равенство TU=современное туу для всех употреблений не следует из учебного списка.

5. ПРОВЕРКА ИНСТРУМЕНТА ВЫРЕЗАНИЯ ИЗОБРАЖЕНИЙ
В новом поисковом вырезе MS3178 обнаружено поведение sips: одновременные координаты0,0 дают центральный вырез. На синтетической120×100 сетке каждая точка кодирует координаты цветом. Для выреза20×20 вместо начала0,0 получено50,40. Проверены ещё0,10;10,0;10,10 — они дают правильное начало. Следовательно, первоначальное подозрение обо всех нулевых координатах было слишком широким.

В прежних CSV-реестрах вырезов найдены9 записей с одной нулевой осью; ни в одной нет пары0,0. Оснований отзывать их выводы из-за этого теста не найдено. Старые фотографии и вырезы не заменялись. Исправлен текущий поисковый вырез, добавлен явный инструмент CoreGraphics с проверкой границ и ориентации. Все четыре координатных теста нового инструмента прошли. В генераторах35,36,41 добавлен вызов явного инструмента для нулевых осей. Это исправление исследовательской подготовки, не обнаруженная ошибка древнего текста или переводчика. Размеров и хешей недостаточно для проверки правильности области изображения: добавлен контроль реальных пикселей.

6. ЧТО УЖЕ МОЖНО ИСПОЛЬЗОВАТЬ В КНИГЕ
Можно показать страницу40 и компактную таблицу старых и современных форм. На одном предмете публикации читатель увидит несколько слов и функции окончаний; это сильнее свободного членения одного созвучия. Рабочее положительное заключение: казакский язык сохраняет узнаваемый исторический словесный и грамматический слой степи, который можно прослеживать через письменные памятники. Сохранение не требует буквальной неизменности каждого звука.

Для шумерской части достигнут иной результат: теперь точно назван недоказанный переход между двумя участками цепи. Наличие глины с древними знаками и сходного современного глагола не заполняет его автоматически. Историческое происхождение всеобщего равенства, первенство одного современного языка или внеземная передача разума этим сравнением не установлены. Этическая линия книги может обсуждаться отдельно от этого языкового вывода.

Отдельный литературный фрагмент подготовлен в synthesis/2026-10-26_birth_chain_book_passage.txt. Книга/PDF не редактировались. Следующий предметный контроль должен закрепить чтение шумерской основы и проверить d/g на других независимых лексических рядах; ещё одна подборка удобных созвучий не заменит его.
'''
reportpath='observations/2026-10-26_round43_birth_chain_and_graphic_controls.txt'
(R/reportpath).write_text(report,encoding='utf-8')
passage='''ТУУ: СЛЕД, КОТОРЫЙ МОЖНО ПРОСЛЕДИТЬ
Литературный фрагмент для возможного включения; в рукопись не вставлен

Когда я ищу память языка, мне важно увидеть, как знакомый смысл проходит через разные времена и способы письма. У такого поиска есть особое мгновение: передо мной возникает целый ряд форм, и слово обретает своё окружение.

На странице40 издания Codex Cumanicus, подготовленного Гезой Кууном в1880 году, стоят togarmen, togdum, toggil. Рядом дан вариант tuarmen. Передо мной несколько способов выразить рождение. Я сопоставляю их с родным туу и его формами: туамын, тудым, ту. В этих рядах можно проследить и сохраняющийся смысл, и способы его грамматического оформления.

На той же странице записано kar yagdi. Мне близко звучание казакского қар жауды. Снег остаётся қар, а действие получает форму, в которой узнаётся знакомое окончание. Здесь же встречаются atagdim и jarastirdum. Я сопоставляю их с атадым и жарастырдым. Так отдельное слово входит в более широкий ряд: называние, согласование, рождение, природное событие.

Для меня особенно ценна эта множественность. Память языка проявляется в связи корня, смысла и присоединяемых частей. Переходы звучания тоже обретают порядок, когда рядом оказываются несколько слов и разные формы одного глагола. Я вижу здесь возможность идти от узнавания к последовательному сравнению.

Современный казакский язык сохраняет живую связь с этим словесным пространством степи. Мой опыт носителя помогает услышать её, а письменные записи позволяют проверить услышанное. В такой работе близость родного языка к прошлому становится исследовательским инструментом.

Шумерское сопоставление рождения открывает следующий, более глубокий слой. К нему я подхожу через сами знаки, древние учебные записи и окружение слова. У каждой ступени этого пути должен быть свой материальный след. Именно так я хочу вести словесную археологию: от знакомого звучания к записи, от записи к связям и дальше — к памяти человеческой мысли.
'''
(R/'synthesis/2026-10-26_birth_chain_book_passage.txt').write_text(passage,encoding='utf-8')

def im(n,c):
 p='round43_sources/'+n+'.png';return '<figure><a href="'+p+'"><img loading="lazy" src="'+p+'"></a><figcaption>'+html.escape(c)+'</figcaption></figure>'
page='''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Блок43: историческая цепочка туу</title><style>body{max-width:1150px;margin:35px auto;padding:0 20px;font:18px/1.6 system-ui;background:#f7f3eb;color:#202830}figure{background:white;padding:16px;margin:24px 0}img{max-width:100%;max-height:1150px;object-fit:contain}figcaption{font-size:16px}.result{padding:20px;background:#e4eee2}table{border-collapse:collapse}th,td{border:1px solid #aaa;padding:12px;text-align:left;vertical-align:top}.scroll{overflow:auto}h1,h2{line-height:1.2}</style><h1>Туу: документированный участок исторической цепи</h1><p>Блок43 ·26 октября2026</p><div class="result">На печатной странице40 Codex Cumanicus1880 проверен ряд форм рождения и параллельные глаголы. Сопоставление с казакским теперь опирается на формы слов и грамматические окончания. Переход шумерского tud/dud к tog/tug остаётся неустановленным. Новых независимо прочитанных шумерских звуков нет.</div><h2>Страница, на которой видны сами записи</h2>'''
page+=im('Kuun1880_PDF188','Печатная40/PDF188. Издание1880 года, не фотография средневекового почерка. Формы проверены по скану, латинские sic сохранены.')
page+='<h2>Пять форм рождения</h2><div class="scroll"><table><tr><th>Форма в издании</th><th>Латинская колонка</th><th>Казакское сопоставление</th><th>Статус</th></tr>'
for v in paradigm:page+='<tr>'+''.join('<td>'+html.escape(v[k])+'</td>' for k in ['source_form','Latin_column','Kazakh_comparandum','analysis_status'])+'</tr>'
page+='</table></div><h2>Повторение соответствий на других корнях</h2>'+im('Jacquesson2002_PDF20','Печатная111/PDF20. Четыре поля значений; исторические формы, реконструкции и современные нормы различаются. Не процент происхождения языков.')+im('Asimkhan2021_verbs_PDF20','PDF20, видимый колонтитул260: ранние tog/tug и казакское tuw. Авторская публикация, не самостоятельное чтение нами E29.')
page+='<h2>Проверка непосредственно по фотографии</h2>'+im('CBS12709_extended_context','CBS12709(+CBS12709a): справа повторяются сходные комплексы, слева группы меняются. Это наблюдение графики; чтения tu/du/hu-du-us взяты из издания. Узкие поисковые полосы не изолируют точные строки.')
page+='<h2>Проверка клинописного участка остаётся открытой</h2><p>Древний учебный ряд MS3178 требует полной поклиновой колляции. Связь tud/dud → tog/tug не следует из подтверждённого для степного материала g/ğ → w. Для этой связи нужны другие независимые слова и контексты.</p><h2>Контроль координат</h2><p>Синтетическая сетка выявила центровку sips при0,0. Остальные три испытанных сочетания работают правильно. Девять прежних записей с одной нулевой осью этим дефектом не затронуты. Новый явный инструмент прошёл все четыре теста.</p><h2>Отчёт, данные и фрагмент для книги</h2><ul>'
for p,label in [(reportpath,'Полный отчёт и границы вывода'),('data/round43_CBS12709_crops.csv','Координаты и статусы пяти фотoвырезов'),('data/round43_CBS12709_observations.csv','Что видно и что унаследовано из издания'),('data/round43_birth_paradigm.csv','Пять форм рождения'),('data/round43_inflection_controls.csv','Четыре глагольных контекста'),('data/round43_sound_controls.csv','Условия звуковых соответствий'),('data/round43_regional_comparison.csv','Сравнительная таблица17 строк'),('data/round43_chain_status.csv','Какие участки цепи подтверждены'),('round43_sources/coordinate_regression.json','Восемь координатных проверок'),('data/round43_previous_edge_crop_audit.csv','Аудит прежних вырезов с нулевой осью'),('data/round43_page_map.csv','PDF-страницы и печатная пагинация'),('round43_sources/Kuun1880_CodexCumanicus.pdf','Полный скан исторического издания552 страницы'),('synthesis/2026-10-26_birth_chain_book_passage.txt','Новый литературный фрагмент от первого лица')]:page+='<li><a href="'+p+'">'+html.escape(label)+'</a></li>'
page+='</ul><p>Книга и PDF не изменялись. Скан1880 года помечен Commons как public domain; у современных научных публикаций самостоятельные права.</p></html>'
(R/'comparison_round43.html').write_text(page,encoding='utf-8')
class Links(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,t,a):
  for k,v in a:
   if k in ('href','src') and v and not urlsplit(v).scheme:self.urls.append(unquote(v))
l=Links();l.feed(page)
for p in l.urls:assert (R/p).is_file(),p
assert sha(R.parents[1]/'book/Tartar_i_Ya_print_6x9.pdf')=='ca58ace942ddc4d3a5ea33026026f00c14034c9be294a27e9c725e7780d5f445'
summary=dict(status='passed',cuneiform_photo_contexts_checked=1,cuneiform_photo_crops_registered=5,cuneiform_observation_levels=3,new_external_files=len(urls),historical_edition_PDF_pages=552,birth_forms_checked_in_print=5,finite_contexts_compared=4,regional_table_rows=17,modern_language_labels=14,coordinate_regression_cases=8,strict_crop_cases_passed=4,old_zero_axis_records_audited=9,old_records_with_both_axes_zero=0,prior_historical_images_replaced=0,local_links=len(l.urls),Sumerian_to_tog_bridge_established=False,new_independent_Sumerian_sound_values=0,book_PDF='unchanged')
(R/'data/round43_checkpoint.txt').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n');print(json.dumps(summary,ensure_ascii=False,indent=2))
