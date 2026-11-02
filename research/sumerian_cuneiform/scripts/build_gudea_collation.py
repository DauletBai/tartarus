#!/usr/bin/env python3
"""Reproduce selected AO4 crops. No reconstruction or sign recognition."""
from pathlib import Path
import hashlib
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
data = json.loads((ROOT / 'data/gudea_surface_collation.json').read_text())
source = ROOT / data['source']
assert hashlib.sha256(source.read_bytes()).hexdigest() == data['sha256']
original = Image.open(source).convert('RGB')
assert list(original.size) == data['original_size']
rotated = original.rotate(90, expand=True)
crops = []
for entry in data['observations']:
    box = entry['box']
    assert 0 <= box[0] < box[2] <= rotated.width
    assert 0 <= box[1] < box[3] <= rotated.height
    crop = rotated.crop(box)
    # Same factor for every crop; no separate aspect-ratio normalization.
    crops.append((entry, crop.resize((crop.width * 2, crop.height * 2))))
width = max(c.width for _, c in crops) + 40
height = sum(c.height + 48 for _, c in crops) + 60
sheet = Image.new('RGB', (width, height), 'white')
draw = ImageDraw.Draw(sheet)
draw.text((20, 12), 'AO4 / Louvre: selected fields, 90 deg CCW, 2x; not an independent decipherment', fill='black')
y = 46
for entry, crop in crops:
    draw.text((20, y), f"{entry['id']} | edition {entry['editorial_line']} | box {entry['box']}", fill='black')
    sheet.paste(crop, (20, y + 20))
    y += crop.height + 48
sheet.save(ROOT / 'data/gudea_surface_comparison.png')
print(f'Generated {width} x {height}; 3 crops, 1 ancient object')
