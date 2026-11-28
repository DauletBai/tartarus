"""Extract the pinned official-source snapshot. Offline once source files are cached."""
import argparse
import csv
import gzip
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from zipfile import ZipFile
from urllib.request import urlopen
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = json.loads((ROOT / "fixtures/geography-sources.json").read_text())
# Local identifiers are stable project keys. Keep the original BNS names for audit.
NAMES = dict(line.split("|") for line in """Республика Казахстан|KAZ
Абай|abay
Акмолинская|akmola
Актюбинская|aktobe
Алматинская|almaty-region
Атырауская|atyrau
Западно-Казахстанская|west-kazakhstan
Жамбылская|zhambyl
Жетісу|zhetisu
Карагандинская|karaganda
Костанайская|kostanay
Кызылординская|kyzylorda
Мангистауская|mangystau
Павлодарская|pavlodar
Северо-Казахстанская|north-kazakhstan
Туркестанская|turkistan
Ұлытау|ulytau
Восточно-Казахстанская|east-kazakhstan
г.Астана|astana-city
г.Алматы|almaty-city
г.Шымкент|shymkent-city""".splitlines())


def numeric(token):
    return Decimal(token.replace(" ", "").replace(",", "."))


def block(text, first, last):
    return text.split(first, 1)[1].split(last, 1)[0]


def bns_rows(section, scale, column=-1):
    rows = {}
    for line in section.splitlines():
        cols = re.split(r"\s{2,}", line.strip())
        name = cols[0].replace("Мангыстауская", "Мангистауская")
        if name == "Павлодар":
            name = "Павлодарская"
        if name not in NAMES:
            continue
        assert len(cols) == 6 and NAMES[name] not in rows, (name, cols)
        token = cols[column]
        if token in {"x", "х", "X", "Х", "...", "…", "0,0"}:
            # Rounded negligible quantity is not an exact zero.
            value = None
        else:
            value = 0 if token == "-" else numeric(token) * scale
            assert int(value) == value
            value = int(value)
        rows[NAMES[name]] = value
    assert "KAZ" in rows
    return rows


def climate(text, start, end):
    section = block(text, start, end)
    # Table layout puts the area name on or beside its numeric row. The source
    # contains one second row per area with percentiles; those are excluded.
    aliases = {**NAMES, "Жетысу":"zhetisu", "Улытау":"ulytau", "Мангистауская1":"mangystau",
               "Казахстан": "KAZ", "Восточно-": "east-kazakhstan",
               "Западно-": "west-kazakhstan", "Северо-": "north-kazakhstan"}
    labels, records = [], []
    for pos, line in enumerate(section.splitlines()):
        tokens = line.strip().split()
        if not tokens:
            continue
        if tokens[0] in aliases:
            labels.append((pos, aliases[tokens.pop(0)]))
        if len(tokens) == 12 and all(re.fullmatch(r"[-−]?\d+([.,]\d+)?", t) for t in tokens):
            records.append((pos, [float(numeric(t.replace("−", "-"))) for t in tokens]))
    assert len(labels) == len(records) == 18
    result = {}
    for (i, key), (j, values) in zip(labels, records, strict=True):
        assert abs(i-j) <= 1 and key not in result
        result[key] = values
    return result


def copper_rows(text):
    countries = dict(line.split("|") for line in """United States|USA
Australia|AUS
Canada|CAN
Chile|CHL
China|CHN
Congo (Kinshasa)|COD
Germany|DEU
India|IND
Indonesia|IDN
Japan|JPN
Kazakhstan|KAZ
Korea, Republic of|KOR
Mexico|MEX
Peru|PER
Poland|POL
Russia|RUS
Zambia|ZMB""".splitlines())
    section = block(text, "Reserves6", "Other countries ")
    # Australia's reserve has a superscript 7 on a separate line in this edition.
    section = re.sub(r"(Australia[^\n]+?)\s+7\s*\n\s+(100,000)", r"\1  \2", section)
    result = {}
    for line in section.splitlines():
        cells = re.split(r"\s{2,}", line.strip())
        if cells[0] in countries:
            assert len(cells) == 6
            result[countries[cells[0]]] = [0 if x == "—" else int(x.replace(",", ""))*1000
                                           for x in (cells[2], cells[4], cells[5])]
    assert len(result) == len(countries)
    return result


