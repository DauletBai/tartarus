#!/usr/bin/env python3
"""Derive static brand assets from the canonical SVG lines; standard library only."""
import html
import math
import re
from pathlib import Path
import struct
import tomllib
import xml.etree.ElementTree as ET
import zlib

ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / 'assets/brand'

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

def png(size, lines, colors):
    """Render alternating over/under channels with subtle cylindrical shading.

    Geometry remains canonical. Only raster derivatives use this woven surface;
    the accepted animated SVG and its movement remain unchanged.
    """
    radius = 45*.85/2
    geometry = []
    for x1, y1, x2, y2 in lines:
        dx, dy = x2-x1, y2-y1
        length = math.hypot(dx, dy)
        geometry.append((x1, y1, dx, dy, length, length*length))
    # Sort actual offsets, so alternating depth follows spatial neighbours,
    # not the original SVG's central/outer serialization order.
    a_order = sorted(range(3), key=lambda i: lines[i][1]-lines[i][0])
    b_order = sorted(range(3, 6), key=lambda i: lines[i][1]+lines[i][0])
    above = {}
    for ai, i in enumerate(a_order):
        for bj, j in enumerate(b_order):
            above[i, j] = i if (ai+bj) % 2 == 0 else j
            above[j, i] = above[i, j]
    rows = bytearray()
    for y in range(size):
        rows.append(0)
        for x in range(size):
            count = 0
            channels = [0., 0., 0.]
            for sy in range(4):
                py = (y+(sy+.5)/4)*400/size
                for sx in range(4):
                    px = (x+(sx+.5)/4)*400/size
                    distances, sides, hits = [], [], []
                    for i, (x1,y1,dx,dy,length,length2) in enumerate(geometry):
                        t = max(0, min(1, ((px-x1)*dx+(py-y1)*dy)/length2))
                        distance = math.hypot(px-x1-t*dx, py-y1-t*dy)
                        distances.append(distance)
                        sides.append(((px-x1)*(-dy)+(py-y1)*dx)/length)
                        if distance <= radius:
                            hits.append(i)
                    if not hits:
                        continue
                    top = hits[0] if len(hits) == 1 else above[hits[0], hits[1]]
                    side = sides[top]/radius
                    bulge = math.sqrt(max(0, 1-(distances[top]/radius)**2))
                    shade = .64 + .36*bulge
                    highlight = .09*math.exp(-((side+.28)/.32)**2)*bulge
                    shadow = 1.
                    for other in range(6):
                        if (top, other) in above and above[top, other] == other:
                            gap = max(0, distances[other]-radius)
                            shadow *= 1-.48*math.exp(-(gap/4.5)**2)
                    count += 1
                    for c in range(3):
                        color = colors[top][c]*shade
                        color += (255-color)*highlight
                        channels[c] += color*shadow
            rows.extend((*(round(v/count) if count else 0 for v in channels), round(255*count/16)))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',size,size,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows,9))+chunk(b'IEND',b'')

def badge(name,label,value,color):
    left=math.ceil(len(label)*7.2+18);right=math.ceil(len(value)*7.2+18)
    def text(x,t):
        return f'<text x="{x}" y="16" fill="white" font-family="Verdana,DejaVu Sans,sans-serif" font-size="11" text-anchor="middle">{html.escape(t)}</text>'
    svg=f'<svg xmlns="http://www.w3.org/2000/svg" width="{left+right}" height="24" role="img" aria-label="{html.escape(label+": "+value)}"><rect width="{left+right}" height="24" rx="4" fill="{color}"/><path d="M4 0H{left}V24H4Q0 24 0 20V4Q0 0 4 0" fill="#374151"/>{text(left/2,label)}{text(left+right/2,value)}</svg>\n'
    (BRAND/'badges'/f'{name}.svg').write_text(svg)

def main():
    root=ET.parse(BRAND/'qazna.svg').getroot()
    palette = dict(re.findall(r'--qazna-(bright|deep):\s*(#[0-9a-fA-F]{6})', root.get('style', '')))
    if set(palette) != {'bright', 'deep'}: raise SystemExit('Canonical two-color palette missing')
    hexes = [palette[k] for k in ('bright', 'deep', 'deep', 'bright', 'deep', 'deep')]
    colors = [tuple(int(h[i:i+2],16) for i in (1,3,5)) for h in hexes]
    raw=[e for e in root.iter('{http://www.w3.org/2000/svg}line') if e.get('class')=='stroke']
    lines=[tuple(float(e.get(a))*.85+30 for a in ['x1','y1','x2','y2']) for e in raw]
    if len(lines)!=6:raise SystemExit('Expected the six canonical shanyrak strokes')
    svg='<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 400"><title>Qazna</title><g fill="none" stroke-width="38.25" stroke-linecap="round">'
    svg+=''.join(f'<line stroke="{color}" x1="{a}" y1="{b}" x2="{c}" y2="{d}"/>' for (a,b,c,d),color in zip(lines,hexes))+'</g></svg>\n'
    (BRAND/'favicon.svg').write_text(svg)
    images={n:png(n,lines,colors) for n in [16,32,48,180,192,512]}
    for n,name in [(32,'favicon-32.png'),(180,'apple-touch-icon.png'),(192,'icon-192.png'),(512,'icon-512.png')]:
        (BRAND/name).write_bytes(images[n])
    offset=6+16*3;entries=[];data=[]
    for n in [16,32,48]:
        image=images[n];entries.append(struct.pack('<BBBBHHII',n,n,0,0,1,32,len(image),offset));data.append(image);offset+=len(image)
    (BRAND/'favicon.ico').write_bytes(struct.pack('<HHH',0,1,3)+b''.join(entries)+b''.join(data))
    cargo=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']
    lock=tomllib.loads((ROOT/'Cargo.lock').read_text())['package']
    versions={p['name']:p['version'] for p in lock}
    for name,label,value,color in [
        ('version','crate',cargo['version'],'#b91c1c'),('stage','stage','research prototype','#a16207'),
        ('rust','Rust MSRV',cargo['rust-version'],'#a54413'),('edition','edition',cargo['edition'],'#a54413'),
        ('tokio','Tokio',versions['tokio'],'#6d28d9'),('axum','Axum',versions['axum'],'#6d28d9'),
        ('sqlx','SQLx',versions['sqlx'],'#0369a1'),('postgresql','PostgreSQL','16 tested','#0369a1'),
        ('python','Python','3.11+ tooling','#0369a1'),('license','code','AGPL-3.0+','#166534')]:
        badge(name,label,value,color)
    print('Generated canonical favicon variants and 10 local badges')

if __name__=='__main__':main()
