#!/usr/bin/env python3
"""Repeatable checks of witness coverage and exact editorial sign-label matches."""
import csv
from collections import Counter
from html.parser import HTMLParser
from itertools import combinations
from pathlib import Path
import re
from extract_early_and_uriii import cdli

ROOT=Path(__file__).resolve().parents[1]
EARLY=['P000449','P000106','P000161','P000163']

class Score(HTMLParser):
    def __init__(self):
        super().__init__(); self.rows=[]; self.row=[]; self.cell=None
    def handle_starttag(self,tag,attrs):
        if tag=='tr': self.row=[]
        if tag=='td': self.cell=[]
    def handle_data(self,data):
        if self.cell is not None: self.cell.append(data)
    def handle_endtag(self,tag):
        if tag=='td' and self.cell is not None:
            self.row.append(' '.join(' '.join(self.cell).split())); self.cell=None
        if tag=='tr' and self.row: self.rows.append(self.row)

def write(name,fields,rows):
    with (ROOT/'data'/name).open('w',encoding='utf-8-sig',newline='') as out:
        writer=csv.DictWriter(out,fieldnames=fields); writer.writeheader(); writer.writerows(rows)

def coordinate(row):
    return ' '.join([row['surface'],row['column'],row['line']]).strip()

def signature(text):
    # Remove only the inventory-entry prefix. Damage and uncertainty stay visible.
    return re.sub(r'^\[?1\(N01\)\]?\s*,\s*','',text).strip()

def main():
    data={art:cdli(art) for art in EARLY}
    score=Score(); score.feed((ROOT/'sources/ArchaicLuA_CDLI_score.html').read_text())
    parent=''; indexed=[]
    for row in score.rows:
        match=re.match(r'P\d{6}',row[0])
        if match and len(row)>=3 and match[0] in EARLY:
            indexed.append(dict(artifact=match[0],composite_reference=parent,witness_coordinate=row[1],editorial_text=row[2]))
        elif not match: parent=row[0]
    assert indexed
    write('round2_score_rows.csv',list(indexed[0]),indexed)
    coverage=[]
    for art in EARLY:
        selected=[r for r in indexed if r['artifact']==art]
        coverage.append(dict(artifact=art,catalogue_records=len(data[art]),score_rows=len(selected),composite_references=';'.join(dict.fromkeys(r['composite_reference'] for r in selected))))
    write('round2_score_coverage.csv',list(coverage[0]),coverage)
    matches=[]
    for left,right in combinations(EARLY,2):
        for a in data[left]:
            for b in data[right]:
                key=signature(a['editorial_text'])
                if key==signature(b['editorial_text']) and key and not any(x in key for x in ['[',']','X']):
                    matches.append(dict(left=left,left_coordinate=coordinate(a),right=right,right_coordinate=coordinate(b),sign_label_sequence=key,uncertain_markers=bool(re.search(r'[#?]',key)),claim_level='exact editorial labels; not ancient pronunciation'))
    assert matches
    write('round2_exact_sign_label_matches.csv',list(matches[0]),matches)
    gudea=cdli('P251600')
    assert len(gudea)==8
    targets=[r for r in gudea if r['editorial_text'] in {'lugal-a-ni','mu-na-du3'}]
    assert len(targets)==2
    write('round2_gudea_targets.csv',list(targets[0]),targets)
    print('Coverage:',coverage)
    print('Exact matches by pair:',dict(Counter((r['left'],r['right']) for r in matches)))
    print('Gudea targets:',targets)

if __name__=='__main__': main()
