#!/usr/bin/env python3
"""Reproducible editorial comparison, not independent decipherment or ancestry test."""
import collections
import csv
import hashlib
import json
from pathlib import Path
import re
import zipfile
from census_possessives import nodes

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / 'sources/ETCSRI_2026-10-24_json.zip'
EXPECTED = '66991601df193a5cb93bccf92e9051e86a98fbb668a9dd343477d14f0c9d54d6'

def write(name, fields, rows):
    with (ROOT / 'data' / name).open('w', encoding='utf-8-sig', newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields); w.writeheader(); w.writerows(rows)

def line_labels(document):
    result={}
    def visit(obj):
        if isinstance(obj,dict):
            if obj.get('type')=='line-start':result[obj.get('ref','')]=obj.get('label','')
            for v in obj.values():visit(v)
        elif isinstance(obj,list):
            for v in obj:visit(v)
    visit(document)
    return result

def main():
    assert hashlib.sha256(ARCHIVE.read_bytes()).hexdigest()==EXPECTED
    cop=[]; lexical=[]; all_values=collections.Counter(); seen=set(); failed=[]
    domain_labels={
        'water':{'water'}, 'earth_place':{'earth','place','ground'},
        'sun':{'sun'}, 'moon':{'moon'}, 'person':{'person'},
        'woman':{'woman'}, 'man_male':{'man','male'}, 'mother':{'mother'},
        'father':{'father'}, 'name':{'name'},
    }
    with zipfile.ZipFile(ARCHIVE) as z:
        cat=json.loads(z.read('etcsri/catalogue.json'))['members']
        for path in sorted(z.namelist()):
            if not re.search(r'/corpusjson/Q\d+\.json$',path):continue
            qid=Path(path).stem
            try:doc=json.loads(z.read(path))
            except ValueError:failed.append(path);continue
            labels=line_labels(doc)
            for n in nodes(doc):
                f=n['f']
                if not f.get('lang','').startswith('sux'):continue
                key=(qid,n.get('ref',''),f.get('form',''))
                if key in seen:continue
                seen.add(key)
                row=dict(qid=qid,ref=n.get('ref',''),line_label=labels.get(n.get('ref','').rsplit('.',1)[0],''),
                         form=f.get('form',''),cf=f.get('cf',''),gw=f.get('gw',''),sense=f.get('sense',''),
                         norm=f.get('norm',''),morph=f.get('morph',''),morph2=f.get('morph2',''),
                         period=cat.get(qid,{}).get('period',''),designation=cat.get(qid,{}).get('designation',''))
                match=re.search(r'(?:^|\.)N6=([^.]*)',row['morph'])
                if match:
                    all_values[match[1]]+=1
                    if match[1] in {'men','am','menden','menzen','meš','mesh'} or '-me-en' in row['form']:
                        cop.append(dict(row,n6_value=match[1]))
                if '-me-en' in row['form'] and not match:
                    cop.append(dict(row,n6_value='no_N6_label'))
                for domain, meanings in domain_labels.items():
                    if row['gw'].lower() in meanings or row['sense'].lower() in meanings:
                        lexical.append(dict(row,domain=domain))
    fields=['qid','ref','line_label','form','cf','gw','sense','norm','morph','morph2','period','designation']
    write('round11_copula_all_matches.csv',fields+['n6_value'],cop)
    write('round11_lexical_all_matches.csv',fields+['domain'],lexical)
    men=[r for r in cop if r['n6_value']=='men']
    counts=collections.Counter(re.search(r'N6=([^.]*)',r['morph2'])[1] if 'N6=' in r['morph2'] else 'unlabelled' for r in men)
    candidates=[r for r in cop if r['ref']=='Q000377.64.2']
    assert len(candidates)==1
    r=candidates[0]
    assert r['line_label']=='Cylinder A iii 6'
    assert r['form']=='nu-tuku-me' and 'N6=COP-2-SG' in r['morph2']
    write('round11_annotation_discrepancies.csv',
          ['source','coordinate','literal_label','comparison','status'],[
        dict(source='Jagersma2010',coordinate='printed682 / PDF709 / example26',literal_label='you=ABS=be-1SG.S',
             comparison='same example translated you yourself; example31 labels corresponding copula2SG',
             status='suspected local gloss typo; checked on rendered PDF; original unchanged'),
        dict(source='ETCSRI2026-10-24 snapshot vs Jagersma2010',coordinate='Q000377.64.2 / Cylinder A iii6 / example30 printed683',literal_label=r['morph2'],
             comparison='Jagersma example30 assigns1SG to nu-tuku-me in same line',
             status='person-tag disagreement, not a photographic sound decipherment; needs full-context collation'),
    ])
    summary=dict(date='2026-10-25',snapshot_sha256=EXPECTED,parse_failures=failed,
                 unique_sumerian_word_nodes=len(seen),all_n6_counts=dict(all_values),
                 exported_copula_rows=len(cop),men_rows=len(men),men_editorial_person_counts=dict(counts),
                 men_compositions=len({r['qid'] for r in men}),
                 men_rows_explicit_final_n=sum(bool(re.search(r'me-en',r['form'])) for r in men),
                 lexical_rows=len(lexical),lexical_domain_counts=dict(collections.Counter(r['domain'] for r in lexical)),
                 limits=['Composites and word nodes are not independent physical witnesses.',
                         'Person-tag counts contain known disputed annotation; no corrected count silently substituted.',
                         'Corpus absence is not language absence; no ancestry percentage or chance p-value.'])
    (ROOT/'data/round11_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(summary,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
