#!/usr/bin/env python3
"""Check selected fault detection in temporary source copies and a private DB.

No existing database, real credentials, model calls or runtime source edits.
This small, curated mutation set is not a general mutation coverage score.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[1]
MUTATIONS = [
    ("noncanonical_amount", "src/domain.rs", "value.starts_with('0')", 'value == "0"'),
    ("changed_idempotent_payload", "src/ledger.rs", "if stored != *payload {", "if stored.is_null() {"),
    ("reviewer_write_authority", "src/api.rs", "|| (reviewer && review_allowed)", "|| (reviewer && (review_allowed || !get))"),
    ("oversized_http_body", "src/api.rs", "DefaultBodyLimit::max(16 * 1024)", "DefaultBodyLimit::max(16 * 1024 * 1024)"),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".local/mutation-report.json")
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    pg_dir = Path(os.environ.get("PG_BIN", "/opt/homebrew/opt/postgresql@16/bin"))
    report = {"scope": "four curated mutations; not complete coverage", "results": []}
    with tempfile.TemporaryDirectory(prefix="qazna-mutation-") as directory:
        root = Path(directory)
        source, data, sock = root / "source", root / "data", root / "socket"
        source.mkdir()
        sock.mkdir(mode=0o700)
        for name in ("Cargo.toml", "Cargo.lock", "build.rs"):
            shutil.copy2(ROOT / name, source / name)
        for name in ("src", "migrations", "tests"):
            shutil.copytree(ROOT / name, source / name)
        env = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"))
        env.pop("DATABASE_URL", None)
        env["QAZNA_TEST_DATABASE_URL"] = "postgresql://qazna@localhost/mutation_test?" + urlencode({"host": str(sock)})
        started = False

        def pg(name, *values):
            subprocess.run([str(pg_dir / name), *map(str, values)], check=True, stdout=subprocess.DEVNULL)

        def check(label):
            results = []
            for suite, command in (
                ("lib", ["cargo", "test", "--locked", "--offline", "--lib"]),
                ("postgres", ["cargo", "test", "--locked", "--offline", "--test", "postgres", "--", "--ignored", "--test-threads=4"]),
            ):
                run = subprocess.run(command, cwd=source, env=env, capture_output=True, text=True, timeout=600)
                output = run.stdout + run.stderr
                log = args.output.parent / f"mutation-{label}-{suite}.log"
                log.write_text(output)
                summaries = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed", output)
                if not summaries or sum(int(p) + int(f) for _, p, f in summaries) == 0:
                    raise RuntimeError(f"{label}/{suite}: build or infrastructure failure; see {log}")
                if run.returncode not in (0, 101):
                    raise RuntimeError(f"{label}/{suite}: unexpected exit {run.returncode}")
                results.append({"suite": suite, "exit": run.returncode, "summaries": summaries,
                                "failed_tests": re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)})
            return results

        try:
            pg("initdb", "-D", data, "-U", "qazna", "--auth-local=trust", "--auth-host=reject", "--encoding=UTF8", "--locale=C")
            pg("pg_ctl", "-D", data, "-l", root / "postgres.log", "-o", f"-c listen_addresses='' -c unix_socket_directories='{sock}'", "-w", "start")
            started = True
            pg("createdb", "-h", sock, "-U", "qazna", "mutation_test")
            report["baseline"] = check("baseline")
            if any(x["exit"] for x in report["baseline"]):
                raise RuntimeError("Baseline failed; mutations would be uninterpretable")
            for label, relative, old, new in MUTATIONS:
                file = source / relative
                original = file.read_text()
                if original.count(old) != 1:
                    raise RuntimeError(f"{label}: source drift; expected exactly one anchor")
                try:
                    file.write_text(original.replace(old, new))
                    suites = check(label)
                    detected = any(int(f) > 0 for s in suites for _, _, f in s["summaries"])
                    report["results"].append({"mutation": label, "file": relative, "detected": detected, "suites": suites})
                    print(f"{label}: {'detected' if detected else 'SURVIVED'}", flush=True)
                finally:
                    file.write_text(original)
            args.output.write_text(json.dumps(report, indent=2) + "\n")
        finally:
            if started:
                pg("pg_ctl", "-D", data, "-m", "fast", "-w", "stop")
    if not all(x["detected"] for x in report["results"]):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