def world(cache, copper):
    with gzip.open(cache/"wpp", "rt", encoding="utf-8-sig") as f:
        rows = [r for r in csv.DictReader(f) if r["LocTypeName"] in {"Country/Area", "World"}
                and 2026 <= int(r["Time"]) <= 2100 and r["Variant"] == "Medium"]
    def count(row):
        people = Decimal(row["TPopulation1July"]) * 1000
        assert people == int(people)
        return int(people)
    totals = {int(r["Time"]): count(r) for r in rows if r["LocTypeName"] == "World"}
    year = min(y for y,p in totals.items() if p >= 10**10)
    indicators = {}
    for key, field in [("cereals", "cereal_t_2024"), ("arable", "arable_ha_2023"),
                       ("freshwater", "internal_renewable_water_billion_m3_per_year_2022_entry"),
                       ("rain", "long_run_precip_mm_2022_entry")]:
        meta, data = json.loads((cache/key).read_text())
        assert meta["pages"] == 1 and meta["total"] == len(data)
        indicators[field] = {r["countryiso3code"]:r["value"] for r in data if r["countryiso3code"]}
    sdg = json.loads((cache/"renewables").read_text())
    assert sdg["totalPages"] == 1 and sdg["totalElements"] == len(sdg["data"])
    capacity = {}
    for r in sdg["data"]:
        if r["dimensions"] == {"Reporting Type":"G", "Type of renewable technology":"ALL"}:
            assert r["timePeriodStart"] == 2023 and r["attributes"]["Units"] == "W_PER_CAPITA"
            watts = float(r["value"])
            assert capacity.setdefault(r["geoAreaCode"], watts) == watts
    areas = {}
    for r in rows:
        y = int(r["Time"])
        if r["LocTypeName"] != "Country/Area" or y not in (2026, year-1, year):
            continue
        key = r["LocID"]
        iso = r["ISO3_code"]
        area = areas.setdefault(key, {"un_location_id":int(key), "iso3":iso, "name":r["Location"],
                                      "installed_renewables_w_per_person_2023":capacity.get(r["SDMX_code"]),
                                      **dict(zip(("copper_mined_t_2025_estimate", "copper_refined_t_2025_estimate",
                                                  "copper_reserves_t_mcs2026"), copper.get(iso, [None]*3), strict=True)),
                                      **{k:v.get(iso) for k,v in indicators.items()}, "july_population":{}})
        assert str(y) not in area["july_population"]
        area["july_population"][str(y)] = count(r)
    assert len(areas) == 237
    return {"july_world_population":{str(y):totals[y] for y in (2026,year-1,year)},
            "areas":sorted(areas.values(), key=lambda a:a["un_location_id"])}


def worksheet(path, number):
    with ZipFile(path) as source:
        strings = ["".join(entry.itertext()) for entry in ET.fromstring(source.read("xl/sharedStrings.xml"))]
        sheet = ET.fromstring(source.read(f"xl/worksheets/sheet{number}.xml"))
    for row in sheet.findall(".//{*}row"):
        cells = {}
        for cell in row:
            value = cell.findtext("{*}v")
            if value is not None:
                col = cell.get("r").rstrip("0123456789")
                cells[col] = strings[int(value)].strip() if cell.get("t") == "s" else value
        yield cells


def october(cache, regions):
    parent = None
    for cells in worksheet(cache/"districts", 5):
        if not cells.get("B", "").isdigit():
            continue
        name = cells["A"]
        counts = {f"{prefix}people_2026_10_01": (0 if cells[col] == "-" else int(cells[col]))
                  for prefix,col in (("", "B"), ("urban_", "E"), ("rural_", "H"))}
        assert counts["people_2026_10_01"] == counts["urban_people_2026_10_01"] + counts["rural_people_2026_10_01"]
        if name in NAMES:
            parent = NAMES[name]
            regions[parent].update(counts)
        else:
            assert parent and parent != "KAZ"
            districts = regions[parent].setdefault("districts", {})
            assert name not in districts
            districts[name] = counts
    for field in counts:
        assert sum(area[field] for key, area in regions.items() if key != "KAZ") == regions["KAZ"][field]
        for key, area in regions.items():
            if key != "KAZ":
                assert sum(d[field] for d in area["districts"].values()) == area[field]
    for cells in worksheet(cache/"pop_oct", 4):
        if cells.get("A") == "Городское население":
            break
        if not cells.get("B", "").isdigit():
            continue
        area = regions[NAMES[cells["A"].replace("г. ", "г.")]]
        opening, change, natural, migration, closing = (int(cells[k]) for k in "BCDEF")
        assert opening == area["people_2026_01_01"] and closing == area["people_2026_10_01"]
        assert opening + natural + migration == closing and natural + migration == change
        area.update(natural_increase_jan_sep_2026=natural, net_migration_jan_sep_2026=migration)
    assert all("net_migration_jan_sep_2026" in r for r in regions.values())


