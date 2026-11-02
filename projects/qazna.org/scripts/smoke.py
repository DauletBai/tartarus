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
    production_hold = None
    capacity = "00000000-0000-0000-0000-000000000009"
    recipe = "00000000-0000-0000-0000-000000000010"
    treasury = "00000000-0000-0000-0000-000000000002"
    resident = "00000000-0000-0000-0000-000000000003"
    producer = "00000000-0000-0000-0000-000000000004"
    resource = "00000000-0000-0000-0000-000000000005"
    root_unit = "00000000-0000-0000-0000-000000000006"
    child_unit = "00000000-0000-0000-0000-000000000007"
    need = "00000000-0000-0000-0000-000000000008"
    output_resource = "00000000-0000-0000-0000-000000000011"
    meal_norm = "00000000-0000-0000-0000-000000000012"
    packing_norm = "00000000-0000-0000-0000-000000000013"
    packing_need = "00000000-0000-0000-0000-000000000014"
    completed_plan = None
    completion_request = None
    completed_receipt = None
    for iteration in range(3):
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
            if iteration == 2:
                assert call(f"/v1/production/plans/{completed_plan}")["state"] == "completed"
                assert call(f"/v1/production/plans/{completed_plan}/completion") == completed_receipt
                assert call(f"/v1/production/plans/{completed_plan}/complete", completion_request, "complete-production") == completed_receipt
                origin = call(f"/v1/resources/{output_resource}/provenance")
                assert origin["origin"] == "production"
                assert origin["completion"]["inputs"][0]["resource"] == resource
                assert origin["completion"]["inputs"][0]["loss"] == "1"
                assert call(f"/v1/resources/{resource}")["on_hand"] == "4"
                assert call(f"/v1/resources/{output_resource}")["available"] == "1"
                totals = call(f"/v1/production/capacities/{capacity}")
                assert (totals["reserved"], totals["consumed"], totals["available"]) == ("0", "4", "6")
                assert call(f"/v1/accounts/{producer}")["balance"] == "80"
                assert call(f"/v1/resources/{resource}/provenance")["origin"] == "declared"
                assert call(f"/v1/needs/{need}/norm")["id"] == meal_norm
                assert call(f"/v1/need-norms/{meal_norm}")["quantity"] == "12"
                call("/v1/need-norms", {"id": packing_norm, "code": "demo-packed-meals", "revision": 1,
                     "supersedes": None, "category": "food", "unit": "packed_meal", "quantity": "3",
                     "applicability": "Explicit demo need", "basis": "Experimental assumption"}, "packing-norm")
                call("/v1/needs/from-norm", {"id": packing_need, "resident": resident, "assigned_unit": child_unit,
                     "period": "2026-11-02", "norm": packing_norm, "basis": "Explicit application"}, "packing-need")
                proposal_request = {"period": "2026-11-02", "basis": "Operator priority order", "needs": [
                    {"need": packing_need, "resources": [output_resource], "production": [
                        {"recipe": recipe, "capacity": capacity, "inputs": [{"slot": "meal", "resource": resource}]}]}]}
                advice = call("/v1/planning/assess", proposal_request)
                assert advice["advisory"] is True
                assert advice["needs"][0]["stock_selected"] == "1"
                assert advice["needs"][0]["expected_from_production"] == "2"
                assert advice["needs"][0]["residual"] == "0"
                assert advice["needs"][0]["production"][0]["request"]["batches"] == "2"
                assert call(f"/v1/production/capacities/{capacity}")["reserved"] == "0"
                assert call(f"/v1/needs/{packing_need}")["uncovered"] == "3"
                try:
                    urlopen(Request(base + "/v1/planning/assess", json.dumps(proposal_request).encode(),
                                    {"Content-Type": "application/json"}), timeout=5)
                except HTTPError as error:
                    assert error.code == 401
                else:
                    raise AssertionError("planning accepted an unauthenticated request")
            elif iteration == 0:
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
                call("/v1/need-norms", {"id": meal_norm, "code": "demo-meals", "revision": 1,
                     "supersedes": None, "category": "food", "unit": "meal", "quantity": "12",
                     "applicability": "Explicit synthetic resident/day", "basis": "Experimental demo assumption"}, "meal-norm")
                call("/v1/needs/from-norm", {"id": need, "resident": resident, "assigned_unit": root_unit,
                     "period": "2026-11-02", "norm": meal_norm, "basis": "Explicit norm application"}, "need")
                blocked = call(f"/v1/needs/{need}/reserve", {"resource": resource, "quantity": "12",
                               "basis": "Attempt full provision"}, "need-blocked")
                assert blocked["status"] == "blocked"
                assert blocked["reasons"] == ["physical_shortage"]
                attempt = call(f"/v1/needs/{need}/reserve", {"resource": resource, "quantity": "4",
                               "basis": "Partial provision"}, "reserve")
                held = call(f"/v1/reservations/{attempt['reservation_id']}")
                assert call(f"/v1/accounts/{resident}")["reserved"] == "80"
                call("/v1/production/capacities", {"id": capacity, "unit": child_unit,
                     "period": "2026-11-02", "capacity_unit": "packing_minutes", "quantity": "10",
                     "basis": "Synthetic packing capacity"}, "capacity")
                call("/v1/production/recipes", {"id": recipe, "unit": child_unit, "name": "Pack meal",
                     "capacity_unit": "packing_minutes", "capacity_per_batch": "2", "output_category": "food",
                     "output_unit": "packed_meal", "output_per_batch": "1", "inputs": [{"slot": "meal",
                     "category": "food", "unit": "meal", "quantity_per_batch": "1"}], "basis": "Synthetic packing process"}, "recipe")
                production_request = {"recipe": recipe, "capacity": capacity, "batches": "7",
                     "inputs": [{"slot": "meal", "resource": resource}], "basis": "Packing plan"}
                assessment = call("/v1/production/assess", production_request)
                assert assessment["max_batches"] == "5"
                assert assessment["reasons"] == ["capacity_shortage", "input_shortage"]
                production_request["batches"] = "2"
                production_hold = call("/v1/production/plans", production_request, "production")
                assert production_hold["status"] == "reserved"
                assert call(f"/v1/resources/{resource}")["available"] == "4"
            else:
                assert call(f"/v1/reservations/{held['id']}")["state"] == "reserved"
                plan_id = production_hold["plan_id"]
                assert call(f"/v1/production/plans/{plan_id}")["state"] == "reserved"
                assert call("/v1/production/plans", production_request, "production") == production_hold
                assert call(f"/v1/production/capacities/{capacity}")["available"] == "6"
                assert call(f"/v1/production/recipes/{recipe}")["inputs"][0]["slot"] == "meal"
                assert call(f"/v1/resources/{resource}")["available"] == "4"
                cancelled = call(f"/v1/production/plans/{plan_id}/cancel", {"basis": "Withdraw plan"}, "cancel-production")
                assert cancelled["state"] == "cancelled"
                assert call(f"/v1/production/capacities/{capacity}")["available"] == "10"
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
                completed_plan = call("/v1/production/plans", production_request, "production-to-complete")["plan_id"]
                completion_request = {"output": {"id": output_resource, "producer": producer,
                    "quantity": "1", "unit_measure": "20"}, "losses": [{"slot": "meal", "quantity": "1"}],
                    "basis": "Observed packing result", "shortfall_basis": "One portion damaged"}
                completed_receipt = call(f"/v1/production/plans/{completed_plan}/complete", completion_request, "complete-production")
                assert completed_receipt["output_shortfall"] == "1"
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
    print("HTTP smoke passed: norms, advisory planning, provisioning and production provenance across two restarts")
