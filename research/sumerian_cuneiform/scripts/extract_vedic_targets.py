#!/usr/bin/env python3
"""Extract selected Sanskrit verse coordinates, without assigning new meanings."""
import csv
import re
from html import unescape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {1: {32}, 10: {90, 129, 191}}
rows = []
for mandala, hymns in TARGETS.items():
    source = ROOT / 'sources' / f'RV_mandala{mandala:02d}.html'
    text = unescape(re.sub(r'<[^>]+>', '\n', source.read_text(encoding='utf-8')))
    for match in re.finditer(r'RV_(\d+),(\d+)\.(\d+)([a-z])\s+([^\n]+)', text):
        m, h, v, half, words = match.groups()
        if int(m) == mandala and int(h) in hymns:
            rows.append([f'RV_{int(m)},{int(h):03d}.{int(v):02d}{half}', words.strip(), source.name])
expected = 2 * (15 + 16 + 7 + 4)
assert len(rows) == expected, (len(rows), expected)
assert len({row[0] for row in rows}) == expected
with (ROOT / 'data' / 'vedic_target_verses.csv').open('w', encoding='utf-8-sig', newline='') as out:
    writer = csv.writer(out)
    writer.writerow(['coordinate', 'sanskrit_editorial_text', 'local_source'])
    writer.writerows(rows)
print(f'Extracted {len(rows)} half-verses from four hymns; no translation or etymology inferred.')
