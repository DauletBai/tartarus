#!/usr/bin/env python3
"""Extract editorial records while preserving sign/reading/model distinctions."""
import csv
from html.parser import HTMLParser
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]

class Plain(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts=[]
        self.italic=0
    def handle_starttag(self, tag, attrs):
        if tag=='i': self.italic+=1
        if tag=='br': self.parts.append('\n')
    def handle_endtag(self, tag):
        if tag=='i': self.italic-=1
    def handle_data(self,data):
        if not self.italic: self.parts.append(data)

def write(name,fields,rows):
    with (ROOT/'data'/name).open('w',encoding='utf-8-sig',newline='') as out:
        w=csv.DictWriter(out,fieldnames=fields)
        w.writeheader(); w.writerows(rows)

def cdli(artifact):
    raw=(ROOT/'sources'/f'{artifact}_card.html').read_text()
    block=re.search(r'<div class="translate-indent[^\"]*">(.*?)</div>',raw,re.S)
    assert block, artifact
    parser=Plain(); parser.feed(block[1])
    obj=surface=column=''
    rows=[]
    for line in ''.join(parser.parts).splitlines():
        line=line.strip()
        if re.fullmatch(r'(?:object )?(?:tablet|canephore|cone)',line):
            obj=line.removeprefix('object '); surface=column=''
        elif line in {'obverse','reverse'} or line.startswith('surface '):
            surface=line; column=''
        elif line.startswith('column '): column=line[7:]
        else:
            m=re.fullmatch(r"(\d+[a-z]?'?)\.\s*(.+)",line)
            if m:
                rows.append(dict(artifact=artifact,object=obj,surface=surface,column=column,line=m[1],editorial_text=m[2]))
    assert len({tuple(r[k] for k in ['object','surface','column','line']) for r in rows})==len(rows)
    write(f'{artifact}_early_editorial_lines.csv',list(rows[0]),rows)
    return rows

class Words(HTMLParser):
    def __init__(self):
        super().__init__(); self.active=None; self.depth=0; self.rows=[]
    def handle_starttag(self,tag,attrs):
        d=dict(attrs)
        if tag=='sup' and self.active is not None:
            self.active['parts'].append('{')
        if tag=='span' and self.active is not None:
            self.depth+=1
        elif tag=='span' and d.get('class','').startswith('w sux ') and d.get('id','').startswith('Q000954.'):
            self.active={'word_id':d['id'],'raw_annotation':d.get('title',''),'parts':[]}; self.depth=1
    def handle_data(self,data):
        if self.active is not None: self.active['parts'].append(data)
    def handle_endtag(self,tag):
        if tag=='sup' and self.active is not None:
            self.active['parts'].append('}')
        if tag=='span' and self.active is not None:
            self.depth-=1
            if not self.depth:
                a=self.active; ann=a['raw_annotation']
                morph=re.search(r'\$([^/]+)/',ann)
                self.rows.append(dict(word_id=a['word_id'],editorial_form=''.join(a['parts']),lemma=ann.split('[')[0],editorial_morph_model=morph[1] if morph else '',raw_annotation=ann))
                self.active=None

def main():
    a=cdli('P000449'); b=cdli('P226284')
    assert len(a)==14 and len(b)==20, (len(a),len(b))
    parser=Words(); parser.feed((ROOT/'sources/UrNamma34_Q000954.html').read_text())
    assert len(parser.rows)==16, len(parser.rows)
    write('Q000954_word_layers.csv',list(parser.rows[0]),parser.rows)
    hits=[]
    for artifact in ['P222607','P222608']:
        with (ROOT/'data'/f'{artifact}_editorial_lines.csv').open(encoding='utf-8-sig') as source:
            for row in csv.DictReader(source):
                if 'mu-na-du3' in row['editorial_transliteration'].split():
                    hits.append(dict(artifact=artifact,object='cone',coordinate=f"{row['column']}:{row['line']}",form='mu-na-du3',level='editorial transcription'))
    for row in b:
        for token in row['editorial_text'].split():
            if token in {'mu-na-du3','mu-na-gi4','nin-a-ni','e2-a-ni'}:
                hits.append(dict(artifact=row['artifact'],object=row['object'],coordinate=f"{row['surface']} {row['column']}:{row['line']}",form=token,level='editorial transcription'))
    write('2026-10-24_structural_hits.csv',list(hits[0]),hits)
    print(f'P000449: {len(a)} lines; P226284: {len(b)} lines across two object sections; Q000954: {len(parser.rows)} editorially analyzed words.')
    print(f'{len(hits)} targeted occurrences; these are not independent etymological confirmations.')

if __name__=='__main__': main()
