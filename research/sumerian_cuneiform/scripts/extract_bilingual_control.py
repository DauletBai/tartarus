#!/usr/bin/env python3
"""Extract the saved SEAL witness, preserving editorial damage markers."""
import csv
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class WitnessTable(HTMLParser):
    def __init__(self):
        super().__init__()
        self.active = False
        self.cell = None
        self.cells = []
        self.rows = []

    def handle_starttag(self, tag, attrs):
        if tag == 'table' and dict(attrs).get('id') == '27667':
            self.active = True
        if not self.active:
            return
        if tag == 'tr':
            self.cells = []
        if tag == 'td':
            self.cell = []
        if self.cell is not None and tag == 'sub':
            self.cell.append('_')
        if self.cell is not None and tag == 'sup':
            self.cell.append('{')

    def handle_endtag(self, tag):
        if not self.active:
            return
        if tag == 'sup' and self.cell is not None:
            self.cell.append('}')
        if tag == 'td' and self.cell is not None:
            self.cells.append(' '.join(''.join(self.cell).split()))
            self.cell = None
        if tag == 'tr':
            self.rows.append(self.cells)
        if tag == 'table':
            self.active = False

    def handle_data(self, value):
        if self.active and self.cell is not None:
            self.cell.append(value)


def main():
    parser = WitnessTable()
    parser.feed((ROOT / 'sources/SEAL_27667_CBS346.html').read_text())
    surface = line = ''
    records = []
    for cells in parser.rows:
        if len(cells) == 1:
            surface = cells[0]
        elif len(cells) == 2:
            label, text = cells
            if label:
                line = label
            records.append(dict(witness='CBS346 / P257792', edition='SEAL27667',
                                surface_as_edition=surface, line=line,
                                row_role='numbered_Sumerian_or_mixed' if label else 'unnumbered_Akkadian',
                                editorial_text=text))
    assert len([r for r in records if r['row_role'].startswith('numbered')]) == 14
    assert any('inim-zu' in r['editorial_text'] for r in records)
    assert any('a-wa-at-ka' in r['editorial_text'] for r in records)
    with (ROOT / 'data/round5_CBS346_edition_rows.csv').open('w', encoding='utf-8', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=list(records[0]))
        writer.writeheader(); writer.writerows(records)
    print('Extracted', len(records), 'edition rows; 14 numbered and',
          len(records)-14, 'interlinear rows')


if __name__ == '__main__':
    main()
