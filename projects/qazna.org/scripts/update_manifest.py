#!/usr/bin/env python3
"""Refresh only Qazna publication entries in the parent repository manifest."""
import csv
from pathlib import Path
from publication_inventory import ROOT, inventory


def main():
    manifest = ROOT.parents[1] / "MANIFEST.csv"
    with manifest.open(encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        if reader.fieldnames != ["path", "bytes", "sha256"]:
            raise SystemExit("Unexpected parent manifest format")
        entries = {row["path"]: row for row in reader}
    files = inventory()
    for item in files:
        entries[item["path"]] = {key: item[key] for key in ["path", "bytes", "sha256"]}
    with manifest.open("w", encoding="utf-8-sig", newline="") as destination:
        writer = csv.DictWriter(destination, fieldnames=["path", "bytes", "sha256"], lineterminator="\n")
        writer.writeheader()
        writer.writerows(entries[path] for path in sorted(entries))
    print(f"Updated {len(files)} Qazna/CI manifest entries; preserved unrelated entries")


if __name__ == "__main__":
    main()
