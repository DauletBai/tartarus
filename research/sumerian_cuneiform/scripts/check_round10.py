#!/usr/bin/env python3
"""Check saved image-first records and extract the two individual editions.

No network, no automatic sign recognition, no independent phonetic decipherment.
"""
import csv
import hashlib
import json
from pathlib import Path
from extract_early_and_uriii import cdli

ROOT = Path(__file__).resolve().parents[1]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def write(name, rows):
    with (ROOT / 'data' / name).open('w', encoding='utf-8-sig', newline='') as out:
        w = csv.DictWriter(out, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)

def main():
    freeze = json.loads((ROOT / 'data/round10_visual_freeze.json').read_text())
    assert digest(ROOT / 'data/round10_protocol.json') == freeze['protocol_sha256']
    assert digest(ROOT / 'data/round10_visual_before_text.json') == freeze['visual_record_sha256']
    assert digest(ROOT / 'sources/P499399_photo.jpg') == freeze['photo_sha256']
    rows = cdli('P499399')
    assert len(rows) == 10
    front = [r for r in rows if r['surface'] == 'obverse']
    back = [r for r in rows if r['surface'] == 'reverse']
    assert len(front) == 7 and len(back) == 3
    # This is a visual observation made by the researcher, not inferred by the parser.
    assert json.loads((ROOT / 'data/round10_visual_before_text.json').read_text())['source_dimensions'] == [1006, 1762]
    write('round10_post_freeze_comparison.csv', [
        dict(target='G1 initial complex', coordinate='obverse 2 / obverse 3', editorial_forms=front[1]['editorial_text']+' / '+front[2]['editorial_text'],
             observation='similar initial graphic group recorded before text exposure', status='partial graphic agreement; sound nin not independently established'),
        dict(target='G2 radial complex', coordinate='obverse 1 / obverse 4', editorial_forms=front[0]['editorial_text']+' / '+front[3]['editorial_text'],
             observation='radial forms recorded before text exposure', status='editor assigns divine classifiers; graphic recurrence is not necessarily a spoken root'),
        dict(target='G3 triangular motifs', coordinate='obverse 3 / obverse 7 upper tier', editorial_forms=front[2]['editorial_text']+' / '+front[6]['editorial_text'],
             observation='similar motifs with differing adjunct strokes; boundaries unresolved', status='no canonical sign identity assigned'),
        dict(target='same ending after two different bases', coordinate='obverse 3 / reverse 1', editorial_forms=front[2]['editorial_text']+' / '+back[0]['editorial_text'],
             observation='only one photographed face; reverse absent in supplied detail', status='not visually tested as a pair'),
    ])
    contact = cdli('P325891')
    assert len(contact) == 11
    idx = {(r['surface'], r['line']):r for r in contact}
    assert idx['obverse','3']['editorial_text'].endswith('lugal-ta')
    assert idx['obverse','5']['editorial_text'] == 'in-da-ge26-en'
    write('round10b_contact_control.csv', [
        dict(artifact='P325891', coordinate='obverse 3', editorial_form=idx['obverse','3']['editorial_text'],
             photo_status='front visible; the text continues across visual tiers',
             interpretation='possible comitative function with ta spelling; contextual analysis from Sallaberger 2011 p351',
             limitation='ta identification not independently established from sound; no error-rate estimate'),
        dict(artifact='P325891', coordinate='obverse 5', editorial_form=idx['obverse','5']['editorial_text'],
             photo_status='front visible; separate group sequence',
             interpretation='da in verbal chain supplies an internal comparison in the proposed analysis',
             limitation='unblinded collation; contact explanation competes with internal variation and scribal/editorial error'),
    ])
    write('round10_hypothesis_matrix.csv', [
        dict(model='common_descent', prediction='recurring lexical and bound-morpheme correspondences in independent dated witnesses', present_result='not established by this round', next_test='predefine basic-word and function-matched paradigm sample; keep nonmatches'),
        dict(model='contact', prediction='locally concentrated spelling or grammatical variants correlated with independently supported contact context', present_result='one identifiable modern analysis and a physical photograph located; not independently generalized', next_test='compare ta/da environments across Garshana and control archive; verify individual signs'),
        dict(model='independent_typology', prediction='similar segmentation architecture without stable corresponding exponents', present_result='compatible with current evidence; not uniquely selected', next_test='compare full paradigms including unrelated agglutinative controls'),
        dict(model='graphic_ambiguity', prediction='same or similar visible motifs need not encode the same spoken root', present_result='radial and triangular repetitions require contextual sign analysis', next_test='compare spelling variants and ancient phonetic/lexical witnesses'),
    ])
    print('Freeze verified; 10 + 11 editorial lines; 4 image-comparison rows; 2 contact-control rows; 4 competing models.')

if __name__ == '__main__':
    main()
