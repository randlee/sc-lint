#!/usr/bin/env python3
"""Run the F.3b host-protocol matrix against a released Wyvern binary.

This is deliberately an HTTP client, not a replacement wizard implementation.
Every state transition is a request to the released host and every terminal
result is read from that process's stdout.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


PROTOCOL_VERSION = "wizard-http-v1"
EXPECTED_VERSION = "wyvern 0.6.0"
TIMEOUT_SECONDS = 30
ROOT = Path(__file__).resolve().parent
WIZARD = ROOT / "wizard.json"


class SmokeFailure(RuntimeError):
    pass


def request(base: str, path: str, body: dict[str, Any] | None = None) -> tuple[int, dict[str, Any]]:
    payload = None if body is None else json.dumps(body, separators=(",", ":")).encode()
    headers = {} if payload is None else {"Content-Type": "application/json"}
    req = Request(f"{base}{path}", data=payload, headers=headers)
    try:
        with urlopen(req, timeout=5) as response:
            return response.status, json.loads(response.read())
    except HTTPError as error:
        return error.code, json.loads(error.read())
    except URLError as error:
        raise SmokeFailure(f"HTTP request failed for {path}: {error}") from error


def spawn(binary: Path) -> tuple[subprocess.Popen[str], str]:
    process = subprocess.Popen(
        [str(binary), str(WIZARD), "--viewer", "none"],
        cwd=ROOT,
        env={**os.environ, "WYVERN_VIEWER": "none"},
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=True,
    )
    assert process.stderr is not None
    deadline = time.monotonic() + 10
    url = ""
    while time.monotonic() < deadline:
        line = process.stderr.readline()
        if line.startswith("WYVERN_DIALOG_URL="):
            url = line.strip().split("=", 1)[1]
            break
        if process.poll() is not None:
            break
    if not url:
        process.kill()
        stderr = process.stderr.read()
        raise SmokeFailure(f"released Wyvern did not publish WYVERN_DIALOG_URL: {stderr}")
    return process, url.split("/wizard/", 1)[0]


def finish_process(process: subprocess.Popen[str], expected_button: str) -> dict[str, Any]:
    try:
        stdout, stderr = process.communicate(timeout=5)
    except subprocess.TimeoutExpired as error:
        process.kill()
        process.communicate()
        raise SmokeFailure("terminal request did not unblock the released host") from error
    if process.returncode != 0:
        raise SmokeFailure(f"Wyvern exited {process.returncode}: {stderr.strip()}")
    try:
        result = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise SmokeFailure(f"Wyvern stdout was not JSON: {stdout!r}") from error
    if result.get("button") != expected_button:
        raise SmokeFailure(f"expected terminal button {expected_button!r}, got {result!r}")
    return result


def stop_process(process: subprocess.Popen[str]) -> None:
    if process.poll() is None:
        if os.name == "nt":
            process.terminate()
        else:
            os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            if os.name == "nt":
                process.kill()
            else:
                os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=2)


def descriptor(page_id: str) -> dict[str, str]:
    return {"id": page_id, "title": page_id.title(), "html": f"pages/{page_id}.html"}


def state(base: str) -> dict[str, Any]:
    status, payload = request(base, "/api/wizard/state")
    if status != 200:
        raise SmokeFailure(f"state returned HTTP {status}: {payload}")
    return payload


def navigate(base: str, action: str, page_id: str | None = None, data: Any = None, next_page: str | None = None) -> tuple[int, dict[str, Any]]:
    body: dict[str, Any] = {"action": action, "data": {} if data is None else data}
    if page_id is not None:
        body["page_id"] = page_id
    if next_page is not None:
        body["next"] = descriptor(next_page)
    return request(base, "/api/wizard/navigate", body)


def terminal(binary: Path, button: str, data: Any = None, drive: Any = None) -> dict[str, Any]:
    process, base = spawn(binary)
    try:
        snapshot = state(base)
        if drive is not None:
            drive(base, snapshot)
            snapshot = state(base)
        current_data = {} if data is None else data
        stack = snapshot["stack"] + [{"page": snapshot["page"], "data": current_data}]
        status, payload = request(
            base,
            "/api/wizard/finish",
            {"button": button, "data": current_data, "stack": stack},
        )
        if status != 200:
            raise SmokeFailure(f"{button} returned HTTP {status}: {payload}")
        return finish_process(process, button)
    finally:
        stop_process(process)


def run_case(binary: Path, name: str, callback: Any) -> dict[str, Any]:
    try:
        result = callback(binary)
        return {"case": name, "status": "passed", "result": result}
    except SmokeFailure:
        raise


def case_navigation(binary: Path) -> dict[str, Any]:
    process, base = spawn(binary)
    try:
        initial = state(base)
        if initial["page"]["id"] != "overview" or initial["stack"] != []:
            raise SmokeFailure(f"unexpected initial state: {initial}")
        status, _ = navigate(base, "next", page_id="baseline", data={"minimum_version": "0.5.0"}, next_page="baseline")
        if status != 200:
            raise SmokeFailure(f"forward returned HTTP {status}")
        status, _ = navigate(base, "next", page_id="review", data={"profile": "recommended"}, next_page="review")
        if status != 200:
            raise SmokeFailure(f"second forward returned HTTP {status}")
        after_forward = state(base)
        status, _ = navigate(base, "back", data={})
        if status != 200:
            raise SmokeFailure(f"back to baseline returned HTTP {status}")
        restored = state(base)
        if restored["page"]["id"] != "baseline" or restored["page_data"] != {"profile": "recommended"}:
            raise SmokeFailure(f"back did not restore opaque page data: {restored}")
        status, _ = navigate(base, "back", data={})
        if status != 200:
            raise SmokeFailure(f"branch setup back to overview returned HTTP {status}")
        status, _ = navigate(base, "next", page_id="review", data={"branch": "new"}, next_page="review")
        if status != 200:
            raise SmokeFailure(f"branch forward returned HTTP {status}")
        branched = state(base)
        if len(branched["stack"]) != 1 or branched["page"]["id"] != "review":
            raise SmokeFailure(f"stale forward history was not truncated: {branched}")
        return {"initial": initial, "forward": after_forward, "restored": restored, "branch": branched}
    finally:
        stop_process(process)


def case_first_page_back(binary: Path) -> dict[str, Any]:
    process, base = spawn(binary)
    try:
        back_status, payload = navigate(base, "back", data={})
        if back_status != 400 or payload.get("code") != "WIZARD_AT_FIRST_PAGE":
            raise SmokeFailure(f"first-page back contract changed: HTTP {back_status} {payload}")
        cancel_status, _ = request(base, "/api/wizard/finish", {"button": "cancel", "data": {}, "stack": []})
        if cancel_status != 200:
            raise SmokeFailure(f"cleanup cancel returned HTTP {cancel_status}")
        result = finish_process(process, "cancel")
        return {"http_status": back_status, "error_code": payload["code"], "terminal": result}
    finally:
        stop_process(process)


def case_finish(binary: Path) -> dict[str, Any]:
    def drive(base: str, _: dict[str, Any]) -> None:
        status, _ = navigate(base, "next", page_id="baseline", data={"minimum_version": "0.5.0"}, next_page="baseline")
        if status != 200:
            raise SmokeFailure(f"finish setup returned HTTP {status}")
        status, _ = navigate(base, "next", page_id="review", data={"profile": "recommended"}, next_page="review")
        if status != 200:
            raise SmokeFailure(f"finish second page returned HTTP {status}")

    result = terminal(binary, "finish", {"confirmed": True}, drive)
    if len(result.get("stack", [])) != 3 or result.get("data") != {"confirmed": True}:
        raise SmokeFailure(f"finish did not deliver full stack: {result}")
    return result


def case_cancel(binary: Path) -> dict[str, Any]:
    result = terminal(binary, "cancel", {"ignored": True})
    if result.get("data") != {} or result.get("stack") != []:
        raise SmokeFailure(f"cancel result was not empty: {result}")
    return result


def case_dismissed(binary: Path) -> dict[str, Any]:
    def drive(base: str, _: dict[str, Any]) -> None:
        status, _ = navigate(base, "next", page_id="baseline", data={"minimum_version": "0.5.0"}, next_page="baseline")
        if status != 200:
            raise SmokeFailure(f"dismiss setup returned HTTP {status}")

    result = terminal(binary, "dismissed", {}, drive)
    if result.get("data") != {} or len(result.get("stack", [])) != 2:
        raise SmokeFailure(f"dismissed result did not preserve full visited stack: {result}")
    return result


def case_timeout(binary: Path) -> dict[str, Any]:
    process, _ = spawn(binary)
    started = time.monotonic()
    try:
        stdout, stderr = process.communicate(timeout=TIMEOUT_SECONDS + 10)
    except subprocess.TimeoutExpired as error:
        process.kill()
        process.communicate()
        raise SmokeFailure("headless session exceeded its 30-second timeout budget") from error
    elapsed = time.monotonic() - started
    if process.returncode == 0:
        raise SmokeFailure(f"undriven headless wizard unexpectedly succeeded: {stdout!r}")
    if "session" not in stdout.lower() and "timeout" not in stderr.lower():
        raise SmokeFailure(f"timeout result lacked stable session-timeout evidence: stdout={stdout!r} stderr={stderr!r}")
    return {"returncode": process.returncode, "elapsed_seconds": round(elapsed, 1), "stderr": stderr.strip()}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True, help="path to the checksum-verified native v0.6.0 wyvern binary")
    parser.add_argument("--output", type=Path, help="write normalized JSON report to this path")
    args = parser.parse_args()
    binary = args.binary.expanduser().resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise SystemExit(f"binary is not executable: {binary}")
    version = subprocess.run([str(binary), "--version"], check=True, capture_output=True, text=True).stdout.strip()
    if version != EXPECTED_VERSION:
        raise SystemExit(f"expected {EXPECTED_VERSION!r}, got {version!r}")

    cases = [
        ("forward_back_restore_branch", case_navigation),
        ("first_page_back_disabled", case_first_page_back),
        ("finish_full_stack", case_finish),
        ("cancel", case_cancel),
        ("dismissed", case_dismissed),
        ("timeout", case_timeout),
    ]
    report: dict[str, Any] = {"protocol_version": PROTOCOL_VERSION, "version": version, "cases": []}
    for name, callback in cases:
        report["cases"].append(run_case(binary, name, callback))
    encoded = json.dumps(report, sort_keys=True, indent=2) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SmokeFailure as error:
        print(f"F3B SMOKE FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
