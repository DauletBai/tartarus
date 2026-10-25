#!/usr/bin/env python3
"""Complete the saved three-object graphic and conditional arithmetic audit.

Modern sign labels/weights remain explicitly separate from visual observations.
"""
import collections
import csv
import hashlib
import json
from pathlib import Path
import re
from extract_early_and_uriii import Plain

ROOT=Path(__file__).resolve().parents[1]

def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()

def write(name,rows):
    with (ROOT/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

def cells(artifact):
    raw=(ROOT/'sources'/f'{artifact}_card.html').read_text()
    block=re.search(r'<div class="translate-indent[^\"]*">(.*?)</div>',raw,re.S)
    p=Plain();p.feed(block[1]);surface=column='';result=[]
    for line in ''.join(p.parts).splitlines():
        line=line.strip()
        if line in {'obverse','reverse'}:surface=line;column=''
        elif line.startswith('column '):column=line[7:]
        else:
            m=re.fullmatch(r'(\d+(?:\.[a-z]\d*)?)\.\s*(.*)',line)
            if m:
                prefix=m[2].split(',')[0]
                counts=dict((n,int(k)) for k,n in re.findall(r'(\d+)\((N\d+(?:~[a-z])?)\)',prefix))
                result.append(dict(artifact=artifact,surface=surface,column=column,cell=m[1],
                                   editorial_sign_sequence=m[2],
                                   proposed_numeral_counts=json.dumps(counts,sort_keys=True),
                                   uncertain_numeral_prefix=any(c in prefix for c in ['#','?','[',']'])))
    assert len({(r['surface'],r['column'],r['cell']) for r in result})==len(result)
    return result

def main():
    freeze=json.loads((ROOT/'data/round14_visual_freeze.json').read_text())
    assert digest(ROOT/'data/round14_manual_annotations.json')==freeze['record_sha256']
    assert digest(ROOT/'data/round14_protocol.json')==freeze['protocol_sha256']
    assert digest(ROOT/'data/round14_heldout_visual_results.json')==freeze['heldout_results_sha256']
    for name,sha in freeze['images'].items():assert digest(ROOT/name)==sha
    ann=json.loads((ROOT/'data/round14_manual_annotations.json').read_text())
    panels=[];graphs=[]
    for v in ann['views']:
        ids={p['id'] for p in v['panels']}
        for p in v['panels']:
            x1,y1,x2,y2=p['box'];assert 0<=x1<x2<=1000 and 0<=y1<y2<=1000
            panels.append(dict(view=v['id'],object=v['object'],panel=p['id'],box=json.dumps(p['box']),
                               description=p['description'],blank_control=p['blank_control'],boundary_status=p['boundary_status']))
        for g in v['candidates']:
            assert g['panel'] in ids
            x1,y1,x2,y2=g['box'];assert 0<=x1<x2<=1000 and 0<=y1<y2<=1000
            graphs.append(dict(view=v['id'],object=v['object'],id=g['id'],panel=g['panel'],family=g['family'],
                               box=json.dumps(g['box']),description=g['description'],unit=g['unit'],
                               visible_mark_count_if_clear=g['visible_mark_count_if_clear'],
                               boundary_uncertain=g['boundary_uncertain'],sound='',meaning=''))
    assert len(panels)==29 and len(graphs)==60
    write('round14_panels.csv',panels);write('round14_graphic_candidates.csv',graphs)
    identity=[];allcells=[]
    for oid,artifact,suffix in [(327384,'P005390','02'),(329081,'P005393','01'),(327385,'P005366','03')]:
        api=json.loads((ROOT/'sources'/f'Met{oid}_api.json').read_text())
        raw=(ROOT/'sources'/f'{artifact}_card.html').read_text()
        assert f'MMA 1988.433.{suffix}' in raw
        assert api['accessionNumber']==f'1988.433.{int(suffix)}'
        identity.append(dict(met_object=oid,accession=api['accessionNumber'],cdli=artifact,
                             met_period=api.get('period'),met_date=api.get('objectDate'),
                             cdli_language='Undetermined' if 'Undetermined' in raw else 'not independently extracted',
                             match='museum accession verified; zero-padding normalized, ID adjacency not used'))
        allcells.extend(cells(artifact))
    assert len(allcells)==26
    write('round14_verified_identity.csv',identity);write('round14_editorial_cells.csv',allcells)
    lookup={(r['artifact'],r['surface'],r['column'],r['cell']):r for r in allcells}
    def get(a,s,c,k):
        r=lookup[a,s,c,k];assert not r['uncertain_numeral_prefix'];return json.loads(r['proposed_numeral_counts'])
    def difference(total,parts):
        out=collections.Counter(total)
        for p in parts:
            for k,v in p.items():out[k]-=v
        return {k:v for k,v in out.items() if v}
    a=get('P005390','reverse','1','1.a')
    b=get('P005390','reverse','1','1.b1');c=get('P005390','reverse','1','1.b2')
    diff=difference(a,[b,c]);assert diff=={'N45':1,'N14':-10}
    she={'N45':60,'N14':6,'N01':1}
    s={'N45':3600,'N14':10,'N01':1}
    val=lambda counts,weights:sum(n*weights[k] for k,n in counts.items())
    assert val(a,she)==153 and val(b,she)==102 and val(c,she)==51
    assert val(a,s)!=val(b,s)+val(c,s)
    left=get('P005366','obverse','1','1.b')
    right=get('P005366','obverse','1','2.b')
    total=get('P005366','reverse','1','2')
    assert not difference(total,[left,right])
    arithmetic={'date':'2026-10-25','status':'post-editorial exploratory arithmetic, not a blind phonetic decipherment',
        'focal':{'artifact':'P005390','assumption':'reverse1.1.a is additive parent of1.1.b1 and1.1.b2; must be checked against layout and genre',
                 'total_vector':a,'component_vectors':[b,c],'total_minus_components':diff,
                 'conditional_relation':'weight(N45)=10*weight(N14)',
                 'what_is_not_determined':'N14/N01 ratio is not determined by this equation; words, pronunciation and units in litres not determined',
                 'external_model_reference':'Englund2004 Figure4b printed33: SE system N45=10*N14,N14=6*N1',
                 'external_weights_in_N1_units':she,'values':[153,102,51],'sum_holds':True,
                 'alternative_S_model_reference':'Englund2004 Figure4a printed32',
                 'alternative_S_values':[val(a,s),val(b,s),val(c,s)],'alternative_S_sum_holds':False},
        'comparison':{'artifact':'P005366','source_cells':['obverse1.1.b','obverse1.2.b','reverse1.2'],
                      'left':left,'right':right,'total':total,
                      'symbolic_equation':'2*A + (4*B+C) = 2*A+4*B+C',
                      'holds_without_unit_weights':True,
                      'limit':'This preserves a formal inventory of editorial numeral classes. It is not an independent photographic verification of each class or proof of phonetic readings.'},
        'limits':['Additive-parent interpretation is a hypothesis, not a consequence of a Latin subcell label alone.',
                  'Numerical-system tables were consulted during verification; no fully blind rediscovery claim.',
                  'Two different measurement systems cannot be conflated because sign names overlap.']}
    (ROOT/'data/round14_arithmetic_checks.json').write_text(json.dumps(arithmetic,ensure_ascii=False,indent=2)+'\n')
    model_rows=[
        dict(model='alphabetic',support='repeated shapes alone compatible with many scripts',missing='independent phoneme anchors; systematic alphabetic segmentation; value transfer to new sequences',result='not established; do not assign60 candidate groups to60 letters'),
        dict(model='syllabic',support='not selected or excluded by shape counts alone',missing='independent syllabic spellings and correspondences',result='not established'),
        dict(model='logographic_or_mixed_account',support='cell layout; recurring complex beside variable impression bundles; conditional accounting equalities',missing='word meanings, speech syntax and phonetic parts not independently identified',result='compatible with observed account structure; no full translation'),
        dict(model='single_global_numeric_weight_per_shape',support='simple repeated counts',missing='same sign names occur in different proposed numerical systems',result='not supported as a universal rule; context-dependent systems must remain distinct'),
        dict(model='Kazakh_ancestral_direction_K0',support='no independently recovered comparative phonetic sequence in this batch',missing='dated intermediates; regular lexical and bound-morpheme correspondences; directional innovation',result='remains open, not established or disproved by this graphic batch'),
    ]
    write('round14_model_assessment.csv',model_rows)
    summary={'date':'2026-10-25','analysed_objects':3,'selected_image_assets':6,'photographic_views':5,'drawing_assets_separate':1,
             'viewing_regions':len(panels),'blank_control_regions':sum(p['blank_control'] for p in panels),
             'candidate_groups':len(graphs),'groups_by_family':dict(collections.Counter(g['family'] for g in graphs)),
             'verified_target_editorial_cells':len(allcells),'identity_mismatches_retained':['Met327383 is1988.380.5, not1988.433.1','P005389 andP005391 do not identify the comparison accession numbers'],
             'heldout_visual_prediction_results':json.loads((ROOT/'data/round14_heldout_visual_results.json').read_text()),
             'independent_complete_translations':0,'alphabet_size':None,'book_modified':False}
    (ROOT/'data/round14_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print('Verified:3 objects,5 photographic views+1 drawing,29 panels,60 candidate groups,26 editorial cells; conditional153=102+51 and independent-weight symbolic sum checked.')

if __name__=='__main__':main()
