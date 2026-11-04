"""Deterministic tests for rendering native verification results into the durable report."""
import html
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_native_report as report

ROOT = Path(__file__).resolve().parents[3]
TAG = "prerelease/v1.2.3"
COMMIT = "a" * 40
X86 = "x86_64-unknown-linux-gnu"
ARM = "aarch64-apple-darwin"
GENERATED = datetime(2026, 11, 4, 18, 0, 0, tzinfo=timezone.utc)
LATER = datetime(2026, 11, 5, 9, 30, 0, tzinfo=timezone.utc)


class Recorder:
    """Stands in for the sc-compose wheel: records each render and returns text the tests can check."""

    def __init__(self):
        self.renders = []

    def __call__(self, template, variables):
        self.renders.append((template, variables))
        if template.endswith("panel.xhtml.j2"):
            return (
                '<section xmlns="http://www.w3.org/1999/xhtml" class="native-panel-{}">'
                "<h2>{}</h2><p>{}</p></section>"
            ).format(variables["status_class"], html.escape(variables["title"]), variables["status"])
        if template.endswith("report.html.j2"):
            return f"REPORT {variables['title']}\n{variables['panels_html']}"
        return "INDEX"

    def templates(self):
        return [template for template, _ in self.renders]

    def last(self, suffix):
        return next(variables for template, variables in reversed(self.renders) if template.endswith(suffix))


def write_result(directory: Path, target: str, *, checks=(), error="", tag=TAG):
    directory.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": 1,
        "tag": tag,
        "target": target,
        "checks": [{"name": name, "status": status, "detail": f"{name} detail"} for name, status in checks],
        "error": error,
    }
    (directory / f"{target}.json").write_text(json.dumps(payload), encoding="utf-8")


def build(tmp_path, *, expected, render, generated=GENERATED, **overrides):
    arguments = {
        "results_dir": tmp_path / "results",
        "expected_targets": expected,
        "tag": TAG,
        "source_workflow": "Prerelease Archive",
        "commit": COMMIT,
        "run_id": "4242",
        "run_url": "https://github.com/randlee/sc-lint/actions/runs/4242",
        "trigger": "workflow_run of Prerelease Archive run 4241",
        "generated_at": generated,
        "site_root": tmp_path / "site",
        "render": render,
    }
    arguments.update(overrides)
    return report.build_report(**arguments)


def test_missing_expected_target_is_reported_as_a_failing_panel(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("sha256 archive", "pass")])
    results = report.read_results(tmp_path / "results", [X86, ARM], TAG)

    assert [result["target"] for result in results] == [X86, ARM]
    assert [result["status"] for result in results] == ["PASS", "FAIL"]
    assert "no result JSON was produced" in results[1]["error"]


@pytest.mark.parametrize("checks, error", [
    ([("sha256 archive", "fail")], ""),
    ([], "SHA256 mismatch for archive"),
])
def test_failing_check_or_error_fails_the_target(tmp_path, checks, error):
    write_result(tmp_path / "results", X86, checks=checks, error=error)
    [result] = report.read_results(tmp_path / "results", [X86], TAG)

    assert result["status"] == "FAIL"
    assert report.overall_status([result]) == "FAIL"


def test_all_targets_passing_passes_the_report(tmp_path):
    for target in (X86, ARM):
        write_result(tmp_path / "results", target, checks=[("native host", "pass"), ("wheel", "pass")])
    results = report.read_results(tmp_path / "results", [X86, ARM], TAG)

    assert [result["status"] for result in results] == ["PASS", "PASS"]
    assert report.overall_status(results) == "PASS"


def test_no_results_at_all_is_a_failing_report():
    assert report.overall_status([]) == "FAIL"


def test_targets_outside_the_expected_matrix_are_rejected(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])
    write_result(tmp_path / "results", "extra-target", checks=[("native host", "pass")])

    with pytest.raises(report.ReportError, match="is not one of the expected native targets"):
        report.read_results(tmp_path / "results", [X86], TAG)


def test_target_names_must_be_plain_file_name_segments(tmp_path):
    write_result(tmp_path / "results", "bad name", checks=[("native host", "pass")])

    with pytest.raises(report.ReportError, match="is not one of the expected native targets"):
        report.read_results(tmp_path / "results", ["bad name"], TAG)


def test_absent_results_directory_fails_every_expected_target(tmp_path):
    results = report.read_results(tmp_path / "absent", [X86, ARM], TAG)

    assert [result["status"] for result in results] == ["FAIL", "FAIL"]
    assert report.overall_status(results) == "FAIL"


