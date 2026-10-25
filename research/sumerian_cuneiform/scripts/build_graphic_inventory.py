#!/usr/bin/env python3
"""Index acquired images; never equate image count, graph count and letter count."""
import collections
import csv
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess

ROOT=Path(__file__).resolve().parents[1]

def main():
    with (ROOT/'SOURCE_REGISTER.csv').open(encoding='utf-8-sig',newline='') as f:
        register={r['local_file']:r for r in csv.DictReader(f)}
    images=[]
    for p in sorted((ROOT/'sources').iterdir()):
        if p.suffix.lower() not in {'.jpg','.jpeg','.png','.tif','.tiff'}:continue
        rel=p.relative_to(ROOT).as_posix();source=register.get(rel,{})
        dims=subprocess.check_output(['sips','-g','pixelWidth','-g','pixelHeight',str(p)],stderr=subprocess.DEVNULL).decode()
        width=re.search(r'pixelWidth:\s*(\d+)',dims);height=re.search(r'pixelHeight:\s*(\d+)',dims)
        assert width and height,p
        drawing=('lineart' in p.name or 'Langdon1919_plate' in p.name
                 or 'drawing' in source.get('type',''))
        photo=('photo' in p.name or p.name.startswith(('Louvre_','Met'))
               or 'photograph' in source.get('type',''))
        kind='editorial_drawing_or_plate' if drawing else 'photograph_or_photographic_composite' if photo else 'unclassified'
        sha=hashlib.sha256(p.read_bytes()).hexdigest()
        if source:assert sha==source['sha256'],p
        images.append(dict(image_id=f'IMG{len(images)+1:04d}',local_file=rel,kind=kind,
                           width=int(width[1]),height=int(height[1]),sha256=sha,
                           source_url=source.get('source_url',''),source_type=source.get('type',''),
                           source_notes=source.get('rights_and_notes',''),
                           coverage='acquired source file only; no claim of complete surface or independent object',
                           sign_segmentation_status='not exhaustively segmented'))
    with (ROOT/'data/graphic_image_inventory.csv').open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(images[0]));w.writeheader();w.writerows(images)
    # Existing G labels are local observations, not a universal alphabet.
    observations=[]
    ten=json.loads((ROOT/'data/round10_visual_before_text.json').read_text())
    for g in ten['graphic_pairs']:
        observations.append(dict(group_id='R10-P499399-'+g['id'],source_record='data/round10_visual_before_text.json',
                                 source_image='sources/P499399_photo.jpg',description=g['result'],
                                 unit='pair of tentative graphic complexes',canonical_identity='unresolved',
                                 phonetic_value='',meaning='',cross_object_equivalence='not established'))
    twelve=json.loads((ROOT/'data/round12_visual_before_text.json').read_text())
    for g in twelve['neutral_graphic_families']:
        observations.append(dict(group_id='R12-Met327384-'+g['id'],source_record='data/round12_visual_before_text.json',
                                 source_image='sources/Met327384_DP297593.jpg | sources/Met327384_DP297601.jpg',
                                 description=g['description'],unit='tentative family, possibly several distinct signs',
                                 canonical_identity='unresolved',phonetic_value='',meaning='',cross_object_equivalence='not established'))
    later=ROOT/'data/round14_manual_annotations.json'
    if later.exists():
        for v in json.loads(later.read_text())['views']:
            for g in v['candidates']:
                observations.append(dict(group_id='R14-'+g['id'],source_record='data/round14_manual_annotations.json',
                                         source_image=v['image'],description=g['description'],unit=g['unit'],
                                         canonical_identity='unresolved',phonetic_value='',meaning='',
                                         cross_object_equivalence='candidate graphic family only; no canonical merge'))
    with (ROOT/'data/graphic_group_inventory.csv').open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(observations[0]));w.writeheader();w.writerows(observations)
    counts=collections.Counter(r['kind'] for r in images)
    summary=dict(date='2026-10-25',source_images=len(images),by_kind=dict(counts),
                 distinct_image_hashes=len({r['sha256'] for r in images}),
                 local_observation_groups=len(observations),
                 exhaustive_sign_count=None,alphabet_size=None,independent_phonetic_values=0,
                 scope='All image files currently in sources/, not all available cuneiform worldwide.',
                 cautions=['One photo can show several objects, several photos can show the same object.',
                           'An impression is not automatically a sign; a sign is not automatically a letter.',
                           'Image dimensions do not guarantee that a particular line is legible.',
                           'Local group labels were not silently merged across stages or periods.'])
    (ROOT/'data/graphic_inventory_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    parts=['''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Инвентарь изображений и графических групп</title><style>body{font:17px/1.6 system-ui;max-width:1120px;margin:auto;padding:28px 20px;background:#f5f2eb;color:#233740}a{color:#155a70}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:18px}figure{background:white;margin:0;padding:14px}img{width:100%;height:240px;object-fit:contain}figcaption{font-size:14px;overflow-wrap:anywhere}table{border-collapse:collapse;width:100%;font-size:14px;background:white}td,th{padding:10px;border:1px solid #c8d1d0;text-align:left;vertical-align:top}</style><h1>Инвентарь до фонетического чтения</h1><p>25 октября2026. Это каталог приобретённых исходных изображений и ранее записанных графических наблюдений. Он не является восстановленным алфавитом.</p>''',
           '<p>Изображений: '+str(len(images))+'. Фото/фотокомпозитов: '+str(counts['photograph_or_photographic_composite'])+'. Прорисовок/табличных листов: '+str(counts['editorial_drawing_or_plate'])+'. Исчерпывающего числа знаков пока нет.</p>',
           '<p><a href="observations/2026-10-25_round13_inventory_method.txt">Метод и порядок работы</a> · <a href="data/graphic_image_inventory.csv">Каталог изображений</a> · <a href="data/graphic_group_inventory.csv">Локальные графические группы</a></p><h2>Группы пока не объединены в единый набор знаков</h2><table><tr><th>ID</th><th>Наблюдение</th><th>Единица</th></tr>']
    for g in observations:parts.append('<tr><td>'+html.escape(g['group_id'])+'</td><td>'+html.escape(g['description'])+'</td><td>'+html.escape(g['unit'])+'</td></tr>')
    parts.append('</table><h2>Исходные изображения</h2><div class="grid">')
    for r in images:
        rel=html.escape(r['local_file'],quote=True)
        parts.append(f'<figure><a href="{rel}"><img loading="lazy" src="{rel}" alt="{r["image_id"]}"></a><figcaption><strong>{r["image_id"]}</strong> · {html.escape(r["kind"])}<br>{html.escape(r["local_file"])}<br>{r["width"]} × {r["height"]} пикселей</figcaption></figure>')
    parts.append('</div><p>Книга не изменялась. Прорисовки и шрифтовые знаки не подменяют фотографии.</p></html>')
    (ROOT/'graphic_inventory.html').write_text(''.join(parts))
    print(json.dumps(summary,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
