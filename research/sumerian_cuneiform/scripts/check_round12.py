#!/usr/bin/env python3
"""Verify a graph-first observation and keep proposed readings in separate tables."""
import csv
import hashlib
import json
from pathlib import Path
import re
from extract_early_and_uriii import Plain

ROOT=Path(__file__).resolve().parents[1]

def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()

def write(name, rows):
    with (ROOT/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

def main():
    freeze=json.loads((ROOT/'data/round12_visual_freeze.json').read_text())
    assert digest(ROOT/'data/round12_protocol.json')==freeze['protocol_sha256']
    assert digest(ROOT/'data/round12_visual_before_text.json')==freeze['visual_sha256']
    for name,sha in freeze['image_sha256'].items():assert digest(ROOT/'sources'/name)==sha
    api=json.loads((ROOT/'sources/Met327384_api.json').read_text())
    assert api['objectID']==327384 and api['accessionNumber']=='1988.433.2'
    raw=(ROOT/'sources/P005390_card.html').read_text()
    assert 'MMA 1988.433.02' in raw
    block=re.search(r'<div class="translate-indent[^\"]*">(.*?)</div>',raw,re.S)
    parser=Plain();parser.feed(block[1])
    surface=column='';rows=[]
    for line in ''.join(parser.parts).splitlines():
        line=line.strip()
        if line in {'obverse','reverse'}:surface=line;column=''
        elif line.startswith('column '):column=line[7:]
        else:
            m=re.fullmatch(r'(\d+(?:\.[a-z]\d*)?)\.\s*(.*)',line)
            if m:rows.append(dict(artifact='P005390',surface=surface,column=column,
                                  cell_as_source=m[1],editorial_sign_sequence=m[2]))
    assert len(rows)==14
    assert len({(r['surface'],r['column'],r['cell_as_source']) for r in rows})==14
    write('round12_P005390_editorial_cells.csv',rows)
    write('round12_post_freeze_comparison.csv',[
        dict(target='G01 branching graphic family',observation='Three approximate regions recorded without sound labels; native-coordinate correction retained separately',
             editorial_claim='SZE~a occurs twice; SUM~b also occurs in the edition',
             result='Branch-like appearance is not enough to equate whole signs; region-to-cell mapping provisional'),
        dict(target='G02 repeated lower pair on viewB',observation='Paired closely similar complexes recorded before individual text',
             editorial_claim='reverse column2 cell1: SANGA~a GAL~a SANGA~a',
             result='Repeated outer labels compatible with observed pair; central boundary and full sign identity not independently established'),
        dict(target='G03 rounded impressions',observation='Different apparent sizes; two small impressions in one field and large/small pairing in another; damaged outlines retained',
             editorial_claim='Edition uses N01,N14,N19,N45,N46 numeric-sign conventions',
             result='Category labels and numerical values not independently derived here; full arithmetic reconstruction not performed'),
        dict(target='root+affix chain',observation='No independently established spoken-word segmentation',
             editorial_claim='Early sign sequences with cells and metrological labels',
             result='Neither confirms nor refutes agglutinative spoken grammar'),
    ])
    gdl=json.loads((ROOT/'data/round12_ama_adda_editorial_gdl_nodes.json').read_text())
    audit=[]
    for row in gdl:
        f=row['f'];signs=f.get('gdl',[])
        audit.append(dict(ref=row['ref'],editorial_lemma=f['cf'],proposed_meaning=f['gw'],
                          editorial_form=f.get('form',''),
                          sign_list_labels=' '.join(s.get('gdl_sign','') for s in signs),
                          unicode_font_preview=''.join(s.get('utf8','') for s in signs),
                          original_photo_collation='not established for this occurrence',
                          damage_flags=json.dumps([{k:s[k] for k in ['break','breakStart','o','ho','hc'] if k in s} for s in signs if any(k in s for k in ['break','breakStart','o','ho','hc'])],ensure_ascii=False)))
    assert len(audit)==62
    write('round12_ama_adda_layer_audit.csv',audit)
    readers={'P135177':['P135177_reader_32302.html','P135177_reader_32303.html'],
             'P226721':['P226721_reader_44580.html']}
    availability=[]
    for artifact,files in readers.items():
        urls=[]
        for name in files:
            s=(ROOT/'sources'/name).read_text()
            urls.extend(u.replace('\\/','/') for u in re.findall(r'url:\s*"([^"]+)"',s))
        availability.append(dict(artifact=artifact,reader_urls=' | '.join(urls),
                                 cdli_photo_available=any('/photo/' in u for u in urls),
                                 outcome='drawing-only readers; no photo collated'))
    write('round12b_image_availability.csv',availability)
    summary={'date':'2026-10-25','editorial_cells':len(rows),'target_layer_audit_rows':len(audit),
             'ama_editorial_occurrences':sum(r['editorial_lemma']=='ama' for r in audit),
             'adda_editorial_occurrences':sum(r['editorial_lemma']=='adda' for r in audit),
             'new_graph_first_object':'MMA1988.433.2 / P005390','official_views_inspected':2,
             'independent_phonetic_readings':0,'independent_complete_translations':0,
             'limits':['Object/title prior already known; no semantic-blind claim.',
                       'Spatial amendment preserves the original frozen record.',
                       'N-labels and sign-list labels are editorial hypotheses, not observed pronunciation.',
                       'Drawing-only availability is not proof that no photograph exists anywhere.']}
    (ROOT/'data/round12_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print('Freeze verified;14 editorial cells with subcell coordinates preserved;62 reading-layer audit rows;2 drawing-only target records.')

if __name__=='__main__':main()
