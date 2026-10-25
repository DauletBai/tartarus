#!/usr/bin/env python3
"""Descriptive census of editorial forms; not an independent decipherment."""
import collections
import csv
import hashlib
import json
from pathlib import Path
import re
import zipfile

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / 'sources/ETCSRI_2026-10-24_json.zip'
EXPECTED = '66991601df193a5cb93bccf92e9051e86a98fbb668a9dd343477d14f0c9d54d6'


def nodes(obj):
    if isinstance(obj, dict):
        if obj.get('node') == 'l' and isinstance(obj.get('f'), dict):
            yield obj
        for value in obj.values():
            yield from nodes(value)
    elif isinstance(obj, list):
        for value in obj:
            yield from nodes(value)


def main():
    digest = hashlib.sha256(ARCHIVE.read_bytes()).hexdigest()
    assert digest == EXPECTED, 'Source snapshot changed'
    counts = collections.Counter()
    targets = []
    seen = set()
    texts = set()
    parse_failures = []
    with zipfile.ZipFile(ARCHIVE) as archive:
        catalogue_bytes = archive.read('etcsri/catalogue.json')
        catalogue = json.loads(catalogue_bytes)['members']
        files = sorted(n for n in archive.namelist()
                       if re.search(r'/corpusjson/Q\d+\.json$', n))
        for filename in files:
            qid = Path(filename).stem
            raw = archive.read(filename)
            try:
                document = json.loads(raw)
            except (ValueError, UnicodeDecodeError) as error:
                parse_failures.append(dict(file=filename, bytes=len(raw), error=str(error)))
                continue
            for node in nodes(document):
                f = node['f']
                if not f.get('lang', '').startswith('sux'):
                    counts['excluded_non_sumerian_or_missing_language'] += 1
                    continue
                key = (qid, node.get('ref', ''), f.get('form', ''))
                if key in seen:
                    counts['duplicate_nodes_excluded'] += 1
                    continue
                seen.add(key)
                texts.add(qid)
                counts['sumerian_word_occurrences'] += 1
                if f.get('morph'):
                    counts['occurrences_with_morphology'] += 1
                visible = f.get('form', '').endswith('-a-ni')
                model = bool(re.search(r'(?:^|\.)N3=ani(?:\.|$)', f.get('morph', '')))
                category = ('both' if visible and model else
                            'visible_only' if visible else
                            'model_only' if model else 'neither')
                counts[category] += 1
                if visible or model:
                    metadata = catalogue.get(qid, {})
                    targets.append(dict(qid=qid, ref=node.get('ref', ''),
                                        category=category,
                                        form=f.get('form', ''), cf=f.get('cf', ''),
                                        gw=f.get('gw', ''), pos=f.get('pos', ''),
                                        morph=f.get('morph', ''),
                                        morph2=f.get('morph2', ''),
                                        norm=f.get('norm', ''),
                                        period=metadata.get('period', ''),
                                        designation=metadata.get('designation', '')))
    out = ROOT / 'data/round3b_possessive_occurrences.csv'
    with out.open('w', encoding='utf-8', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(targets[0]))
        writer.writeheader()
        writer.writerows(targets)
    category_details = {}
    for category in ('both', 'visible_only', 'model_only'):
        rows = [r for r in targets if r['category'] == category]
        bases = collections.Counter(r['cf'] for r in rows if r['cf'])
        category_details[category] = dict(
            occurrences=len(rows), compositions=len({r['qid'] for r in rows}),
            distinct_editorial_lexical_bases=len(bases),
            top_bases=bases.most_common(15), examples=rows[:8])
    assert sum(counts[c] for c in ('both', 'visible_only', 'model_only', 'neither')) == counts['sumerian_word_occurrences']
    summary = dict(snapshot_sha256=digest,
                   catalogue_sha256=hashlib.sha256(catalogue_bytes).hexdigest(),
                   corpus_json_files=len(files), catalogue_members=len(catalogue),
                   compositions_with_sumerian_nodes=len(texts),
                   parse_failures=parse_failures,
                   counts=dict(counts), categories=category_details,
                   unit='Unique editorial word node (Q id, ref, form); composites are not physical objects',
                   limitations=['Editorial morphological labels are hypotheses, not independent validation',
                                'Exact suffix test deliberately misses spelling variants and following suffixes',
                                'No ancestry, population or language-age percentages inferred'])
    (ROOT / 'data/round3b_summary.json').write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: summary[k] for k in ['corpus_json_files', 'catalogue_members',
                                            'compositions_with_sumerian_nodes', 'counts']}, ensure_ascii=False))
    for category, detail in category_details.items():
        print(category, {k: detail[k] for k in ['compositions', 'distinct_editorial_lexical_bases', 'top_bases']})
        print('examples', [(r['ref'], r['form'], r['morph']) for r in detail['examples'][:3]])


if __name__ == '__main__':
    main()
