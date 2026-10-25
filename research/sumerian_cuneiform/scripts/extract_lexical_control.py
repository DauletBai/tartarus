#!/usr/bin/env python3
"""Preserve the two columns of the saved YBC9868 witness edition."""
import csv
import re
from html.parser import HTMLParser
from pathlib import Path
from extract_early_and_uriii import cdli

ROOT = Path(__file__).resolve().parents[1]


class Cell(HTMLParser):
    def __init__(self):
        super().__init__(); self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag == 'sup': self.parts.append('{')
        if tag == 'sub': self.parts.append('_')

    def handle_endtag(self, tag):
        if tag == 'sup': self.parts.append('}')

    def handle_data(self, value):
        self.parts.append(value)


def plain(raw):
    p = Cell(); p.feed(raw)
    return ' '.join(''.join(p.parts).split())


def save(name, rows):
    with (ROOT / 'data' / name).open('w', encoding='utf-8', newline='') as out:
        w = csv.DictWriter(out, fieldnames=list(rows[0]))
        w.writeheader(); w.writerows(rows)


def main():
    raw = (ROOT / 'sources/P310404_ORACC.html').read_text()
    rows = []
    for identity, fragment in re.findall(r'<tr id="(P310404\.\d+)" class="l">(.*?)</tr>', raw, re.S):
        cells = re.findall(r'<td\b[^>]*>(.*?)</td>', fragment, re.S)
        if len(cells) < 2:
            continue
        coordinate = re.search(r'<span class="xlabel [^"]*">(.*?)</span>', cells[0], re.S)
        rows.append(dict(witness='YBC09868 / P310404', row_id=identity,
                         coordinate=plain(coordinate[1]) if coordinate else plain(cells[0]),
                         sumerian_editorial=plain(cells[1]),
                         akkadian_editorial=' | '.join(plain(c) for c in cells[2:])))
    assert len({r['row_id'] for r in rows}) == len(rows)
    targets = [r for r in rows if re.match(r'^[⸢\[\s]*inim\b', r['sumerian_editorial'])]
    assert any(r['coordinate'] == 'ii 42' and r['akkadian_editorial'] == 'a-wa-tum' for r in targets)
    save('round6_YBC9868_ORACC_rows.csv', rows)
    save('round6_YBC9868_inim_targets.csv', targets)
    card_rows = cdli('P310404')
    roman = {'1': 'i', '2': 'ii', '3': 'iii', '4': 'iv', '5': 'v', '6': 'vi'}
    card_index = {roman.get(r['column'], r['column'])+' '+r['line']: r for r in card_rows}
    oracc_index = {r['coordinate']: r for r in rows}
    coverage = []
    for key in sorted(set(card_index) | set(oracc_index)):
        coverage.append(dict(coordinate=key,
                             present_in_cdli=int(key in card_index),
                             present_in_oracc=int(key in oracc_index),
                             cdli_text=card_index.get(key, {}).get('editorial_text', ''),
                             oracc_row=oracc_index.get(key, {}).get('row_id', ''),
                             oracc_sumerian=oracc_index.get(key, {}).get('sumerian_editorial', ''),
                             oracc_akkadian=oracc_index.get(key, {}).get('akkadian_editorial', '')))
    save('round6_YBC9868_coordinate_coverage.csv', coverage)
    print('ORACC edition rows:', len(rows), '; inim-initial:', len(targets), '; CDLI rows:', len(card_rows))
    print('Coordinate overlap only:', sum(r['present_in_cdli'] and r['present_in_oracc'] for r in coverage),
          '; CDLI-only:', sum(r['present_in_cdli'] and not r['present_in_oracc'] for r in coverage),
          '; ORACC-only:', sum(r['present_in_oracc'] and not r['present_in_cdli'] for r in coverage))
    for r in targets:
        if r['coordinate'] in {'ii 42', 'ii 49', 'iii 8', 'iii 9', 'iii 10'}:
            print(r['coordinate'], r['sumerian_editorial'], '=', r['akkadian_editorial'])


if __name__ == '__main__':
    main()
