#!/usr/bin/env python3
"""Check Rust and run integration tests in a disposable, socket-only PostgreSQL.

Needs PostgreSQL 16+ binaries on PATH, or PG_BIN pointing to their directory.
Never connects to a user's running cluster. Only removes its own temp directory.
"""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import urlencode
from smoke import check_restart
from exercise import run_exercise
from evidence_exercise import run_evidence_exercise
from recovery_check import check_recovery
from circulation_exercise import run as run_circulation

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    subprocess.run(args, cwd=ROOT, check=True, **kwargs)


def main():
    if not __debug__:
        raise SystemExit("Checks require Python assertions: remove -O and PYTHONOPTIMIZE")
    run([sys.executable, "scripts/test_ai_review.py"])
    run(["cargo", "fmt", "--check"])
    run([sys.executable, "scripts/licenses.py", "--check"])
    run(["cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings"])
    run(["cargo", "test", "--locked", "--lib"])
    run(["cargo", "test", "--locked", "--test", "mathematics"])
    run(["cargo", "test", "--locked", "--test", "nutrition"])
    run(["cargo", "test", "--locked", "--test", "seasonal"])
    run(["cargo", "test", "--locked", "--test", "capital"])
    run(["cargo", "test", "--locked", "--test", "rolling"])
    run(["cargo", "test", "--locked", "--test", "dated"])
    run(["cargo", "test", "--locked", "--test", "population"])
    run(["cargo", "test", "--locked", "--example", "no_ai_study"])
    run(["cargo", "test", "--locked", "--example", "coupled_study"])
    with tempfile.TemporaryDirectory(prefix="qazna-math-") as math_dir:
        generated = Path(math_dir) / "horizon-results.json"
        run(["cargo", "run", "--locked", "--example", "horizon_study", "--", str(generated)])
        if generated.read_bytes() != (ROOT / "research/horizon-results.json").read_bytes():
            raise SystemExit("Published horizon results differ from current calculations")
        circulation = Path(math_dir) / "circulation-model.json"
        run(["cargo", "run", "--locked", "--example", "circulation_study", "--", str(circulation)])
        if circulation.read_bytes() != (ROOT / "research/circulation-model.json").read_bytes():
            raise SystemExit("Published circulation model differs from current calculations")
        coupled = Path(math_dir) / "coupled-results.json"
        run(["cargo", "run", "--locked", "--release", "--example", "coupled_study", "--", str(coupled)])
        if coupled.read_bytes() != (ROOT / "research/coupled-results.json").read_bytes():
            raise SystemExit("Published coupled model differs from current calculations")
    with tempfile.TemporaryDirectory(prefix="qazna-extended-") as directory:
        target = Path(directory) / "extended-scenario.json"
        run(["cargo", "run", "--locked", "--release", "--example", "extended_scenario", "--", str(target)])
        if target.read_bytes() != (ROOT / "fixtures/extended-scenario.json").read_bytes():
            raise SystemExit("Extended scenario differs from reproducible calculation")
    with tempfile.TemporaryDirectory(prefix="qazna-population-") as directory:
        fixture, report = Path(directory) / "population-study.json", Path(directory) / "population-results.json"
        run(["cargo", "run", "--locked", "--release", "--example", "population_study", "--", str(fixture), str(report)])
        for generated, published in [(fixture, ROOT / "fixtures/population-study.json"), (report, ROOT / "research/population-results.json")]:
            if generated.read_bytes() != published.read_bytes():
                raise SystemExit("Population inputs or results differ from reproducible calculation")
    pg_dir = os.environ.get("PG_BIN")
    if not pg_dir:
        binary = shutil.which("initdb")
        if not binary:
            raise SystemExit("Set PG_BIN to the PostgreSQL 16+ bin directory")
        pg_dir = str(Path(binary).parent)
    pg = lambda name: str(Path(pg_dir) / name)
    started = False
    with tempfile.TemporaryDirectory(prefix="qazna-pg-") as directory:
        root = Path(directory)
        data, sock = root / "data", root / "socket"
        sock.mkdir(mode=0o700)
        try:
            run([pg("initdb"), "-D", str(data), "-U", "qazna", "--auth-local=trust", "--auth-host=reject", "--encoding=UTF8", "--locale=C"], stdout=subprocess.DEVNULL)
            # No TCP listener. Trust authentication is confined to a private socket directory.
            run([pg("pg_ctl"), "-D", str(data), "-l", str(root / "postgres.log"), "-o", f"-c listen_addresses='' -c unix_socket_directories='{sock}'", "-w", "start"])
            started = True
            run([pg("createdb"), "-h", str(sock), "-U", "qazna", "qazna_test"])
            env = dict(os.environ)
            env["QAZNA_TEST_DATABASE_URL"] = "postgresql://qazna@localhost/qazna_test?" + urlencode({"host": str(sock)})
            run(["cargo", "test", "--locked", "--test", "postgres", "--", "--ignored", "--test-threads=4"], env=env)
            run(["cargo", "build", "--locked"])
            run([pg("createdb"), "-h", str(sock), "-U", "qazna", "qazna_demo"])
            env["DATABASE_URL"] = "postgresql://qazna@localhost/qazna_demo?" + urlencode({"host": str(sock)})
            run([str(ROOT / "target/debug/qazna"), "migrate"], env=env)
            run([pg("psql"), "-h", str(sock), "-U", "qazna", "-d", "qazna_demo", "-v", "ON_ERROR_STOP=1", "-f", "fixtures/demo.sql"], stdout=subprocess.DEVNULL)
            check_restart(ROOT, env["DATABASE_URL"])
            run([pg("createdb"), "-h", str(sock), "-U", "qazna", "evidence_demo"])
            evidence_url = "postgresql://qazna@localhost/evidence_demo?" + urlencode({"host": str(sock)})
            run([str(ROOT / "target/debug/qazna"), "migrate"], env=dict(env, DATABASE_URL=evidence_url))
            run([pg("psql"), "-h", str(sock), "-U", "qazna", "-d", "evidence_demo", "-v", "ON_ERROR_STOP=1", "-f", "fixtures/demo.sql"], stdout=subprocess.DEVNULL)
            run_evidence_exercise(evidence_url, root / "evidence")
            check_recovery(pg, root, sock)
            run([pg("createdb"), "-h", str(sock), "-U", "qazna", "circulation_demo"])
            circulation_url = "postgresql://qazna@localhost/circulation_demo?" + urlencode({"host": str(sock)})
            run([str(ROOT / "target/debug/qazna"), "migrate"], env=dict(env, DATABASE_URL=circulation_url))
            run([pg("psql"), "-h", str(sock), "-U", "qazna", "-d", "circulation_demo", "-v", "ON_ERROR_STOP=1", "-f", "fixtures/demo.sql"], stdout=subprocess.DEVNULL)
            run_circulation(circulation_url, root / "circulation", steps=60, tick=.02)
            run([pg("createdb"), "-h", str(sock), "-U", "qazna", "closed_demo"])
            closed_url = "postgresql://qazna@localhost/closed_demo?" + urlencode({"host": str(sock)})
            run([str(ROOT / "target/debug/qazna"), "migrate"], env=dict(env, DATABASE_URL=closed_url))
            run([pg("psql"), "-h", str(sock), "-U", "qazna", "-d", "closed_demo", "-v", "ON_ERROR_STOP=1", "-f", "fixtures/demo.sql"], stdout=subprocess.DEVNULL)
            run_circulation(closed_url, root / "closed", steps=35, tick=.02, closed=True)
            for seed in (7, 19):
                database = f"exercise_{seed}_demo"
                run([pg("createdb"), "-h", str(sock), "-U", "qazna", database])
                env["DATABASE_URL"] = f"postgresql://qazna@localhost/{database}?" + urlencode({"host": str(sock)})
                run([str(ROOT / "target/debug/qazna"), "migrate"], env=env)
                run([pg("psql"), "-h", str(sock), "-U", "qazna", "-d", database, "-v", "ON_ERROR_STOP=1", "-f", "fixtures/demo.sql"], stdout=subprocess.DEVNULL)
                run_exercise(env["DATABASE_URL"], root / f"exercise-{seed}", seed, .02, 20)
        except subprocess.CalledProcessError:
            log = root / "postgres.log"
            if log.exists():
                print(log.read_text()[-6000:])
            raise
        finally:
            if started:
                run([pg("pg_ctl"), "-D", str(data), "-m", "fast", "-w", "stop"])


if __name__ == "__main__":
    main()
