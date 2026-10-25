#!/usr/bin/env python3
"""Extract N3 annotations and retain ambiguous/non-possessive analyses."""
import collections
import csv
import hashlib
import json
import re
import zipfile
from census_possessives import ROOT, ARCHIVE, EXPECTED, nodes
from extract_early_and_uriii import cdli


def main():
    assert hashlib.sha256(ARCHIVE.read_bytes()).hexdigest() == EXPECTED
    rows, seen = [], set()
    with zipfile.ZipFile(ARCHIVE) as archive:
        for filename in sorted(archive.namelist()):
            if not re.search(r'/corpusjson/Q\d+\.json$', filename):
                continue
            raw = archive.read(filename)
            if not raw:
                continue
            for node in nodes(json.loads(raw)):
                f = node['f']
                key = (filename, node.get('ref', ''), f.get('form', ''))
                if not f.get('lang', '').startswith('sux') or key in seen:
                    continue
                seen.add(key)
                nominal = [p[3:] for p in f.get('morph', '').split('.') if p.startswith('N3=')]
                if not nominal:
                    continue
                glosses = [p[3:] for p in f.get('morph2', '').split('.') if p.startswith('N3=')]
                rows.append(dict(qid=filename.rsplit('/', 1)[-1][:-5],
                                 ref=node.get('ref', ''), form=f.get('form', ''),
                                 cf=f.get('cf', ''), N3='|'.join(nominal),
                                 N3_gloss='|'.join(glosses),
                                 morph=f.get('morph', ''), morph2=f.get('morph2', '')))
    path = ROOT / 'data/round4_N3_annotations.csv'
    with path.open('w', encoding='utf-8', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)
    summary = []
    for value in sorted({r['N3'] for r in rows}):
        group = [r for r in rows if r['N3'] == value]
        summary.append(dict(N3=value, occurrences=len(group),
                            compositions=len({r['qid'] for r in group}),
                            gloss_counts=dict(collections.Counter(r['N3_gloss'] for r in group)),
                            examples=group[:3]))
    (ROOT / 'data/round4_N3_summary.json').write_text(
        json.dumps(summary, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    for s in summary:
        print(s['N3'], s['occurrences'], s['gloss_counts'])
    for artifact in ('P226465', 'P226487', 'P226725'):
        lines = cdli(artifact)
        print(artifact, len(lines), 'editorial records; targets',
              [(r['object'], r['surface'], r['column'], r['line'], r['editorial_text'])
               for r in lines if '-a-ni' in r['editorial_text']])


if __name__ == '__main__':
    main()
