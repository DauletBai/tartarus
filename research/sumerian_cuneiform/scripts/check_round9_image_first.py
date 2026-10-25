#!/usr/bin/env python3
"""Reproduce availability and post-freeze comparison; no automated decipherment."""
import csv
import hashlib
import json
from pathlib import Path
import re
from extract_early_and_uriii import cdli

ROOT = Path(__file__).resolve().parents[1]


def main():
    freeze = json.loads((ROOT / 'data/round9b_visual_freeze.json').read_text())
    observation = ROOT / 'data/round9b_visual_observations_before_text.json'
    photo = ROOT / 'sources/P342808_photo.jpg'
    assert hashlib.sha256(observation.read_bytes()).hexdigest() == freeze['visual_record_sha256']
    assert hashlib.sha256(photo.read_bytes()).hexdigest() == freeze['photo_sha256']
    cohorts = [('round9', ['P226422', 'P226524', 'P226526']),
               ('round9b', ['P226887', 'P226912', 'P342808', 'P499399'])]
    readers = {'P226887':'P226887_reader.html', 'P342808':'P342808_photo_reader.html',
               'P499399':'P499399_reader.html'}
    rows = []
    for cohort, artifacts in cohorts:
        for artifact in artifacts:
            raw = (ROOT / 'sources' / f'{artifact}_card.html').read_text()
            links = list(dict.fromkeys(re.findall(r'href="([^"]*/reader/[^\"]+)"', raw)))
            urls = []
            if artifact in readers:
                source = (ROOT / 'sources' / readers[artifact]).read_text()
                urls = [u.replace('\\/', '/') for u in re.findall(r'url:\s*"([^"]+)"', source)]
            kind = ('photograph' if any('/photo/' in u for u in urls) else
                    'drawing_only' if any('/lineart/' in u for u in urls) else 'no_image_in_card')
            rows.append(dict(cohort=cohort, artifact=artifact, image_type=kind,
                             reader_count=len(links), individual_text_exposed=artifact=='P342808',
                             selected=artifact=='P342808',
                             visual_outcome='unresolvable' if artifact=='P342808' else
                             'not_inspected' if kind!='no_image_in_card' else 'unavailable'))
    eligible = [r['artifact'] for r in rows if r['cohort']=='round9b' and r['image_type']=='photograph']
    assert eligible[0] == 'P342808'
    with (ROOT / 'data/round9_availability_and_outcomes.csv').open('w', encoding='utf-8', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0])); writer.writeheader(); writer.writerows(rows)
    edited = cdli('P342808')
    comparison = []
    for row in edited:
        if row['editorial_text'] in {'nin-a-ni', 'e2-a-ni'}:
            comparison.append(dict(artifact='P342808', column=row['column'], line=row['line'],
                                   editorial_claim=row['editorial_text'],
                                   visual_result_before_exposure='unresolvable',
                                   agreement_status='not_tested_at_sign_level',
                                   inferred_sound='', inferred_meaning=''))
    assert len(comparison)==2
    with (ROOT / 'data/round9_post_freeze_comparison.csv').open('w', encoding='utf-8', newline='') as out:
        writer=csv.DictWriter(out,fieldnames=list(comparison[0])); writer.writeheader(); writer.writerows(comparison)
    print('7 availability records; 1 selected photo; 2 editorial target claims remain visually unresolved')


if __name__=='__main__': main()
