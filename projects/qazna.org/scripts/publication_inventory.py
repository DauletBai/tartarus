#!/usr/bin/env python3
"""List only the Qazna source/publication files; no build output or local data.

Used when adding this project to the Tartarus monorepo through GitHub's API.
Prints paths, sizes and content hashes. Does not commit, upload or alter files.
"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOP = ["build.rs", ".gitignore", "AGENTS.md", "Cargo.toml", "Cargo.lock", "README.md", "LICENSE",
       "LICENSE-DOCS", "NOTICE", "CONTRIBUTING.md", "SECURITY.md", "CHANGELOG.md",
       "THIRD_PARTY_LICENSES.json"]


def inventory():
    files = [ROOT / name for name in TOP]
    for directory in ["src", "migrations", "fixtures", "scripts", "tests", "docs"]:
        files += [p for p in (ROOT / directory).rglob("*")
                  if p.is_file() and p.suffix in {".rs", ".sql", ".py", ".md"}
                  and "__pycache__" not in p.parts]
    result = []
    for p in sorted(files):
        if p.is_symlink():
            raise SystemExit(f"Publication does not follow symlinks: {p.name}")
        data = p.read_bytes()
        data.decode("utf-8")
        result.append({"path": "projects/qazna.org/" + p.relative_to(ROOT).as_posix(),
                       "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                       "git_blob_sha": hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()})
    workflow = ROOT.parents[1] / ".github/workflows/qazna.yml"
    if workflow.is_file():
        data = workflow.read_bytes()
        result.append({"path": ".github/workflows/qazna.yml", "bytes": len(data),
                       "sha256": hashlib.sha256(data).hexdigest(),
                       "git_blob_sha": hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()})
    return result


if __name__ == "__main__":
    print(json.dumps(inventory(), ensure_ascii=False, indent=2))
