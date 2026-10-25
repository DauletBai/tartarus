#!/usr/bin/env python3
"""Extract catalogue sign correspondences, without pretending to collate a tablet."""
import csv
from pathlib import Path
import re
from extract_lexical_control import plain

ROOT = Path(__file__).resolve().parents[1]
specs = [('OSL_am_homophones.html', 'am'), ('OSL_ma_homophones.html', 'ma'),
         ('OSL_an_homophones.html', 'an'), ('OSL_ku_homophones.html', 'ku'),
         ('OSL_gu_homophones.html', 'gu₁₀')]
rows = []
for filename, target in specs:
    raw = (ROOT / 'sources' / filename).read_text()
    selected = []
    for fragment in re.findall(r'<tr\b[^>]*>(.*?)</tr>', raw, re.S):
        cells = re.findall(r'<td\b[^>]*>(.*?)</td>', fragment, re.S)
        if len(cells) == 4 and plain(cells[0]) == target:
            selected.append(dict(cited_value=target, sign_name=plain(cells[1]),
                                 unicode_reference=plain(cells[2]),
                                 other_catalogue_values=plain(cells[3]), source_file=filename,
                                 status='Catalogue mapping; physical sign not independently collated'))
    assert len(selected) == 1, (filename, len(selected))
    rows.extend(selected)
mu = next(r for r in rows if r['cited_value'] == 'gu₁₀')
assert mu['sign_name'] == 'MU' and 'mu' in mu['other_catalogue_values'].split()
with (ROOT / 'data/round7_sign_value_control.csv').open('w', encoding='utf-8', newline='') as out:
    writer = csv.DictWriter(out, fieldnames=list(rows[0])); writer.writeheader(); writer.writerows(rows)
for row in rows:
    print(row['cited_value'], row['sign_name'], row['unicode_reference'])
