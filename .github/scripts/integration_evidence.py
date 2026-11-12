"""Run adoption integration tests and render their actual results with sc-compose."""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEMPLATES = ROOT / "release/integration-evidence"
REPORTS = ROOT / "site/reports/integration"


def normalize(xml_path: Path, deselected: int, version: str, commit: str, timestamp: str, exit_code: int = 0) -> dict:
    root = ET.parse(xml_path).getroot()
    tests = []
    for case in root.iter("testcase"):
        outcome = "passed"
        detail = ""
        for kind in ("error", "failure", "skipped"):
            node = case.find(kind)
            if node is not None:
                outcome = {"error": "error", "failure": "failed", "skipped": "skipped"}[kind]
                detail = node.get("message", "")
                break
        tests.append({"name": f'{case.get("classname", "")}::{case.get("name", "")}', "outcome": outcome, "detail": detail})
    if not tests:
        raise ValueError("pytest produced no test cases; no evidence report was generated")
    counts = {key: sum(test["outcome"] == key for test in tests) for key in ("passed", "failed", "error", "skipped")}
    counts["deselected"] = deselected
    return {"version": version, "commit": commit, "timestamp": timestamp, "exit_code": exit_code, "status": "PASS" if exit_code == 0 and not counts["failed"] and not counts["error"] else "FAIL", "counts": counts, "tests": tests}


def render(template: str, data: Path, output: Path) -> None:
    subprocess.run(["sc-compose", "render", "--strict", "--root", str(TEMPLATES), "--file", str(TEMPLATES / template), "--var-file", str(data), "--output", str(output)], check=True)


def main() -> int:
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT, text=True))
    version = next(package["version"] for package in metadata["packages"] if package["name"] == "sc-lint")
    source_status = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=all"], cwd=ROOT, text=True).splitlines()
    now = datetime.now(timezone.utc)
    timestamp = now.strftime("%Y-%m-%dT%H:%M:%SZ")
    name = f'{now.strftime("%Y%m%dT%H%M%SZ")}-v{version}-{commit[:8]}'
    with tempfile.TemporaryDirectory(prefix="sc-lint-integration-") as tmp:
        temp = Path(tmp)
        xml = temp / "pytest.xml"
        selections = temp / "selections.json"
        env = os.environ.copy()
        env["PYTHONPATH"] = os.pathsep.join(filter(None, [str(ROOT / ".github/scripts"), env.get("PYTHONPATH", "")]))
        env["SC_LINT_EVIDENCE_DESELECTIONS"] = str(selections)
        run = subprocess.run(["just", "test-adoption-integration", "-p", "integration_evidence_pytest", f"--junitxml={xml}"], cwd=ROOT, env=env, check=False)
        if not xml.is_file() or not selections.is_file():
            print("Integration run did not produce complete pytest results", file=sys.stderr)
            return run.returncode or 1
        results = normalize(xml, json.loads(selections.read_text())["deselected"], version, commit, timestamp, run.returncode)
        results["source_status"] = source_status
        out = REPORTS / name
        out.mkdir(parents=True, exist_ok=False)
        (out / "pytest.xml").write_bytes(xml.read_bytes())
        result_file = out / "results.json"
        result_file.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
        render("panel.xhtml.j2", result_file, out / "panel.xhtml")
        report_vars = out / "report.json"
        report_vars.write_text(json.dumps({"title": f"Integration tests: v{version}", "panels_html": (out / "panel.xhtml").read_text(encoding="utf-8")}, indent=2) + "\n", encoding="utf-8")
        render("report.html.j2", report_vars, out / "report.html")
        runs = []
        for result in sorted(REPORTS.glob("*/results.json"), reverse=True):
            data = json.loads(result.read_text(encoding="utf-8"))
            runs.append({"href": f"{result.parent.name}/report.html", "version": data["version"], "commit": data["commit"], "timestamp": data["timestamp"], "counts": data["counts"], "status": data["status"]})
        index_vars = REPORTS / "index.json"
        index_vars.write_text(json.dumps({"runs": runs}, indent=2) + "\n", encoding="utf-8")
        render("index.html.j2", index_vars, REPORTS / "index.html")
        print(f"Integration report: {out / 'report.html'}")
        return run.returncode


if __name__ == "__main__":
    raise SystemExit(main())
