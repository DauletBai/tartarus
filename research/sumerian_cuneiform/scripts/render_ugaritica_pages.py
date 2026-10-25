#!/usr/bin/env python3
"""Render inspected source pages without modifying the archived publication."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
source = ROOT / 'sources/UgariticaV_1968.pdf'
assert source.stat().st_size == 80122241, 'Incomplete source download'
specifications = [(460, 444, True), (461, 445, True), (330, 314, False)]
records = []
with tempfile.TemporaryDirectory() as directory:
    for pdf_page, printed_page, rotate in specifications:
        prefix = Path(directory) / f'page{pdf_page}'
        subprocess.run(['pdftoppm', '-f', str(pdf_page), '-l', str(pdf_page),
                        '-scale-to', '2200', '-png', '-singlefile', str(source), str(prefix)], check=True)
        image = prefix.with_suffix('.png')
        destination = ROOT / f'data/round6_UgariticaV_printed{printed_page}.png'
        if rotate:
            subprocess.run(['sips', '-r', '90', str(image), '--out', str(destination)],
                           check=True, stdout=subprocess.DEVNULL)
        else:
            destination.write_bytes(image.read_bytes())
        records.append(dict(pdf_page_one_based=pdf_page, printed_page=printed_page,
                            clockwise_rotation=90 if rotate else 0,
                            local_file=str(destination.relative_to(ROOT)),
                            sha256=hashlib.sha256(destination.read_bytes()).hexdigest()))
(ROOT / 'data/round6_render_manifest.json').write_text(json.dumps(dict(
    source='sources/UgariticaV_1968.pdf',
    source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
    note='Research derivatives; original publication unchanged; repository restricts commercial use',
    pages=records), ensure_ascii=False, indent=2)+'\n')
print('Rendered 3 inspected source pages; manifest saved')
