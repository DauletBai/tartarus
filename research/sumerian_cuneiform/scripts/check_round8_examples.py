#!/usr/bin/env python3
"""Audit a predeclared grammar section and its first two first-person witnesses."""
import collections
import csv
import hashlib
import json
from pathlib import Path
import re
import subprocess
from extract_early_and_uriii import Plain

ROOT = Path(__file__).resolve().parents[1]


def save(name, rows):
    with (ROOT / 'data' / name).open('w', encoding='utf-8', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)


def parentheses(text):
    start = None
    depth = 0
    for i, char in enumerate(text):
        if char == '(':
            if depth == 0: start = i+1
            depth += 1
        elif char == ')' and depth:
            depth -= 1
            if depth == 0:
                yield ' '.join(text[start:i].split())


SIGLA = re.compile(r'\b(?:Cyl|St|TCS|NG|DP|SRU|ASJ|TSDU|MVN|FAOS|En\.|Gutium|Ent\.|TCTI|OSP|TCL|TRU|PDT|AUCT|NATN|UET|JCS|TMHC|YOS|VS|DI P|Inanna|Lugal-e|AfO)\b')


def witness(artifact):
    raw = (ROOT / 'sources' / f'{artifact}_card.html').read_text()
    block = re.search(r'<div class="translate-indent[^\"]*">(.*?)</div>', raw, re.S)
    assert block
    parser = Plain(); parser.feed(block[1])
    obj = surface = column = ''
    occurrences = collections.Counter()
    rows = []
    for line in ''.join(parser.parts).splitlines():
        line = line.strip()
        if line.startswith('object '):
            obj = line[7:]; surface = column = ''
        elif line in {'obverse', 'reverse'} or line.startswith('surface '):
            surface = line; column = ''
        elif line.startswith('column '): column = line[7:]
        else:
            match = re.fullmatch(r"(\d+[a-z]?'?)\.\s*(.+)", line)
            if match:
                key = (obj, surface, column, match[1]); occurrences[key] += 1
                rows.append(dict(artifact=artifact, object=obj, surface=surface,
                                 column=column, line_as_source=match[1],
                                 occurrence_at_coordinate=occurrences[key],
                                 editorial_text=match[2]))
    assert len({(r['object'], r['surface'], r['column'], r['line_as_source'],
                 r['occurrence_at_coordinate']) for r in rows}) == len(rows)
    save(f'round8_{artifact}_all_rows.csv', rows)
    return rows, [dict(artifact=artifact, object=k[0], surface=k[1], column=k[2],
                       line_as_source=k[3], records=v) for k,v in occurrences.items() if v>1]


def main():
    pdf = ROOT / 'sources/Jagersma_2010_mirror.pdf'
    text = subprocess.run(['pdftotext', '-layout', str(pdf), '-'],
                          check=True, capture_output=True, text=True).stdout
    pages = text.split('\f')
    section = '\n'.join(pages[238:246])
    section = section.split('8.3.1. General remarks', 1)[1].split('8.4. Demonstratives', 1)[0]
    matches = list(re.finditer(r'^\((\d+)\)\s+(.+)$', section, re.M))
    examples = []
    for i, match in enumerate(matches):
        block = section[match.end():matches[i+1].start() if i+1<len(matches) else len(section)]
        citations = [x for x in parentheses(block) if SIGLA.search(x)]
        examples.append(dict(example=int(match[1]), ancient_form_first_line=match[2].strip(),
                             first_text_reference=citations[0] if citations else '',
                             other_references_in_following_discussion=json.dumps(citations[1:], ensure_ascii=False)))
    assert [r['example'] for r in examples] == list(range(28,67))
    references = [dict(sequence=i+1, raw_reference=x) for i,x in enumerate(parentheses(section)) if SIGLA.search(x)]
    save('round8_grammar_numbered_examples.csv', examples)
    save('round8_grammar_reference_candidates.csv', references)
    all_rows, warnings = [], []
    for artifact in ('P232300', 'P232275'):
        rows, issues = witness(artifact); all_rows.extend(rows); warnings.extend(issues)
    save('round8_coordinate_warnings.csv', warnings)
    targets = [r for r in all_rows if re.search(r'\b(?:lugal|iri|uru|igi|e2)-(?:mu|ga2)\b', r['editorial_text'])]
    save('round8_first_person_candidate_rows.csv', targets)
    summary = dict(grammar_sha256=hashlib.sha256(pdf.read_bytes()).hexdigest(),
                   numbered_examples=len(examples),
                   parenthetical_reference_candidates=len(references),
                   witnesses={a:sum(r['artifact']==a for r in all_rows) for a in ('P232300','P232275')},
                   coordinate_warnings=warnings, candidate_rows=len(targets),
                   limits=['Grammar-selected examples, not a random corpus sample',
                           'Candidate search is exploratory and includes names/other functions',
                           'Photographs and editorial readings do not establish pronunciation independently'])
    (ROOT / 'data/round8_summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(summary, ensure_ascii=False))
    print('Source-order examples30/32:', [(r['example'],r['first_text_reference']) for r in examples if r['example'] in {30,32}])
    print('StatueB same-base controls:', [(r['column'],r['line_as_source'],r['editorial_text']) for r in targets if r['artifact']=='P232275' and re.search(r'\b(?:iri|lugal)-(?:mu|ga2)',r['editorial_text'])])


if __name__ == '__main__': main()