def extract(cache):
    for key, source in SOURCES.items():
        if hashlib.sha256((cache/key).read_bytes()).hexdigest() != source["sha256"]:
            raise ValueError(f"Source {key} changed. Review the edition before updating the snapshot.")
    def pdf(key):
        return subprocess.check_output(["pdftotext", "-layout", str(cache/key), "-"]).decode()
    pop, ag, met = (pdf(k) for k in ("pop","ag","met"))
    population = pop.split("3.3 Численность населения по полу и регионам", 1)[1]
    copper = copper_rows(pdf("copper"))
    series = {
        "people_2026_01_01": bns_rows(block(pop,"3.3 Численность населения по полу и регионам","Женщины и мужчины Казахстана"),1),
        "urban_people_2026_01_01": bns_rows(block(population,"Городское население","Сельское население"),1),
        "rural_people_2026_01_01": bns_rows(block(population,"Сельское население","Мужчины"),1),
        "grain_pulses_t_2024": bns_rows(block(ag,"4.54 Зерновые (включая рис) и бобовые культуры","4.55 Семена"),1000),
        "grain_sown_ha_2024": bns_rows(block(ag,"4.47 Зерновые (включая рис) и бобовые культуры","4.48 Семена"),1000),
        "potatoes_t_2024": bns_rows(block(ag,"4.56 Картофель","4.57 Свекла"),1000),
        "vegetables_t_2024": bns_rows(block(ag,"4.59 Овощи","4.60 Зерновые"),1000),
        "carcass_meat_t_2024": bns_rows(block(ag,"4.73 Забито в хозяйстве","4.74 Молоко"),1000),
        "milk_t_2024": bns_rows(block(ag,"4.74 Молоко","4.75 Яйца"),1000),
        "eggs_count_2024": bns_rows(block(ag,"4.75 Яйца","4.76 Шерсть"),10**6),
        "portland_cement_excl_white_t_2024": bns_rows(block(ag,"Портландцемент (кроме белого), тыс. тонн","Известь гашенная, негашенная"),1000),
        "electricity_kwh_2024": bns_rows(block(ag,"Электроэнергия, млн. кВт. часов","Пар и горячая вода"),10**6),
        "pumps_produced_2024": bns_rows(block(ag,"Насосы центробежные для перекачки жидкостей; насосы прочие, штук","Косилки, включая"),1),
    }
    rows = {key: {"source_name":name, **{field:series[field].get(key) for field in series}}
            for name,key in NAMES.items()}
    for year, key in ((2023,"met23"), (2024,"met24"), (2025,"met")):
        weather = pdf(key) if year < 2025 else met
        rain = climate(weather,"Таблица 3.2 – Регионально","Примечания:")
        heat = climate(weather,"Таблица 2.3 " + ("-" if year == 2025 else "–") + " Регионально","Примечания:")
        for area, row in rows.items():
            for index in range(12):
                month = f"{year}-{index:02}" if index else f"{year-1}-12"
                row.setdefault("monthly_weather", {})[month] = (
                    {"precipitation_anomaly_mm":rain[area][index],"temperature_anomaly_c":heat[area][index]}
                    if area in rain and area in heat else None)
    grain = block(ag,"4.54 Зерновые (включая рис) и бобовые культуры","4.55 Семена")
    for column, year in enumerate(range(2020, 2025), 1):
        harvest = bns_rows(grain, 1000, column)
        for key, row in rows.items():
            row.setdefault("annual_grain_pulses_t", {})[str(year)] = harvest.get(key)
    # National stock and capacity observations are transcribed from the pinned
    # USGS chromium table and KEGOC annual report (the latter uses a specific derating).
    october(cache, rows)
    kz = rows.pop("KAZ")
    assert len(series["people_2026_01_01"]) == 21 and sum(r["people_2026_01_01"] for r in rows.values()) == kz["people_2026_01_01"]
    for r in [kz, *rows.values()]:
        urban, total = r["urban_people_2026_01_01"], r["people_2026_01_01"]
        if r["rural_people_2026_01_01"] is None and urban == total:
            r["rural_people_2026_01_01"] = 0
        assert urban + r["rural_people_2026_01_01"] == total
    iron = re.search(r"^Kazakhstan\s+([\d, ]+)$", pdf("iron"), re.M).group(1).split()
    assert len(iron) == 6
    kz.update(installed_mw_2026_01_01=26807, available_mw_2026_01_01=20081,
              iron_usable_ore_mined_t_2025_estimate=int(iron[1].replace(",", ""))*1000,
              iron_crude_ore_reserves_t_mcs2026=int(iron[4].replace(",", ""))*10**6,
              iron_contained_reserves_t_mcs2026=int(iron[5].replace(",", ""))*10**6,
              copper_mined_t_2025_estimate=copper["KAZ"][0], copper_refined_t_2025_estimate=copper["KAZ"][1],
              copper_reserves_t_mcs2026=copper["KAZ"][2],
              chromite_ore_reserves_t_mcs2026=230000000, chromite_mined_t_2025_estimate=7000000)
    return {"edition":"2026-11-28", "sources":"geography-sources.json", "world":world(cache, copper),
            "kazakhstan":{"national":kz,"regions":rows}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache", type=Path, help="directory with source files named as in geography-sources.json")
    parser.add_argument("output", type=Path)
    parser.add_argument("--download", action="store_true", help="retrieve missing pinned source files")
    args = parser.parse_args()
    if not __debug__:
        raise SystemExit("Run with Python assertions enabled")
    if args.download:
        args.cache.mkdir(parents=True, exist_ok=True)
        for key, source in SOURCES.items():
            if not (args.cache/key).exists():
                with urlopen(source["url"], timeout=120) as response:
                    data = response.read()
                if hashlib.sha256(data).hexdigest() != source["sha256"]:
                    raise ValueError(f"Upstream edition changed: {key}")
                (args.cache/key).write_bytes(data)
    args.output.write_text(json.dumps(extract(args.cache), indent=2, ensure_ascii=False, allow_nan=False)+"\n")
