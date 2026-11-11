#!/usr/bin/env python3
"""Offline, hash-pinned extract of USDA Release 6 rice preparation factors."""
import csv
import hashlib
import io
import json
from pathlib import Path
import sys

SHA256 = 'b863e891989020edf3a429af8060523e5dee275699ae08893a5f91ff9a84b1e5'
ROOT = Path(__file__).resolve().parents[1]

def extract(path):
    raw = Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('Input differs from reviewed USDA Release 6 CSV')
    rows = [r for r in csv.DictReader(io.StringIO(raw.decode())) if r['Retn_Code'] in ('431','432','434')]
    if len(rows) != 78 or len({(r['Retn_Code'],r['Nutr_No']) for r in rows}) != 78:
        raise ValueError('Unexpected source dimensions or duplicates')
    return {
        'release': 'USDA Table of Nutrient Retention Factors, Release 6 (2007)',
        'source_url': 'https://ndownloader.figshare.com/files/44488754',
        'source_sha256': SHA256,
        'documentation': 'https://www.ars.usda.gov/arsuserfiles/80400530/pdf/retn06.pdf',
        'catalogue': 'https://catalog.data.gov/dataset/usda-table-of-nutrient-retention-factors-release-6-2007',
        'license': 'CC0-1.0',
        'scope': 'Preparation estimates, reported in steps of 5 percent, capped at 100; not confidence intervals or storage-life certification. Source nutrient identifiers are preserved; no implicit mapping of folate or vitamin A definitions.',
        'rows': rows,
    }

if __name__ == '__main__':
    destination = Path(sys.argv[2]) if len(sys.argv)>2 else ROOT/'research/food-retention.json'
    destination.write_text(json.dumps(extract(sys.argv[1]),ensure_ascii=False,indent=2)+'\n')
