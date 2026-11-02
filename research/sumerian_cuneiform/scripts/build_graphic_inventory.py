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
        geometry='3D_derived' in source.get('type','')
        kind='published_3D_derived_render' if geometry else 'editorial_drawing_or_plate' if drawing else 'photograph_or_photographic_composite' if photo else 'unclassified'
        sha=hashlib.sha256(p.read_bytes()).hexdigest()
        if source:assert sha==source['sha256'],p
        images.append(dict(image_id=f'IMG{len(images)+1:04d}',local_file=rel,kind=kind,
                           width=int(width[1]),height=int(height[1]),sha256=sha,
                           source_url=source.get('source_url',''),source_type=source.get('type',''),
                           source_notes=source.get('rights_and_notes',''),
                           coverage='acquired source file only; no claim of complete surface or independent object',
                           sign_segmentation_status='not exhaustively segmented'))
    with (ROOT/'data/graphic_image_inventory.csv').open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(images[0]),lineterminator="\n");w.writeheader();w.writerows(images)
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
    audit_path=ROOT/'data/graphic_visual_audit.json'
    if audit_path.exists():
        audit=json.loads(audit_path.read_text())
        for family in ('G01','G02','G03'):
            family_obs=[o for o in audit['observations'] if o['family']==family]
            observations.append(dict(group_id='V20261101-'+family,source_record='data/graphic_visual_audit.json',
                                     source_image=' | '.join(sorted({o['source'] for o in family_obs})),
                                     description=family_obs[0]['description'],unit='manually proposed graphic family',
                                     canonical_identity='unresolved',phonetic_value='',meaning='',
                                     cross_object_equivalence='not established; coordinates and limitations in source record'))
    with (ROOT/'data/graphic_group_inventory.csv').open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(observations[0]),lineterminator="\n");w.writeheader();w.writerows(observations)
    counts=collections.Counter(r['kind'] for r in images)
    summary_path=ROOT/'data/graphic_inventory_summary.json'
    previous=json.loads(summary_path.read_text()) if summary_path.exists() else {}
    summary=dict(date='2026-11-01',source_images=len(images),by_kind=dict(counts),
                 distinct_image_hashes=len({r['sha256'] for r in images}),
                 local_observation_groups=len(observations),
                 exhaustive_sign_count=None,alphabet_size=None,independent_phonetic_values=0,
                 scope='All image files currently in sources/, not all available cuneiform worldwide.',
                 cautions=['One photo can show several objects, several photos can show the same object.',
                           'An impression is not automatically a sign; a sign is not automatically a letter.',
                           'Image dimensions do not guarantee that a particular line is legible.',
                           'Local group labels were not silently merged across stages or periods.'])
    for key in ('visual_audit','geometry_controls','post_freeze_corrections','provenance_audit'):
        if key in previous:summary[key]=previous[key]
    summary_path.write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    parts=['''<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Инвентарь изображений и графических групп</title><style>body{font:17px/1.6 system-ui;max-width:1120px;margin:auto;padding:28px 20px;background:#f5f2eb;color:#233740}a{color:#155a70}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:18px}figure{background:white;margin:0;padding:14px}img{width:100%;height:240px;object-fit:contain}figcaption{font-size:14px;overflow-wrap:anywhere}table{border-collapse:collapse;width:100%;font-size:14px;background:white}td,th{padding:10px;border:1px solid #c8d1d0;text-align:left;vertical-align:top}</style><h1>Инвентарь до фонетического чтения</h1><p>Обновлено 1 ноября 2026. Это каталог приобретённых исходных изображений и ранее записанных графических наблюдений. Он не является восстановленным алфавитом.</p>''',
           '<p>Изображений: '+str(len(images))+'. Фото/фотокомпозитов: '+str(counts['photograph_or_photographic_composite'])+'. Прорисовок/табличных листов: '+str(counts['editorial_drawing_or_plate'])+'. Изображений из 3D-моделей: '+str(counts['published_3D_derived_render'])+'. Исчерпывающего числа знаков пока нет.</p>',
           '<p><a href="observations/2026-10-25_round13_inventory_method.txt">Метод и порядок работы</a> · <a href="data/graphic_image_inventory.csv">Каталог изображений</a> · <a href="data/graphic_group_inventory.csv">Локальные графические группы</a></p><h2>Группы пока не объединены в единый набор знаков</h2><table><tr><th>ID</th><th>Наблюдение</th><th>Единица</th></tr>']
    for g in observations:parts.append('<tr><td>'+html.escape(g['group_id'])+'</td><td>'+html.escape(g['description'])+'</td><td>'+html.escape(g['unit'])+'</td></tr>')
    parts.append('</table>')
    provenance_path=ROOT/'data/provenance_audit.csv'
    if provenance_path.exists():
        with provenance_path.open(encoding='utf-8-sig',newline='') as f:
            provenance=list(csv.DictReader(f))
        labels={'priority_original':'Приоритетный оригинал',
                'graphics_only':'Только графический контроль',
                'geometry_only':'Только контроль отображения рельефа',
                'provenance_hold':'Происхождение требует проверки',
                'negative_control':'Контроль известного вмешательства',
                'quarantine':'Карантин: подозрение не разрешено',
                'comparison_only':'Сравнительный источник',
                'editorial_comparison':'Редакторский композит, не оригинал'}
        parts.append('<section id="provenance"><h2>Происхождение и подлинность источников</h2>'
                     '<p><strong>Техническое сходство изображений не удостоверяет древность надписи.</strong> '
                     'В таблице отдельные предметы, группы и композит: число записей не равно числу артефактов. '
                     'Это проверка документов и опубликованных экспертиз, не собственная лабораторная экспертиза. '
                     'Приоритет оригинала означает подтверждённую каталогом связь с раскопками. '
                     'Объекты вне таблицы пока не прошли эту проверку.</p>'
                     '<p><a href="data/provenance_audit.csv">Полная таблица с источниками и ограничениями</a> · '
                     '<a href="synthesis/conclusions_for_book.txt">Накопительная исследовательская сводка</a></p>'
                     '<table><tr><th>Предмет / свидетель</th><th>Допуск</th><th>Основание</th></tr>')
        for row in provenance:
            parts.append('<tr><td>'+html.escape(row['object_or_witness'])+'</td><td>'+
                         html.escape(labels[row['research_decision']])+'</td><td>'+
                         html.escape(row['physical_status'])+'<br>'+html.escape(row['provenance_evidence'])+'</td></tr>')
        parts.append('</table></section>')
    gudea_path=ROOT/'data/gudea_surface_collation.json'
    if gudea_path.exists():
        gudea=json.loads(gudea_path.read_text())
        parts.append('<section id="gudea"><h2>Гудеа H: проверка повторяющегося начала</h2>'
                     '<p>Три участка одного оригинала AO4. Выбор выполнен с обращением к изданию: '
                     'это целевая колляция, а не слепая дешифровка. Звуки не восстановлены независимо.</p>'
                     '<p>'+html.escape(gudea['result'])+'</p>'
                     '<p><a href="data/gudea_surface_collation.json">Координаты и ограничения</a> · '
                     '<a href="observations/2026-10-26_round35_photographic_collation.txt">Исследовательская запись</a></p>'
                     '<img src="data/gudea_surface_comparison.png" style="max-width:100%;height:auto" '
                     'alt="Три фрагмента оригинального фото AO4, повёрнутые на 90 градусов против часовой стрелки">'
                     '<p>Производное сопоставление. Права музейной фотографии сохраняются; '
                     'разрешение на коммерческую публикацию не установлено.</p></section>')
    parts.append('<h2>Исходные изображения</h2><div class="grid">')
    for r in images:
        rel=html.escape(r['local_file'],quote=True)
        parts.append(f'<figure><a href="{rel}"><img loading="lazy" src="{rel}" alt="{r["image_id"]}"></a><figcaption><strong>{r["image_id"]}</strong> · {html.escape(r["kind"])}<br>{html.escape(r["local_file"])}<br>{r["width"]} × {r["height"]} пикселей</figcaption></figure>')
    parts.append('</div><p>Книга не изменялась. Прорисовки и шрифтовые знаки не подменяют фотографии.</p></html>')
    (ROOT/'graphic_inventory.html').write_text(''.join(parts))
    if (ROOT/'data/graphic_visual_audit.json').exists():visual_audit()
    print(json.dumps({k:v for k,v in summary.items() if k not in ('visual_audit','geometry_controls')},ensure_ascii=False,indent=2))

