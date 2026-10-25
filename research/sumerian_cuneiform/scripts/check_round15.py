#!/usr/bin/env python3
"""Check transferred arithmetic with exact fractions and explicit uncertainty."""
import collections
import csv
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import re
from extract_early_and_uriii import Plain

ROOT=Path(__file__).resolve().parents[1]
WEIGHTS={'N01':Fraction(1),'N14':Fraction(6),'N45':Fraction(60),
         'N34':Fraction(180),'N39~a':Fraction(1,5)}

def write(name, rows):
    with (ROOT/'data'/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

def extract(obj):
    raw=(ROOT/'sources'/f'{obj}_card.html').read_text()
    block=re.search(r'<div class="translate-indent[^\"]*">(.*?)</div>',raw,re.S)
    p=Plain();p.feed(block[1]);surface=column='';rows=[]
    for line in ''.join(p.parts).splitlines():
        line=line.strip()
        if line in {'obverse','reverse'}:surface=line;column=''
        elif line.startswith('column '):column=line[7:]
        else:
            m=re.fullmatch(r'(\d+(?:\.[a-z]\d*)?)\.\s*(.*)',line)
            if not m:continue
            prefix=m[2].split(',')[0].strip()
            quantity=bool(re.match(r'^\[?\d+\(N\d',prefix))
            counts=collections.Counter()
            if quantity:
                for n,k in re.findall(r'(\d+)\((N\d+(?:~[a-z])?)\)',prefix):counts[k]+=int(n)
            rows.append(dict(artifact=obj,surface=surface,column=column,cell=m[1],
                             editorial_sequence=m[2],quantity_prefix=prefix,is_quantity_entry=quantity,
                             prefix_restored='[' in prefix or ']' in prefix,
                             prefix_uncertain='#' in prefix or '?' in prefix,
                             prefix_gap='...' in prefix,
                             proposed_counts=json.dumps(dict(counts),sort_keys=True)))
    assert len({(r['surface'],r['column'],r['cell']) for r in rows})==len(rows)
    return rows

def value(counts,weights=WEIGHTS):return sum((n*weights[k] for k,n in counts.items()),Fraction(0))

def main():
    freeze=json.loads((ROOT/'data/round15_visual_freeze.json').read_text())
    assert hashlib.sha256((ROOT/'data/round15_protocol.json').read_bytes()).hexdigest()==freeze['protocol_sha256']
    assert hashlib.sha256((ROOT/'data/round15_visual_before_rows.json').read_bytes()).hexdigest()==freeze['record_sha256']
    for name,sha in freeze['image_hashes'].items():assert hashlib.sha256((ROOT/'sources'/name).read_bytes()).hexdigest()==sha
    allrows=[];results=[];entries=[]
    for obj in ['P005371','P005372']:
        rows=extract(obj);allrows.extend(rows)
        front=[r for r in rows if r['surface']=='obverse' and r['is_quantity_entry']]
        totals=[r for r in rows if r['surface']=='reverse' and r['is_quantity_entry']]
        assert len(totals)==1,obj
        total=totals[0];sumcounts=collections.Counter();frontsum=Fraction(0)
        for r in front:
            counts=json.loads(r['proposed_counts']);sumcounts.update(counts);v=value(counts);frontsum+=v
            entries.append(dict(artifact=obj,coordinate=f"{r['surface']} {r['column']}:{r['cell']}",
                                proposed_counts=r['proposed_counts'],value_in_N01_units=str(v),
                                restored=r['prefix_restored'],uncertain=r['prefix_uncertain'],gap=r['prefix_gap']))
        tc=json.loads(total['proposed_counts']);tv=value(tc);diff={k:tc.get(k,0)-sumcounts.get(k,0) for k in sorted(set(tc)|set(sumcounts)) if tc.get(k,0)!=sumcounts.get(k,0)}
        uncertain=[r for r in front if r['prefix_restored'] or r['prefix_uncertain'] or r['prefix_gap']]
        clean=[r for r in front if r not in uncertain]
        results.append(dict(artifact=obj,quantity_entries=len(front),unflagged_quantity_entries=len(clean),
                            uncertain_or_restored_entries=len(uncertain),
                            restored_entries=sum(r['prefix_restored'] for r in front),gap_entries=sum(r['prefix_gap'] for r in front),
                            total_vector=tc,entry_sum_vector=dict(sumcounts),total_minus_entries_vector=diff,
                            entry_sum_N01=str(frontsum),total_N01=str(tv),residual_N01=str(tv-frontsum),
                            listed_quantity_sum_holds=tv==frontsum,
                            confidence='editorial unflagged-quantity test, not independent collation of every wedge' if not uncertain else
                            'uncertain: named/restored quantities fit, hidden gap not filled or certified zero',
                            hypotheses='reverse quantity is total of all visible obverse quantity-bearing entries; model weights transferred unchanged'))
    assert len(allrows)==29 and len(entries)==21
    assert results[0]['entry_sum_N01']=='7162/5' and results[0]['uncertain_or_restored_entries']==8
    assert results[1]['entry_sum_N01']=='5856/5' and results[1]['uncertain_or_restored_entries']==0
    assert all(r['listed_quantity_sum_holds'] for r in results)
    assert results[1]['total_minus_entries_vector']=={'N01':-5,'N14':-19,'N34':2,'N39~a':-5,'N45':-4}
    write('round15_editorial_cells.csv',allrows);write('round15_quantity_entries.csv',entries)
    # The one new equation determines N14/N01 only with two imported ratios
    # and the previously obtained conditional N45/N14 relation.
    d=results[1]['total_minus_entries_vector']
    bcoef=d['N34']*3*10+d['N45']*10+d['N14']
    ccoef=Fraction(d['N01'])+Fraction(d['N39~a'],5)
    assert bcoef==1 and ccoef==-6
    derived=-ccoef/bcoef
    counterfactual={'N01':Fraction(1),'N14':Fraction(10),'N45':Fraction(100),
                    'N34':Fraction(300),'N39~a':Fraction(1,5)}
    cfres=sum((n*counterfactual[k] for k,n in d.items()),Fraction(0))
    assert cfres==4
    summary={'date':'2026-10-25','model_weights':{k:str(v) for k,v in WEIGHTS.items()},'results':results,
        'additional_conditional_constraint':{'equation':'2D-4A-19B-5C-5F=0',
             'names':{'D':'N34','A':'N45','B':'N14','C':'N01','F':'N39~a'},
             'assumptions':['D=3A from external numerical model','A=10B conditional result from earlier P005390','F=C/5 from external fractional model','reverse quantity is additive total'],
             'reduction':'B-6C=0','derived_B_over_C':str(derived),
             'counterfactual_B_over_C_10_residual_in_C_units':str(cfres),
             'limit':'This is not a reconstruction of all numerical weights without prior assumptions.'},
        'S_alternative':'Not evaluated as a complete transferable system: N39~a is not mapped by the cited S table; no invented S weight for it.',
        'status':'prior editorial snippets known; new-image observations saved before new extraction; not blind phonetic decipherment',
        'independent_complete_phonetic_translations':0,'book_modified':False}
    (ROOT/'data/round15_arithmetic_results.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print('29 cells;21 quantity entries. P005372:5856/5 total,0 residual,8 unflagged entries. P005371:7162/5,0 named-count residual,8 uncertain/restored entries. Conditional B/C=6; alternative10 residual4.')

if __name__=='__main__':main()
