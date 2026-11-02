"""Exercise the real binary over HTTP, including a process restart.

Called by check.py with a disposable database; credentials are never printed.
"""
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


def check_restart(root: Path, database_url: str):
    env = dict(os.environ)
    env["DATABASE_URL"] = database_url
    env["QAZNA_OPERATOR_TOKEN"] = secrets.token_hex(32)
    binary = root / "target/debug/qazna"
    receipt = None
    held = None
    treasury = "00000000-0000-0000-0000-000000000002"
    resident = "00000000-0000-0000-0000-000000000003"
    producer = "00000000-0000-0000-0000-000000000004"
    resource = "00000000-0000-0000-0000-000000000005"
    root_unit = "00000000-0000-0000-0000-000000000006"
    child_unit = "00000000-0000-0000-0000-000000000007"
    need = "00000000-0000-0000-0000-000000000008"
    for iteration in range(2):
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        env["QAZNA_BIND"] = f"127.0.0.1:{port}"
        base = f"http://127.0.0.1:{port}"
        process = subprocess.Popen([str(binary), "serve"], cwd=root, env=env,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 10
            while True:
                if process.poll() is not None:
                    raise AssertionError(process.stderr.read().decode())
                try:
                    with urlopen(base + "/health/ready", timeout=0.5) as response:
                        assert response.status == 200
                    break
                except (URLError, TimeoutError):
                    if time.monotonic() >= deadline:
                        raise
                    time.sleep(0.05)
            headers = {"Authorization": "Bearer " + env["QAZNA_OPERATOR_TOKEN"],
                       "Content-Type": "application/json", "Idempotency-Key": "restart-payment"}

            def call(path, body=None, key=None):
                auth = {"Authorization": headers["Authorization"], "Content-Type": "application/json"}
                if key:
                    auth["Idempotency-Key"] = key
                data = None if body is None else json.dumps(body).encode()
                with urlopen(Request(base + path, data, auth), timeout=5) as response:
                    return json.load(response)
            body = json.dumps({"from": "00000000-0000-0000-0000-000000000002",
                               "to": "00000000-0000-0000-0000-000000000003", "amount": "100"}).encode()
            with urlopen(Request(base + "/v1/transfers", body, headers), timeout=5) as response:
                current = json.load(response)
            if iteration == 0:
                receipt = current
            else:
                assert current == receipt, "replayed receipt changed after process restart"
            path = base + "/v1/accounts/00000000-0000-0000-0000-000000000002"
            with urlopen(Request(path, headers=headers), timeout=5) as response:
                assert json.load(response)["balance"] == ("900" if iteration == 0 else "700")
            if iteration == 0:
                call("/v1/accounts", {"id": producer, "kind": "producer"})
                call("/v1/allocations", {"treasury": treasury, "resident": resident,
                     "period": "2026-11-02", "category": "food", "amount": "200",
                     "basis": "Synthetic needs allocation"}, "allocation")
                call("/v1/resources", {"id": resource, "producer": producer, "category": "food",
                     "unit": "meal", "quantity": "10", "unit_measure": "20",
                     "basis": "Synthetic lot and experimental norm v1"}, "resource")
                call("/v1/units", {"id": root_unit, "parent": None, "name": "Community",
                     "function": "Coordinate provision", "basis": "Synthetic hierarchy"}, "unit-root")
                call("/v1/units", {"id": child_unit, "parent": root_unit, "name": "Food unit",
                     "function": "Provide meals", "basis": "Synthetic hierarchy"}, "unit-child")
                call(f"/v1/units/{child_unit}/providers", {"producer": producer, "basis": "Synthetic assignment"}, "unit-provider")
                assert call(f"/v1/units/{root_unit}")["children"] == [child_unit]
                call("/v1/needs", {"id": need, "resident": resident, "assigned_unit": root_unit,
                     "period": "2026-11-02", "category": "food", "unit": "meal", "quantity": "12",
                     "basis": "Declared synthetic need"}, "need")
                blocked = call(f"/v1/needs/{need}/reserve", {"resource": resource, "quantity": "12",
                               "basis": "Attempt full provision"}, "need-blocked")
                assert blocked["status"] == "blocked"
                assert blocked["reasons"] == ["physical_shortage"]
                attempt = call(f"/v1/needs/{need}/reserve", {"resource": resource, "quantity": "4",
                               "basis": "Partial provision"}, "reserve")
                held = call(f"/v1/reservations/{attempt['reservation_id']}")
                assert call(f"/v1/accounts/{resident}")["reserved"] == "80"
            else:
                assert call(f"/v1/reservations/{held['id']}")["state"] == "reserved"
                current_need = call(f"/v1/needs/{need}")
                assert current_need["reserved"] == "4" and current_need["received"] == "0"
                assert current_need["unmet"] == "12"
                assert len(call(f"/v1/needs/{need}/attempts")) == 2
                assert call(f"/v1/resources/{resource}")["available"] == "6"
                coverage = call(f"/v1/units/{root_unit}/coverage?period=2026-11-02")
                assert len(coverage["lines"]) == 1
                assert coverage["lines"][0]["requested"] == "12"
                assert coverage["lines"][0]["received"] == "0"
                assert coverage["lines"][0]["unmet"] == "12"
                body = {"basis": "Receipt confirmed"}
                settled = call(f"/v1/reservations/{held['id']}/settle", body, "settle")
                assert settled == call(f"/v1/reservations/{held['id']}/settle", body, "settle")
                assert call(f"/v1/accounts/{resident}")["balance"] == "220"
                assert call(f"/v1/accounts/{producer}")["balance"] == "80"
                assert call(f"/v1/resources/{resource}")["on_hand"] == "6"
                coverage = call(f"/v1/units/{root_unit}/coverage?period=2026-11-02")
                assert coverage["lines"][0]["received"] == "4"
                assert coverage["lines"][0]["unmet"] == "8"
                second = call("/v1/reservations", {"resident": resident, "resource": resource,
                              "quantity": "2", "basis": "Second provision request"}, "reserve-2")
                call(f"/v1/reservations/{second['id']}/cancel", {"basis": "Request withdrawn"}, "cancel-2")
                assert call(f"/v1/accounts/{resident}")["available"] == "220"
                assert call(f"/v1/resources/{resource}")["available"] == "6"
            try:
                urlopen(path, timeout=5)
            except HTTPError as error:
                assert error.code == 401
            else:
                raise AssertionError("private account endpoint accepted an unauthenticated request")
        finally:
            if process.poll() is None:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            process.stderr.close()
    print("HTTP smoke passed: unit hierarchy, needs feedback, persistent reservation and delivered coverage after restart")