def visual_audit():
    """Compare pre-recorded image rectangles without sign names or readings.

    This tests similarity of selected crops, not whole-image recognition,
    decipherment, or independent observers. Requires Pillow and NumPy.
    """
    import base64
    import io
    import numpy as np
    from PIL import Image, ImageDraw, ImageOps

    record_path = ROOT / 'data/graphic_visual_audit.json'
    record_bytes = record_path.read_bytes()
    record = json.loads(record_bytes)
    obs = record['observations']
    by_id = {o['id']: o for o in obs}
    photos = {}
    crops = {}
    for o in obs:
        p = ROOT / o['source']
        if o['source'] not in photos:
            photos[o['source']] = Image.open(p).convert('RGB')
        im = photos[o['source']]
        x0, y0, x1, y1 = o['bbox_xyxy']
        assert 0 <= x0 < x1 <= im.width and 0 <= y0 < y1 <= im.height, o['id']
        crops[o['id']] = im.crop((x0, y0, x1, y1))

    def feature(im, mode):
        a = np.asarray(im.convert('L').resize((64,64), Image.Resampling.BILINEAR), dtype=float)
        if mode == 'gradient_magnitude':
            gy, gx = np.gradient(a)
            a = np.hypot(gx, gy)
        return a

    def score(a,b,trim):
        if trim:
            a,b=a[trim:-trim,trim:-trim],b[trim:-trim,trim:-trim]
        h,w=a.shape
        best=-1.0
        for dx in (-4,0,4):
            for dy in (-4,0,4):
                aa=a[max(0,dy):min(h,h+dy),max(0,dx):min(w,w+dx)].ravel()
                bb=b[max(0,-dy):min(h,h-dy),max(0,-dx):min(w,w-dx)].ravel()
                aa=aa-aa.mean();bb=bb-bb.mean()
                den=float(np.linalg.norm(aa)*np.linalg.norm(bb))
                value=float(np.dot(aa,bb)/den) if den else 0.0
                best=max(best,value)
        return best

    rows=[]
    for mode in ('gray','gradient_magnitude'):
        arrays={k:feature(v,mode) for k,v in crops.items()}
        for trim in (0,3):
            for o in obs:
                if o['role']=='template':continue
                scores={t:score(arrays[t],arrays[o['id']],trim) for t in record['predeclared_computational_test']['templates']}
                winner=max(scores,key=scores.get)
                rows.append(dict(id=o['id'],mode=mode,trim=trim,scores=scores,
                                 nearest_template=winner,expected_family=o['family'],
                                 nearest_family=by_id[winner]['family'],
                                 agreement_with_manual_group=(by_id[winner]['family']==o['family']) if o['family'] else None))
    result=dict(date='2026-11-01',annotation_sha256=hashlib.sha256(record_bytes).hexdigest(),
                observed_rectangles=len(obs),candidate_occurrences=sum(o['family'] is not None for o in obs),
                manually_defined_families=3,objects_with_rectangles=2,
                records=rows,limitations=record['limitations'],
                interpretation='Scores measure image similarity only. There is no semantic or phonetic output.')
    result['by_configuration']=[]
    for mode in ('gray','gradient_magnitude'):
        for trim in (0,3):
            rs=[r for r in rows if r['mode']==mode and r['trim']==trim and r['expected_family']]
            result['by_configuration'].append(dict(mode=mode,trim=trim,n=len(rs),matching=sum(r['agreement_with_manual_group'] for r in rs)))
    summary_path=ROOT/'data/graphic_inventory_summary.json'
    summary=json.loads(summary_path.read_text()) if summary_path.exists() else {}
    summary['visual_audit']=result
    summary_path.write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')

    def data_image(im):
        b=io.BytesIO();im.save(b,format='PNG')
        return 'data:image/png;base64,'+base64.b64encode(b.getvalue()).decode()

    parts=['<!-- visual-audit:start --><section id="visual-audit"><h1>Проверка по фотографиям · 1 ноября 2026</h1>',
           '<p>7 музейных записей; 14 кандидатов в 3 графических семьях на 2 предметах; 8 контрольных участков. Один кандидат неуверенный. Это выбранные области, а не полный инвентарь знаков.</p>',
           '<p>Фонетика и перевод не назначены. K — калибровочный предмет с заранее известным описанием. На V1–V3 при первом просмотре раскрылись прорисовки; полное ослепление не заявляется.</p>',
           '<p>Уточнение протокола после фиксации: у K приобретены 2 фотографии и 2 современные прорисовки, а не 4 фотографии. Прорисовки K просмотрены после числового теста; в расчёте использован только снимок view1. Даты по музейному каталогу: K — около XX–XVI вв. до н. э.; V1–V6 — позднее I тысячелетие до н. э. Разновременные группы не объединены в один алфавит.</p>',
           '<p><a href="data/graphic_visual_audit.json">Фиксированные наблюдения и координаты</a> · <a href="data/graphic_inventory_summary.json">Полные числовые результаты</a></p>',
           '<h2>Размеченные фотографии</h2><div class="grid">']
    colors={'G01':'#ef4040','G02':'#2484ef','G03':'#159447',None:'#cc9600'}
    for source,im in photos.items():
        shown=im.copy();d=ImageDraw.Draw(shown)
        for o in obs:
            if o['source']==source:
                d.rectangle(o['bbox_xyxy'],outline=colors[o['family']],width=max(2,im.width//500))
                d.text((o['bbox_xyxy'][0],o['bbox_xyxy'][1]),o['id'],fill=colors[o['family']])
        shown=ImageOps.contain(shown,(1200,1200))
        parts.append('<figure><a href="'+html.escape(source)+'"><img style="height:auto" src="'+data_image(shown)+'"></a><figcaption>'+html.escape(source)+'</figcaption></figure>')
    parts.append('</div><h2>Фрагменты до назначения звучания</h2><div class="grid">')
    for o in obs:
        parts.append('<figure><img src="'+data_image(crops[o['id']])+'"><figcaption><strong>'+o['id']+' · '+str(o['family'] or 'контроль')+'</strong><br>'+str(o['bbox_xyxy'])+'<br>'+html.escape(o['description'])+'<br>'+o['confidence']+'</figcaption></figure>')
    parts.append('</div><h2>Проверка сходства заранее выделенных фрагментов</h2><p>Шаблоны K1a, K2a, V2a; размер 64 × 64; девять сдвигов; без зеркального отражения и вращения. Правильность здесь означает согласие с нашей визуальной группировкой, а не доказанную идентичность древнего знака.</p><table><tr><th>Признак</th><th>Обрезка края</th><th>Согласие</th></tr>')
    for r in result['by_configuration']:
        parts.append(f'<tr><td>{r["mode"]}</td><td>{r["trim"]} px</td><td>{r["matching"]} / {r["n"]}</td></tr>')
    parts.append('</table><p>Контрольные участки также получают ближайший шаблон: поэтому само максимальное сходство не является распознаванием. Не задан порог принятия, обученный на независимом корпусе; классификация контролей как знаков запрещена. Для серого изображения контроль C08 (0,274) превосходит один из кандидатов G02 (0,273): единый порог не разделяет все выбранные примеры. G03 находится только на другом снимке: возможен эффект освещения, материала и руки писца.</p>')
    geometry=summary.get('geometry_controls',{})
    if geometry:
        parts.append('<h2>Один рельеф при двух способах отображения</h2><p>Опубликованные изображения, полученные из 3D-моделей, а не наши сканы: MaiCuBeDa Hilprecht, Hubert Mara и Timo Homburg, <a href="https://doi.org/10.11588/data/QSNIQ2">DOI 10.11588/data/QSNIQ2</a>, CC BY-SA 4.0. Границы строк заданы авторами набора. Звуковые и смысловые метки не загружались; язык этих трёх HS-надписей на этом этапе не установлен. Два отображения одного предмета не считаются двумя свидетельствами.</p><div class="grid">')
        for r in sorted(geometry['images'],key=lambda x:(x['member'],x['render'])):
            parts.append('<figure><a href="'+html.escape(r['file'])+'"><img style="height:auto" src="'+html.escape(r['file'])+'"></a><figcaption>'+html.escape(r['member']+' · '+r['render'])+'</figcaption></figure>')
        parts.append('</div>')
        if 'comparison' in geometry:
            parts.append('<p>Контроль устойчивости: каждая строка разделена на три равные полосы; это не границы знаков. При сопоставлении 9 полос между двумя отображениями:</p><ul>')
            for r in geometry['comparison']['configurations']:
                parts.append('<li>'+html.escape(r['direction']+'; '+r['feature'])+': '+str(r['top1_correct'])+' / 9 соответствующих полос оказались ближайшими.</li>')
            parts.append('</ul><p>Этот результат проверяет устойчивость представления поверхности. Он не определяет число клиньев, значения знаков или язык надписи.</p>')
    parts.append('</section><!-- visual-audit:end --><hr>')
    target=ROOT/'graphic_inventory.html'
    current=target.read_text()
    current=re.sub(r'<!-- visual-audit:start -->.*?<!-- visual-audit:end -->(?:<hr>)*','',current,flags=re.S)
    index=current.index('<h1>')
    current=current[:index]+''.join(parts)+current[index:]
    target.write_text(current)
    # This diagnostic montage is temporary; original crops remain reproducible by coordinates.
    sheet=Image.new('RGB',(1200,((len(obs)+3)//4)*240),'#eeeeee')
    draw=ImageDraw.Draw(sheet)
    for i,o in enumerate(obs):
        x=i%4*300;y=i//4*240
        crop=ImageOps.contain(crops[o['id']],(280,200))
        sheet.paste(crop,(x,y+30));draw.text((x+5,y+5),o['id']+' '+str(o['family']),fill='black')
    sheet.save('/private/tmp/tartarus_visual_audit_montage.png')
    print(json.dumps(result['by_configuration'],indent=2))


def compare_geometry_controls():
    """Test a fixed representation control; these bands are not segmented signs."""
    import numpy as np
    from PIL import Image
    sp=ROOT/'data/graphic_inventory_summary.json'
    summary=json.loads(sp.read_text())
    controls=summary['geometry_controls']
    plan=controls['predeclared_comparison']
    assert hashlib.sha256(json.dumps(plan,sort_keys=True).encode()).hexdigest()==controls['comparison_plan_sha256']
    features={}
    ids=[]
    bands=[]
    for r in sorted(controls['images'],key=lambda x:(x['member'],x['render'])):
        p=ROOT/r['file']
        assert hashlib.sha256(p.read_bytes()).hexdigest()==r['sha256']
        im=Image.open(p).convert('L')
        for i in range(3):
            key=r['member']+':third'+str(i+1)
            if key not in ids:ids.append(key)
            bbox=[im.width*i//3,0,im.width*(i+1)//3,im.height]
            a=np.asarray(im.crop(bbox).resize((64,64),Image.Resampling.BILINEAR),dtype=float)
            gy,gx=np.gradient(a)
            features[(key,r['render'],'gray')]=a
            features[(key,r['render'],'gradient_magnitude')]=np.hypot(gx,gy)
            bands.append(dict(id=key,render=r['render'],source=r['file'],bbox_xyxy=bbox))
    configurations=[]
    for mode in plan['features']:
        for left,right in [('virtual','msii'),('msii','virtual')]:
            matrix=[];ranks=[]
            for key in ids:
                a=features[(key,left,mode)].ravel();a=a-a.mean()
                scores=[]
                for other in ids:
                    b=features[(other,right,mode)].ravel();b=b-b.mean()
                    scores.append(float(np.dot(a,b)/(np.linalg.norm(a)*np.linalg.norm(b))))
                matrix.append(scores)
                ranks.append(sorted(range(9),key=lambda j:scores[j],reverse=True).index(ids.index(key))+1)
            configurations.append(dict(feature=mode,direction=left+' -> '+right,
                                       top1_correct=sum(x==1 for x in ranks),ranks=ranks,
                                       correlation_matrix=matrix))
    controls['comparison']=dict(ids=ids,bands=bands,configurations=configurations,
                                limits=plan['limitation'],interpretation=plan['outcome'])
    sp.write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps([{k:v for k,v in r.items() if k!='correlation_matrix'} for r in configurations],indent=2))


def acquire_geometry_controls():
    """Range-extract declared line images; do not load transliterations/sign labels."""
    import struct
    import urllib.request
    import zlib
    endpoint='https://heidata.uni-heidelberg.de/api/access/datafile/'
    requested=['line/line_10_HS_0093_03_front.png',
               'line/line_10_HS_1032_06_back.png',
               'line/line_10_HS_1059_03_front.png']
    results=[]
    def fetch_range(url,interval):
        request=urllib.request.Request(url,headers={'Range':'bytes='+interval})
        with urllib.request.urlopen(request,timeout=50) as r:
            if r.status!=206:raise RuntimeError('Server did not honor range; refusing full archive')
            return r.read()
    for dataset_id,render in [(12792,'virtual'),(12793,'msii')]:
        url=endpoint+str(dataset_id)
        tail=fetch_range(url,'-65536')
        p=tail.rfind(b'PK\x05\x06')
        if p<0:raise RuntimeError('No zip footer')
        eocd=struct.unpack_from('<4s4H2IH',tail,p)
        size,offset=eocd[5:7]
        if size==0xffffffff or offset==0xffffffff:
            raise RuntimeError('ZIP64 central offsets require separate parsing')
        central=fetch_range(url,f'{offset}-{offset+size-1}')
        assert len(central)==size
        entries={};i=0
        while i<len(central) and central[i:i+4]==b'PK\x01\x02':
            h=struct.unpack_from('<4s6H3I5H2I',central,i)
            n,e,c=h[10:13]
            name=central[i+46:i+46+n].decode('utf-8')
            if name in requested:entries[name]=h
            i+=46+n+e+c
        for name in requested:
            h=entries[name];off=h[16];compressed=h[8]
            local=fetch_range(url,f'{off}-{off+29}')
            assert local[:4]==b'PK\x03\x04'
            nn,ee=struct.unpack_from('<HH',local,26)
            start=off+30+nn+ee
            payload=fetch_range(url,f'{start}-{start+compressed-1}')
            content=zlib.decompress(payload,-15) if h[4]==8 else payload
            assert len(content)==h[9]
            assert (zlib.crc32(content)&0xffffffff)==h[7]
            stem=Path(name).stem
            rel='sources/MaiCuBeDa_'+stem+'_'+render+'.png'
            (ROOT/rel).write_bytes(content)
            results.append(dict(member=name,render=render,file=rel,source_url=url,
                                doi='10.11588/data/QSNIQ2',bytes=len(content),
                                sha256=hashlib.sha256(content).hexdigest(),crc32=h[7],
                                central_directory_sha256=hashlib.sha256(central).hexdigest(),
                                type='published_3D_derived_line_render_not_photograph',
                                limits='Line crop boundaries supplied by dataset editors; no sign labels or transliteration loaded. Two renders are the same surface, not independent witnesses.'))
    sp=ROOT/'data/graphic_inventory_summary.json'
    summary=json.loads(sp.read_text())
    summary['geometry_controls']=dict(date='2026-11-01',selection='First three PNG line members in VirtualLight archive order, three distinct HS catalogue numbers; declared before viewing.',images=results)
    sp.write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(results,indent=2))


if __name__=='__main__':
    import sys
    if '--acquire-geometry' in sys.argv:acquire_geometry_controls()
    elif '--compare-geometry' in sys.argv:compare_geometry_controls()
    elif '--visual-audit' in sys.argv:visual_audit()
    else:main()
