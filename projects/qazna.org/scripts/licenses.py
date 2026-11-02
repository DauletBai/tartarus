#!/usr/bin/env python3
"""Generate/check upstream license metadata for the complete locked Cargo graph.

No third-party code or binaries are bundled. Platform and optional dependencies
are included conservatively. This inventory does not replace release notices.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def inventory():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=ROOT))
    lock_bytes = (ROOT / "Cargo.lock").read_bytes()
    lock = tomllib.loads(lock_bytes.decode())
    checksums = {(p["name"], p["version"]): p.get("checksum") for p in lock["package"]}
    packages = []
    for p in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if p["source"] is None:
            continue
        if not p.get("license") and not p.get("license_file"):
            raise SystemExit(f"Missing upstream license declaration: {p['name']} {p['version']}")
        packages.append({"name": p["name"], "version": p["version"], "license": p["license"],
                         "license_file": Path(p["license_file"]).name if p.get("license_file") else None,
                         "repository": p["repository"], "source": p["source"],
                         "checksum": checksums[(p["name"], p["version"])]})
    return {"scope": "Locked Cargo graph; includes optional and platform-specific crates; declarations are upstream metadata",
            "cargo_lock_sha256": hashlib.sha256(lock_bytes).hexdigest(), "packages": packages}


if __name__ == "__main__":
    content = json.dumps(inventory(), ensure_ascii=False, indent=2) + "\n"
    target = ROOT / "THIRD_PARTY_LICENSES.json"
    if "--check" in sys.argv:
        if not target.exists() or target.read_text() != content:
            raise SystemExit("License inventory is stale; run python3 scripts/licenses.py")
    else:
        target.write_text(content)
    print("Dependency license inventory verified")
