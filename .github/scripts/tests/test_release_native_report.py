"""Native JSON results are rendered by the checked-in sc-compose templates."""
import html
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_native_report as report

ROOT = Path(__file__).resolve().parents[3]
TAG = "prerelease/v1.2.3"
COMMIT = "a" * 40
TARGETS = ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"]
GENERATED = datetime(2026, 11, 4, 18, 0, 0, tzinfo=timezone.utc)


def write_result(directory, target, *, error="", status="pass", detail="ok"):
    directory.mkdir(parents=True, exist_ok=True)
    result = {
        "schema_version": 1, "tag": TAG, "target": target, "error": error,
        "checks": [{"name": "native verify", "status": status, "detail": detail}],
    }
    (directory / f"{target}.json").write_text(json.dumps(result), encoding="utf-8")


def build(tmp_path, *, render, generated=GENERATED):
    return report.build_report(
        results_dir=tmp_path / "results", expected_targets=TARGETS, tag=TAG,
        source_workflow="Prerelease Archive", commit=COMMIT, run_id="4242",
        run_url="https://github.com/randlee/sc-lint/actions/runs/4242",
        trigger="workflow_run of Prerelease Archive run 4241", generated_at=generated,
        site_root=tmp_path / "site", render=render,
    )


def test_missing_target_becomes_a_failure_panel_and_failed_results_stay_failed(tmp_path):
    write_result(tmp_path / "results", TARGETS[0], error="checksum mismatch")
    results = report.read_results(tmp_path / "results", TARGETS, TAG)

    assert [result["status"] for result in results] == ["FAIL", "FAIL"]
    assert results[0]["error"] == "checksum mismatch"
    assert "no result JSON was produced" in results[1]["error"]


def test_report_tiles_targets_and_keeps_per_run_results_and_index(tmp_path):
    write_result(tmp_path / "results", TARGETS[0])
    write_result(tmp_path / "results", TARGETS[1])
    rendered = []

    def render(template, variables):
        rendered.append((template, variables))
        return json.dumps(variables) if template.endswith("report.html.j2") else "rendered"

    output = build(tmp_path, render=render)

    assert output == tmp_path / "site" / "20261104T180000Z-prerelease-v1.2.3-run4242" / "index.html"
    assert (output.parent / "panels" / f"{TARGETS[0]}.xhtml").read_text() == "rendered"
    assert json.loads((output.parent / "results" / f"{TARGETS[1]}.json").read_text())["status"] == "PASS"
    assert (tmp_path / "site" / "index.html").read_text() == "rendered"
    report_vars = next(variables for name, variables in rendered if name.endswith("report.html.j2"))
    assert report_vars["status"] == "PASS"
    assert report_vars["panels_html"].count("rendered") == 2
    assert {row["label"]: row["value"] for row in report_vars["overview"]}["Commit"] == COMMIT


HOSTILE = '</p><script>alert(1)</script><img src=x onerror="alert(1)"> & "text"'


def test_sc_compose_escapes_result_text(tmp_path):
    write_result(tmp_path / "results", TARGETS[0], detail=HOSTILE)
    write_result(tmp_path / "results", TARGETS[1])
    render = lambda template, variables: report.sc_compose_render(template, variables, root=ROOT)

    output = build(tmp_path, render=render)
    text = output.read_text(encoding="utf-8")
    panel = (output.parent / "panels" / f"{TARGETS[0]}.xhtml").read_text(encoding="utf-8")
    assert "<script>" not in text + panel and "<img" not in text + panel
    assert html.escape(HOSTILE) in text + panel


def test_workflow_renders_and_uploads_the_report():
    text = (ROOT / ".github/workflows/release-native-verify.yml").read_text(encoding="utf-8")
    assert ".github/scripts/release_native_report.py" in text
    assert "bootstrap_sc_compose.py --venv" in text
    assert "site/reports/release-native-verify" in text
    assert "name: native-verify-report" in text
