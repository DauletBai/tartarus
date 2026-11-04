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

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    subprocess.run(args, cwd=ROOT, check=True, **kwargs)


def main():
    run([sys.executable, "scripts/test_ai_review.py"])
    run(["cargo", "fmt", "--check"])
    run([sys.executable, "scripts/licenses.py", "--check"])
    run(["cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings"])
    run(["cargo", "test", "--locked", "--lib"])
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
