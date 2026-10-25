#!/usr/bin/env python3
"""Compare editorial spellings without consulting dictionary translations.

Shared textual composition is not independent evidence for language genealogy.
SequenceMatcher aligns strings, not physical signs or historical morphemes.
"""
import csv
from collections import defaultdict
from difflib import SequenceMatcher
from pathlib import Path
from extract_cdli import save_csv

ROOT = Path(__file__).resolve().parents[1]


def read(artifact):
    with (ROOT / f'data/{artifact}_editorial_lines.csv').open(encoding='utf-8-sig', newline='') as handle:
        return list(csv.DictReader(handle))


def location(row):
    return f"{row['column']}:{row['line']}"


def main():
    left, right = read('P222607'), read('P222608')
    lefttext = [r['editorial_transliteration'] for r in left]
    righttext = [r['editorial_transliteration'] for r in right]
    matcher = SequenceMatcher(None, lefttext, righttext, autojunk=False)
    alignment = []
    exact = 0
    for tag, i, j, k, l in matcher.get_opcodes():
        if tag == 'equal':
            exact += j - i
        alignment.append({'kind': tag,
                          'P222607_locations': '; '.join(map(location, left[i:j])),
                          'P222607_editorial_text': ' | '.join(lefttext[i:j]),
                          'P222608_locations': '; '.join(map(location, right[k:l])),
                          'P222608_editorial_text': ' | '.join(righttext[k:l])})
    save_csv(ROOT / 'data/witness_alignment.csv', list(alignment[0]), alignment)
    families = defaultdict(list)
    for row in left + right:
        for token in row['editorial_transliteration'].split():
            if token.startswith('URU-KA-gi-na'):
                families[('NAME', token)].append(f"{row['artifact']} {location(row)}")
            elif token == 'inim' or token.startswith('inim-'):
                families[('INIM', token)].append(f"{row['artifact']} {location(row)}")
            elif token == 'ama-gi4' or token.startswith('ama-gi4-'):
                families[('AMA_GI', token)].append(f"{row['artifact']} {location(row)}")
    variants = [{'family': family, 'editorial_form': form,
                 'count': len(refs), 'locations': '; '.join(refs)}
                for (family, form), refs in sorted(families.items())]
    save_csv(ROOT / 'data/structural_variants.csv', list(variants[0]), variants)
    print(f'Editorial lines: {len(left)} / {len(right)}; exact aligned lines: {exact}.')
    print('This is string alignment of two related witnesses, not a genetic similarity score.')
    for row in variants:
        print(f"{row['family']}: {row['editorial_form']} × {row['count']}: {row['locations']}")


if __name__ == '__main__':
    main()
