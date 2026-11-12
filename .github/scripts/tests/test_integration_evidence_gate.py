"""Exercise the release evidence gate against real committed Git trees."""
import json
import subprocess
import sys
from pathlib import Path

GATE = Path(__file__).resolve().parents[1] / "integration_evidence_gate.py"


def git(repo, *args):
    return subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True, text=True).stdout.strip()


def gate(repo, version="0.8.8", ref="HEAD", candidate="release-candidate-v0.8.8"):
    return subprocess.run([sys.executable, str(GATE), "--version", version, "--ref", ref, "--candidate-ref", candidate], cwd=repo, capture_output=True, text=True, check=False)


def test_gate_rejects_missing_and_failed_latest_report(tmp_path):
    git(tmp_path, "init")
    git(tmp_path, "config", "user.email", "test@example.invalid")
    git(tmp_path, "config", "user.name", "Test")
    (tmp_path / "README").write_text("release\n")
    git(tmp_path, "add", "README")
    git(tmp_path, "commit", "-m", "empty evidence")
    candidate_commit = git(tmp_path, "rev-parse", "HEAD")
    git(tmp_path, "tag", "-a", "release-candidate-v0.8.8", "-m", "candidate")
    missing = gate(tmp_path)
    assert missing.returncode != 0
    assert "no integration evidence report" in missing.stderr

    report = tmp_path / "site/reports/integration/20261112T001553Z-v0.8.8-abcdef12"
    report.mkdir(parents=True)
    data = {"commit": candidate_commit, "version": "0.8.8", "status": "PASS", "exit_code": 0, "counts": {"passed": 1, "failed": 0, "error": 0, "skipped": 0}, "tests": [{"outcome": "passed"}]}
    (report / "results.json").write_text(json.dumps(data))
    for name in ("report.html", "panel.xhtml", "pytest.xml"):
        (report / name).write_text("evidence")
    git(tmp_path, "add", "site")
    git(tmp_path, "commit", "-m", "passing evidence")
    passing_ref = git(tmp_path, "rev-parse", "HEAD")
    assert gate(tmp_path).returncode == 0
    assert gate(tmp_path, "0.8.9", candidate="release-candidate-v0.8.9").returncode != 0
    assert gate(tmp_path, candidate="HEAD").returncode != 0

    # A same-version report for a different real commit must fail even if PASS.
    data["commit"] = passing_ref
    (report / "results.json").write_text(json.dumps(data))
    git(tmp_path, "add", "site")
    git(tmp_path, "commit", "-m", "stale evidence")
    stale = gate(tmp_path)
    assert stale.returncode != 0
    assert "commit does not match" in stale.stderr
    assert gate(tmp_path, ref=passing_ref).returncode == 0

    for invalid in (None, "", candidate_commit[:6], candidate_commit + "0"):
        data["commit"] = invalid
        (report / "results.json").write_text(json.dumps(data))
        git(tmp_path, "add", "site")
        git(tmp_path, "commit", "-m", "invalid evidence commit")
        assert gate(tmp_path).returncode != 0

    # A sufficiently long prefix of the tagged commit is accepted.
    data["commit"] = candidate_commit[:8]
    (report / "results.json").write_text(json.dumps(data))
    git(tmp_path, "add", "site")
    git(tmp_path, "commit", "-m", "matching abbreviated evidence")
    assert gate(tmp_path).returncode == 0

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
    assert '--version "$RELEASE_VERSION" --ref HEAD --candidate-ref "release-candidate-v${RELEASE_VERSION}"' in preflight
    assert '--version "$RELEASE_VERSION" --ref "$RELEASE_SHA" --candidate-ref "release-candidate-v${RELEASE_VERSION}"' in release
    assert release.index("Require passing integration evidence") < release.index("Ensure tag is correct or create it")
