#!/usr/bin/env python3
"""Extract an archived CDLI transliteration, without translating it.

This is analysis of editorial text, not recognition of signs in a photograph.
Uses Python standard library only. Run from any directory.
"""
from collections import Counter, defaultdict
import csv
from html.parser import HTMLParser
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]


class TextParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.result = []
        self.italic = 0

    def handle_starttag(self, tag, attrs):
        if tag == 'i':
            self.italic += 1
        if tag == 'br':
            self.result.append('\n')

    def handle_endtag(self, tag):
        if tag == 'i':
            self.italic -= 1

    def handle_data(self, data):
        if not self.italic:
            self.result.append(data)


def save_csv(path, fields, rows):
    with path.open('w', encoding='utf-8-sig', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def extract(artifact):
    if not re.fullmatch(r'P\d{6}', artifact):
        raise ValueError('Expected a CDLI identifier such as P222607.')
    raw = (ROOT / f'sources/{artifact}_card.html').read_text(encoding='utf-8')
    match = re.search(r'<b>object cone\s*</b>(.*?)</div>', raw, re.S)
    if not match:
        raise SystemExit('Expected CDLI text block absent: inspect archived HTML.')
    parser = TextParser()
    parser.feed('<b>object cone</b>' + match.group(1))
    lines = []
    column = None
    for raw_line in ''.join(parser.result).splitlines():
        line = raw_line.strip()
        col = re.fullmatch(r'column (\d+)', line)
        if col:
            column = int(col.group(1))
        else:
            found = re.match(r'^(\d+)\.\s*(?:([a-z])\.\s*)?(.+)$', line)
            if found and column is not None:
                lines.append({'artifact': artifact, 'column': column,
                              'line': found.group(1) + (found.group(2) or ''),
                              'editorial_transliteration': found.group(3).strip()})
    if not lines:
        raise SystemExit('No numbered lines extracted.')
    out = ROOT / 'data'
    out.mkdir(exist_ok=True)
    save_csv(out / f'{artifact}_editorial_lines.csv', list(lines[0]), lines)
    locations = defaultdict(list)
    tokens = Counter()
    for row in lines:
        text = row['editorial_transliteration']
        locations[text].append(f"{row['column']}:{row['line']}")
        tokens.update(text.split())
    repeats = [{'sequence': text, 'count': len(refs), 'column_line': '; '.join(refs)}
               for text, refs in sorted(locations.items(), key=lambda x: (-len(x[1]), x[0]))
               if len(refs) > 1]
    save_csv(out / f'{artifact}_repeated_lines.csv', ['sequence', 'count', 'column_line'], repeats)
    save_csv(out / f'{artifact}_editorial_tokens.csv', ['token', 'count'],
             [{'token': token, 'count': count} for token, count in tokens.most_common()])
    targets = ('inim', 'URU-KA', 'ama-gi', 'mu-na-du3')
    selected = [row for row in lines if any(t in row['editorial_transliteration'] for t in targets)]
    save_csv(out / f'{artifact}_target_lines.csv', list(lines[0]), selected)
    print(f'{artifact}: extracted {len(lines)} lines in {len(set(r["column"] for r in lines))} columns.')
    print(f'{len(repeats)} repeated full-line sequences; {len(selected)} target lines.')
    for item in repeats[:6]:
        print(f'{item["count"]} × {item["sequence"]}: {item["column_line"]}')


if __name__ == '__main__':
    for artifact in sys.argv[1:] or ['P222607']:
        extract(artifact)