def test_absent_results_directory_without_expected_targets_is_an_error(tmp_path):
    with pytest.raises(report.ReportError, match="does not exist and no targets were expected"):
        report.read_results(tmp_path / "absent", [], TAG)


@pytest.mark.parametrize("mutate, message", [
    (lambda payload: payload.update(schema_version=2), "unsupported result schema_version"),
    (lambda payload: payload.update(target=""), "target must not be empty"),
    (lambda payload: payload.update(checks="native host"), "checks must be a list"),
    (lambda payload: payload["checks"].append({"name": "x", "status": "maybe", "detail": "d"}), "check status must be pass or fail"),
    (lambda payload: payload["checks"].append({"name": "x", "detail": "d"}), "each check needs string name"),
    (lambda payload: payload.update(error=None), "error must be a string"),
])
def test_malformed_result_is_rejected(tmp_path, mutate, message):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])
    path = tmp_path / "results" / f"{X86}.json"
    payload = json.loads(path.read_text(encoding="utf-8"))
    mutate(payload)
    path.write_text(json.dumps(payload), encoding="utf-8")

    with pytest.raises(report.ReportError, match=message):
        report.read_results(tmp_path / "results", [X86], TAG)


def test_unreadable_json_is_rejected(tmp_path):
    (tmp_path / "results").mkdir()
    (tmp_path / "results" / "broken.json").write_text("{not json", encoding="utf-8")

    with pytest.raises(report.ReportError, match="unreadable result"):
        report.read_results(tmp_path / "results", [], TAG)


def test_duplicate_target_result_is_rejected(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])
    (tmp_path / "results" / "copy.json").write_text(
        (tmp_path / "results" / f"{X86}.json").read_text(encoding="utf-8"), encoding="utf-8"
    )
    with pytest.raises(report.ReportError, match="duplicate result"):
        report.read_results(tmp_path / "results", [X86], TAG)


def test_one_panel_per_target_is_tiled_into_one_report_with_an_overview(tmp_path):
    recorder = Recorder()
    write_result(tmp_path / "results", X86, checks=[("sha256 archive", "pass")])
    write_result(tmp_path / "results", ARM, checks=[("sha256 archive", "fail")])

    output = build(tmp_path, expected=[X86, ARM], render=recorder)

    assert recorder.templates().count("release/native-verify/panel.xhtml.j2") == 2
    assert output == tmp_path / "site" / "20261104T180000Z-prerelease-v1.2.3-run4242" / "index.html"
    report_variables = recorder.last("report.html.j2")
    assert report_variables["status"] == "FAIL"
    assert report_variables["panels_html"].count("native-panel-") == 2
    assert f'class="native-panel-pass"' in report_variables["panels_html"]
    assert f'class="native-panel-fail"' in report_variables["panels_html"]
    overview = {row["label"]: row["value"] for row in report_variables["overview"]}
    assert overview["Tag"] == TAG
    assert overview["Workflow"] == "Prerelease Archive"
    assert overview["Commit"] == COMMIT
    assert overview["Triggered by"] == "workflow_run of Prerelease Archive run 4241"
    assert overview["Verify run"] == "4242 https://github.com/randlee/sc-lint/actions/runs/4242"
    assert overview["Targets"] == f"{X86}, {ARM}"
    assert "release-preflight accepts it only while" in report_variables["description"]


def test_run_directory_holds_panels_json_and_sidecar(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])

    output = build(tmp_path, expected=[X86], render=Recorder())

    run_dir = output.parent
    assert (run_dir / "panels" / f"{X86}.xhtml").read_text(encoding="utf-8").startswith("<section")
    stored = json.loads((run_dir / "results" / f"{X86}.json").read_text(encoding="utf-8"))
    assert stored["status"] == "PASS"
    sidecar = json.loads((run_dir / "index.json").read_text(encoding="utf-8"))
    assert sidecar["href"] == f"{run_dir.name}/index.html"
    assert sidecar["status"] == "PASS"
    assert sidecar["targets"] == [X86]


def test_index_lists_runs_newest_first(tmp_path):
    recorder = Recorder()
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])
    build(tmp_path, expected=[X86], render=recorder, generated=GENERATED)
    build(tmp_path, expected=[X86], render=recorder, generated=LATER, run_id="4300")

    index_variables = recorder.last("index.html.j2")
    assert index_variables["generated_at"] == LATER.isoformat(timespec="seconds")
    assert [run["generated_at"] for run in index_variables["runs"]] == [
        LATER.isoformat(timespec="seconds"),
        GENERATED.isoformat(timespec="seconds"),
    ]
    assert (tmp_path / "site" / "index.html").read_text(encoding="utf-8") == "INDEX"


