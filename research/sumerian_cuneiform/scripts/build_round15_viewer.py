#!/usr/bin/env python3
"""Create a local viewer of checked results, photographs and the ancient anchor."""
import csv
import html
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]

def main():
    data=ROOT/'data'
    result=json.loads((data/'round15_arithmetic_results.json').read_text())
    parts=['''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Цикл15 — перенос модели и древний силлабарий</title><style>
body{font:17px/1.65 system-ui,sans-serif;max-width:1120px;margin:auto;padding:28px 22px;background:#f4f1e9;color:#243944}h1,h2{line-height:1.2}a{color:#145e73}.note{padding:15px 18px;background:#e1ebeb}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:16px}figure{background:white;margin:16px 0;padding:14px}img{max-width:100%;height:auto;display:block;margin:auto}figcaption{font-size:14px;color:#4b626c}table{border-collapse:collapse;width:100%;background:white;font-size:15px}td,th{padding:10px;border:1px solid #c7d1d0;text-align:left;vertical-align:top}.scroll{overflow:auto;max-height:480px}pre{white-space:pre-wrap;background:white;padding:16px}.formula{font-size:27px;font-weight:650}summary{cursor:pointer;font-weight:600}</style>
<h1>Перенос модели на новые таблички</h1><p>25 октября2026 · Комплексный цикл15/15b · Точные дроби, повреждения и древняя графическая опора</p><p><a href="observations/2026-10-25_round15_transfer_and_ancient_anchor.txt">Полный отчёт</a> · <a href="data/round15_arithmetic_results.json">Расчёты и предпосылки</a> · <a href="data/round15_protocol.json">Протокол</a></p>
<p class="note">Веса перенесены без изменения под новые суммы. Редакторские классы знаков и их веса остаются проверяемой моделью. Успех арифметики не заменяет самостоятельного чтения каждого клина.</p>
<table><tr><th>Предмет</th><th>Записей</th><th>Сумма / итог</th><th>Остаток</th><th>Неопределённых или восстановленных префиксов</th></tr>''']
    for x in result['results']:
        values=[x['artifact'],x['quantity_entries'],x['entry_sum_N01']+' / '+x['total_N01'],x['residual_N01'],x['uncertain_or_restored_entries']]
        parts.append('<tr>'+''.join('<td>'+html.escape(str(v))+'</td>' for v in values)+'</tr>')
    parts.append('''</table><h2>Более определённый тест: P005372</h2><p class="formula">1171⅕ = 1171⅕</p><p>Восемь опубликованных количественных префиксов не имеют редакторских флагов повреждения или восстановления. Это не означает независимую проверку нами каждого следа. Вычисления выполнены точными дробями.</p><div class="grid">''')
    for side in ['front','back']:
        file=f'data/round15_P005372_{side}_native.jpg'
        parts.append(f'<figure><a href="sources/P005372_photo.jpg"><img src="{file}" alt="Нативный кадр P005372 {side}"></a><figcaption>P005372 · {side} · исходный масштаб; без дорисовки.</figcaption></figure>')
    parts.append('''</div><h2>Дополнительное условное ограничение</h2><pre>2D − 4A − 19B − 5C − 5F = 0
D = 3A        — внешняя числовая схема
A = 10B       — условный результат предыдущего предмета
F = C/5       — внешняя дробная схема

Следует B = 6C: вес(N14) = 6·вес(N01).
При B = 10C остаток был бы 4C.</pre><p>Предположение об итоговой записи и отношения остальных классов названы явно. Это не восстановление всей системы без предварительных сведений.</p><h2>Повреждённый контроль: P005371</h2><p>Итог7162/5=1432⅖ совпал с суммой названных величин. Но8 префиксов имеют неопределённость, включая2 восстановления и1 пропуск. Незнакомый промежуток не объявлен пустым для удобного совпадения.</p><div class="grid">''')
    for side in ['front','back']:
        file=f'data/round15_P005371_{side}_native.jpg'
        parts.append(f'<figure><a href="sources/P005371_photo.jpg"><img src="{file}" alt="Нативный кадр P005371 {side}"></a><figcaption>P005371 · {side} · повреждения сохранены.</figcaption></figure>')
    parts.append('''</div><h2>Все количественные записи</h2><div class="scroll"><table><tr><th>Предмет / координата</th><th>Предлагаемые классы</th><th>Значение в модели</th><th>Восстановление / неопределённость / пропуск</th></tr>''')
    with (data/'round15_quantity_entries.csv').open(encoding='utf-8-sig',newline='') as f:
        for row in csv.DictReader(f):
            vals=[row['artifact']+' '+row['coordinate'],row['proposed_counts'],row['value_in_N01_units'],row['restored']+' / '+row['uncertain']+' / '+row['gap']]
            parts.append('<tr>'+''.join('<td>'+html.escape(str(v))+'</td>' for v in vals)+'</tr>')
    parts.append('''</table></div><h2>Древний носитель для проверки звучания</h2><p>OIM A02480 / P391514 — Чикагский силлабарий. Маленькое фото490×612 в OIMP32 оказалось недостаточным. В AS7 найдены фотографические таблицыI–II; прорисовкаV сохранена отдельно. Каталожная дата первого тысячелетия до н.э. не переносится автоматически на ранние урукские записи.</p><figure><a href="data/round15b_AS7_photoplates_native-000.png"><img src="data/round15b_CS_right_upper_native.png" width="1030" height="1500" alt="Нативный кадр подробной фоторепродукции древнего списка"></a><figcaption>ТаблицаI, PDF96 в AS7. Наблюдаются параллельные полосы и повторяющиеся комплексы. Кадр1140,340–2170,1840 из встроенной сканированной страницы. Звуки не назначены.</figcaption></figure><p><a href="data/round15b_AS7_photoplates_native-000.png">Фотографическая таблицаI</a> · <a href="data/round15b_AS7_photoplates_native-001.png">Фотографическая таблицаII</a> · <a href="data/round15b_AS7_page100_preview.png">Отдельно: современная прорисовкаV</a> · <a href="data/round15b_photo_observations.json">Наблюдения без назначенных звуков</a></p><p>Роли столбцов, предложенные каталогом, сохранены отдельно от видимой организации. Наличие древнего списка даёт материал для проверки, но ещё не восстанавливает звуки автоматически.</p><h2>Файлы</h2><ul><li><a href="data/round15_editorial_cells.csv">Все29 редакторских ячеек</a></li><li><a href="data/round15_quantity_entries.csv">21 количественная запись</a></li><li><a href="data/round15_source_identity.json">Идентификаторы предметов</a></li><li><a href="data/round15b_photo_manifest.json">Происхождение фототаблиц и кадров</a></li><li><a href="data/round15b_official_pdf_url.json">Официальная ссылка OIMP32</a></li><li><a href="data/round15b_AS7_pdf_url.json">Официальная ссылка AS7</a></li></ul><details><summary>Развернуть полный текст отчёта</summary><pre>''')
    parts.append(html.escape((ROOT/'observations/2026-10-25_round15_transfer_and_ancient_anchor.txt').read_text()))
    parts.append('''</pre></details><p>Книга не менялась. Алфавит, независимые значения ama/adda и направлениеK0→шумерский пока не восстановлены.</p></html>''')
    (ROOT/'comparison_round15.html').write_text(''.join(parts))
    print('Integrated viewer saved: quantities,4 native account crops,ancient photoplates and full report.')

if __name__=='__main__':main()
