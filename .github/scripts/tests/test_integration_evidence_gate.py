"""Exercise the release evidence gate against real committed Git trees."""
import json
import subprocess
import sys
from pathlib import Path

GATE = Path(__file__).resolve().parents[1] / "integration_evidence_gate.py"


def git(repo, *args):
    return subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True, text=True).stdout.strip()


def gate(repo, version="0.8.8", ref="HEAD"):
    return subprocess.run([sys.executable, str(GATE), "--version", version, "--ref", ref], cwd=repo, capture_output=True, text=True, check=False)


def test_gate_rejects_missing_and_failed_latest_report(tmp_path):
    git(tmp_path, "init")
    git(tmp_path, "config", "user.email", "test@example.invalid")
    git(tmp_path, "config", "user.name", "Test")
    (tmp_path / "README").write_text("release\n")
    git(tmp_path, "add", "README")
    git(tmp_path, "commit", "-m", "empty evidence")
    missing = gate(tmp_path)
    assert missing.returncode != 0
    assert "no integration evidence report" in missing.stderr

    report = tmp_path / "site/reports/integration/20261112T001553Z-v0.8.8-abcdef12"
    report.mkdir(parents=True)
    data = {"version": "0.8.8", "status": "PASS", "exit_code": 0, "counts": {"passed": 1, "failed": 0, "error": 0, "skipped": 0}, "tests": [{"outcome": "passed"}]}
    (report / "results.json").write_text(json.dumps(data))
    for name in ("report.html", "panel.xhtml", "pytest.xml"):
        (report / name).write_text("evidence")
    git(tmp_path, "add", "site")
    git(tmp_path, "commit", "-m", "passing evidence")
    passing_ref = git(tmp_path, "rev-parse", "HEAD")
    assert gate(tmp_path).returncode == 0
    assert gate(tmp_path, "0.8.9").returncode != 0

    later = report.parent / "20261112T001554Z-v0.8.8-abcdef12"
    later.mkdir()
    data["status"] = "FAIL"
    (later / "results.json").write_text(json.dumps(data))
    for name in ("report.html", "panel.xhtml", "pytest.xml"):
        (later / name).write_text("evidence")
    git(tmp_path, "add", "site")
    git(tmp_path, "commit", "-m", "failing later evidence")
    failed = gate(tmp_path)
    assert failed.returncode != 0
    assert "not PASS" in failed.stderr
    assert gate(tmp_path, ref=passing_ref).returncode == 0


def test_release_workflows_wire_evidence_gate():
    root = Path(__file__).resolve().parents[3]
    preflight = (root / ".github/workflows/release-preflight.yml").read_text()
    release = (root / ".github/workflows/release.yml").read_text()
    assert 'record integration-evidence-gate "${INTEGRATION_EVIDENCE_GATE}"' in preflight
    assert '--version "$RELEASE_VERSION" --ref HEAD' in preflight
    assert '--version "$RELEASE_VERSION" --ref "$RELEASE_SHA"' in release
    assert release.index("Require passing integration evidence") < release.index("Ensure tag is correct or create it")