@pytest.mark.parametrize("commit", ["", "abc123", "A" * 40, "a" * 39, "g" * 40])
def test_commit_must_be_a_full_lowercase_sha(tmp_path, commit):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])
    with pytest.raises(report.ReportError, match="must be a full 40-character SHA"):
        build(tmp_path, expected=[X86], render=Recorder(), commit=commit)


@pytest.mark.parametrize("run_id, tag", [("run-4", TAG), ("", TAG), ("4242", "prerelease/v1 bad")])
def test_run_id_and_tag_must_name_a_report_directory(tmp_path, run_id, tag):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])
    with pytest.raises(report.ReportError):
        build(tmp_path, expected=[X86], render=Recorder(), run_id=run_id, tag=tag)


def test_malformed_panel_fails_closed_with_a_report_error(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")])

    def broken(template, variables):
        return "<section><h2>unclosed"

    with pytest.raises(report.ReportError, match="not well-formed XHTML"):
        build(tmp_path, expected=[X86], render=broken)


HOSTILE = "</p><script>alert('x')</script><img src=x onerror=\"alert(1)\"> & \"more\""


def test_hostile_text_is_escaped_by_the_real_renderer(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass")], error=HOSTILE)
    results_dir = tmp_path / "results"
    payload = json.loads((results_dir / f"{X86}.json").read_text(encoding="utf-8"))
    payload["checks"][0]["detail"] = HOSTILE
    payload["checks"][0]["name"] = HOSTILE
    (results_dir / f"{X86}.json").write_text(json.dumps(payload), encoding="utf-8")

    output = build(
        tmp_path, expected=[X86], render=lambda template, variables: report.sc_compose_render(template, variables, root=ROOT)
    )

    text = output.read_text(encoding="utf-8")
    assert "<script>" not in text
    assert "<img" not in text
    assert "onerror=\"" not in text
    assert "&lt;script&gt;alert" in text
    assert "&lt;img src=x onerror=" in text
    assert "Overview" in text
    panel = (output.parent / "panels" / f"{X86}.xhtml").read_text(encoding="utf-8")
    assert f"<code>{X86}</code>" in panel
    assert "<script>" not in panel


def test_real_templates_produce_well_formed_panels_and_a_tiled_report(tmp_path):
    write_result(tmp_path / "results", X86, checks=[("native host", "pass"), ("binary sc-lint", "pass")])
    write_result(tmp_path / "results", ARM, error="SHA256 mismatch")

    output = build(
        tmp_path, expected=[X86, ARM],
        render=lambda template, variables: report.sc_compose_render(template, variables, root=ROOT),
    )

    text = output.read_text(encoding="utf-8")
    assert text.count('class="native-panel native-panel-pass"') == 1
    assert text.count('class="native-panel native-panel-fail"') == 1
    assert "SHA256 mismatch" in text
    assert "Triggered by" in text
    assert (output.parent / "panels" / f"{ARM}.xhtml").read_text(encoding="utf-8").count("native-panel-fail") == 1


def test_cli_fails_cleanly_for_a_missing_results_directory(tmp_path, capsys):
    status = report.main([
        "--results-dir", str(tmp_path / "absent"), "--tag", TAG, "--commit", COMMIT,
        "--run-id", "4242", "--source-workflow", "workflow_dispatch", "--trigger", "workflow_dispatch", "--root", str(ROOT),
        "--site-root", str(tmp_path / "site"),
    ])
    assert status == 1


def test_workflow_renders_and_publishes_the_native_report():
    text = (ROOT / ".github/workflows/release-native-verify.yml").read_text(encoding="utf-8")
    assert '--result "native-results/${RELEASE_TARGET}.json"' in text
    assert "if: ${{ always() }}" in text
    assert "if: ${{ always() && needs.plan.outputs.verify == 'true' }}" in text
    assert "name: native-results-${{ matrix.target }}" in text
    assert "  report:" in text
    assert "bootstrap_sc_compose.py --venv" in text
    assert ".github/scripts/release_native_report.py" in text
    assert "--expected-targets" in text
    assert "site/reports/release-native-verify" in text
    assert "GITHUB_STEP_SUMMARY" in text
