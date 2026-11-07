#!/usr/bin/env python3
"""Extract pinned USDA Foundation Foods values; never impute an absent value as zero.

Usage: python3 scripts/import_food_composition.py /path/to/download.zip
The supplied archive is read offline. No credentials or API access are needed.
"""
import hashlib
import json
from decimal import Decimal
from pathlib import Path
import sys
import zipfile

EXPECTED_SHA256 = "186e988ec542e913f51ef62b86a47758e8cdd0d1dc3889e7b055581f3c09c77a"
ROOT = Path(__file__).resolve().parents[1]
# Nutrient definition is part of the key, not just its physical unit.
NUTRIENTS = [
    ('energy_atwater_general',2047,'kcal'),('protein',1003,'g'),('fat',1004,'g'),
    ('carbohydrate_by_difference',1005,'g'),('fibre_total',1079,'g'),
    ('calcium',1087,'mg'),('iron',1089,'mg'),('magnesium',1090,'mg'),
    ('phosphorus',1091,'mg'),('potassium',1092,'mg'),('sodium',1093,'mg'),
    ('zinc',1095,'mg'),('copper',1098,'mg'),('selenium',1103,'µg'),
    ('iodine',1100,'µg'),('vitamin_a_RE',None,'µg RE'),
    ('vitamin_c',1162,'mg'),('vitamin_d_D2_D3',1114,'µg'),
    ('vitamin_e_alpha_tocopherol',1109,'mg'),('vitamin_k_phylloquinone',1185,'µg'),
    ('thiamin',1165,'mg'),('riboflavin',1166,'mg'),('vitamin_b6',1175,'mg'),
    ('vitamin_b12',1178,'µg'),('folate_DFE',1190,'µg DFE'),('choline',1180,'mg'),
    ('niacin_NE',None,'mg NE'),('biotin',1176,'µg'),('pantothenic_acid',1170,'mg'),
]
FOODS = [
    (2512380,'rice','grain'),(2346396,'oats','grain'),(2644283,'lentils','pulses'),
    (2646170,'chicken','meat'),(2646173,'beef','meat'),(2684441,'salmon','fish'),
    (2258586,'carrot','vegetables'),(1999633,'spinach','vegetables'),
    (1750339,'apple','fruit'),(746771,'orange','fruit'),(746782,'milk','dairy'),
    (748608,'olive_oil','oil_crops'),(2346394,'walnuts','nuts'),
]

def extract(path):
    raw = Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != EXPECTED_SHA256:
        raise ValueError("Archive differs from the reviewed April 2026 release")
    with zipfile.ZipFile(path) as archive:
        members = [n for n in archive.namelist() if n.endswith('.json')]
        if len(members) != 1:
            raise ValueError('Expected one JSON dataset')
        source = json.loads(archive.read(members[0]), parse_float=Decimal)
    index = {f['fdcId']:f for f in source['FoundationFoods'] if f}
    foods = []
    for fdc_id, key, sector in FOODS:
        food = index[fdc_id]
        values = {}
        for n in food['foodNutrients']:
            nid = n['nutrient']['id']
            if nid in values:
                raise ValueError(f'Duplicate nutrient {nid} in {fdc_id}')
            values[nid] = n
        cells = []
        for name, nid, unit in NUTRIENTS:
            n = values.get(nid)
            if n is not None and any('loq' in key.lower() for key in n):
                raise ValueError('LOQ-coded values need an explicit interval before import')
            amount = None if n is None else n.get('amount')
            origin = 'not_reported'
            source_amount = None
            source_unit = None
            if amount is not None:
                source_unit = n['nutrient']['unitName']
                # DFE remains a separate definition, despite USDA using the mass unit UG.
                expected = 'µg' if unit == 'µg DFE' else unit
                if source_unit.lower().replace('ug','µg') != expected.lower():
                    raise ValueError(f'Unit mismatch: {name} {source_unit}')
                amount = Decimal(str(amount))
                origin = 'USDA_published_value'
                source_amount = str(amount)
            elif name == 'energy_atwater_general':
                macros = [values.get(i,{}).get('amount') for i in (1003,1004,1005)]
                if all(v is not None for v in macros):
                    amount = sum(Decimal(str(v))*k for v,k in zip(macros,(4,9,4)))
                    origin = 'derived_4_protein_9_fat_4_carbohydrate_USDA_method'
            scaled = None if amount is None else amount*1_000_000
            if scaled is not None and (scaled < 0 or scaled != scaled.to_integral_value()):
                raise ValueError('Negative value or precision beyond declared resolution')
            cells.append({'nutrient':name,'unit':unit,'quanta_per_100g':None if scaled is None else int(scaled),
                          'usda_nutrient_id':nid,'origin':origin,'source_amount':source_amount,
                          'source_unit':source_unit})
        foods.append({'id':key,'sector':sector,'fdc_id':fdc_id,'description':food['description'],
                      'publication_date':food['publicationDate'],'cells':cells})
    return {'schema':1,'source_url':'https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_foundation_food_json_2026-04-30.zip',
            'source_sha256':hashlib.sha256(raw).hexdigest(),'source_member':members[0],
            'basis':'per 100 g edible portion, in the precise raw/dry/as-sold state described; not a cooked diet',
            'quanta_per_unit':1_000_000,'foods':foods}

if __name__ == '__main__':
    result = extract(sys.argv[1])
    (ROOT/'research/food-composition.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
