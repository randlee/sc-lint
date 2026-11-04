"""Render native verification JSON results into a durable HTML report under site/reports.

Each target's JSON result renders to one XHTML panel through the pinned sc-compose wheel. The
panels are tiled into a primary report whose overview says how the run was triggered. Each run
is written to site/reports/release-native-verify/<stamp>-<tag>-run<id>/ beside its JSON results,
and the directory index lists every run.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from release_native_common import RESULT_SCHEMA_VERSION, is_full_sha

TARGET_PATTERN = re.compile(r"[A-Za-z0-9._-]+")
TEMPLATE_DIR = "release/native-verify"
REPORT_ROOT = Path("site/reports/release-native-verify")
REPORT_TITLE = "Release native verification"
DESCRIPTION = (
    "Each native target is verified by its own job on a GitHub-hosted runner for that target. The job "
    "downloads the archive and wheels for the tag, checks each against checksums.txt, runs the bundled "
    "binaries with --version, and installs each wheel under the target's Python to check its version. "
    "A prerelease run gates publication: release-preflight accepts it only while the prerelease commit is "
    "an ancestor of the release candidate and no artifact input changed between them."
)

Render = Callable[[str, dict], str]


class ReportError(ValueError):
    pass


def sc_compose_render(template: str, variables: dict, *, root: Path) -> str:
    """Render one repository template through the pinned sc-compose wheel, confined to root."""
    try:
        import sc_compose
    except ImportError as error:
        raise ReportError(
            "sc-compose wheel is not installed; provision it with .github/scripts/bootstrap_sc_compose.py"
        ) from error
    request = sc_compose.ComposeRequest(
        root=root,
        mode=sc_compose.ComposeMode.file(template),
        vars_input=variables,
        policy=sc_compose.ComposePolicy(strict_undeclared_variables=True),
    )
    return sc_compose.compose_file(request).rendered_text


def validate_result(result: object, source: str) -> None:
    if not isinstance(result, dict):
        raise ReportError(f"{source}: result must be a JSON object")
    if result.get("schema_version") != RESULT_SCHEMA_VERSION:
        raise ReportError(f"{source}: unsupported result schema_version {result.get('schema_version')!r}")
    for key in ("tag", "target", "error"):
        if not isinstance(result.get(key), str):
            raise ReportError(f"{source}: {key} must be a string")
    if not result["target"]:
        raise ReportError(f"{source}: target must not be empty")
    checks = result.get("checks")
    if not isinstance(checks, list):
        raise ReportError(f"{source}: checks must be a list")
    for check in checks:
        if not isinstance(check, dict) or not all(isinstance(check.get(key), str) for key in ("name", "detail", "status")):
            raise ReportError(f"{source}: each check needs string name, status and detail")
        if check["status"] not in ("pass", "fail"):
            raise ReportError(f"{source}: check status must be pass or fail, got {check['status']!r}")


def with_status(result: dict) -> dict:
    """A target passes only when it recorded no error and no failing check."""
    failed = bool(result["error"]) or any(check["status"] == "fail" for check in result["checks"])
    return {**result, "status": "FAIL" if failed else "PASS"}


def missing_result(target: str, tag: str) -> dict:
    return {
        "schema_version": RESULT_SCHEMA_VERSION,
        "tag": tag,
        "target": target,
        "checks": [],
        "error": "no result JSON was produced for this target; see its Native job log",
    }


def read_results(results_dir: Path, expected_targets: list[str], tag: str) -> list[dict]:
    """Load every expected target's result. An absent directory or missing result is a failing panel."""
    if not results_dir.is_dir() and not expected_targets:
        raise ReportError(f"results directory {results_dir} does not exist and no targets were expected")
    paths = sorted(results_dir.glob("*.json")) if results_dir.is_dir() else []
    loaded: dict[str, dict] = {}
    for path in paths:
        try:
            result = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ReportError(f"{path.name}: unreadable result: {error}") from error
        validate_result(result, path.name)
        target = result["target"]
        # A target becomes a panel file name, so it must be a plain name the matrix declared.
        if not TARGET_PATTERN.fullmatch(target) or target not in expected_targets:
            raise ReportError(f"{path.name}: target {target!r} is not one of the expected native targets")
        if target in loaded:
            raise ReportError(f"{path.name}: duplicate result for target {target}")
        loaded[target] = result
    ordered = [loaded[target] if target in loaded else missing_result(target, tag) for target in expected_targets]
    return [with_status(result) for result in ordered]


def overall_status(results: list[dict]) -> str:
    if not results or any(result["status"] == "FAIL" for result in results):
        return "FAIL"
    return "PASS"


def panel_variables(result: dict) -> dict:
    return {
        "title": result["target"],
        "target": result["target"],
        "status": result["status"],
        "status_class": result["status"].lower(),
        "error": result["error"],
        "checks": result["checks"],
    }


def render_panel(result: dict, *, render: Render) -> str:
    fragment = render(f"{TEMPLATE_DIR}/panel.xhtml.j2", panel_variables(result))
    try:
        ET.fromstring(fragment)  # a panel must be well-formed XHTML before it is tiled
    except ET.ParseError as error:
        raise ReportError(f"{result['target']}: panel is not well-formed XHTML: {error}") from error
    return fragment


def tag_slug(tag: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9./_-]*", tag):
        raise ReportError(f"tag {tag!r} cannot name a report directory")
    return tag.replace("/", "-")


def write_index(site_root: Path, *, render: Render, generated_at: str) -> None:
    runs = []
    for sidecar in site_root.glob("*/index.json"):
        runs.append(json.loads(sidecar.read_text(encoding="utf-8")))
    runs.sort(key=lambda run: run["generated_at"], reverse=True)
    index_html = render(f"{TEMPLATE_DIR}/index.html.j2", {"generated_at": generated_at, "runs": runs})
    (site_root / "index.html").write_text(index_html, encoding="utf-8")


def build_report(
    *,
    results_dir: Path,
    expected_targets: list[str],
    tag: str,
    source_workflow: str,
    commit: str,
    run_id: str,
    run_url: str,
    trigger: str,
    generated_at: datetime,
    site_root: Path,
    render: Render,
) -> Path:
    if not run_id.isdigit():
        raise ReportError(f"run id {run_id!r} must be numeric")
    if not is_full_sha(commit):
        raise ReportError(f"commit {commit!r} must be a full 40-character SHA")
    results = read_results(results_dir, expected_targets, tag)
    status = overall_status(results)
    generated = generated_at.astimezone(timezone.utc)
    generated_text = generated.isoformat(timespec="seconds")
    run_dir = site_root / f"{generated.strftime('%Y%m%dT%H%M%SZ')}-{tag_slug(tag)}-run{run_id}"
    panels = [render_panel(result, render=render) for result in results]

    (run_dir / "panels").mkdir(parents=True, exist_ok=True)
    (run_dir / "results").mkdir(parents=True, exist_ok=True)
    for result, fragment in zip(results, panels):
        (run_dir / "panels" / f"{result['target']}.xhtml").write_text(fragment, encoding="utf-8")
        (run_dir / "results" / f"{result['target']}.json").write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    overview = [
        {"label": "Tag", "value": tag},
        {"label": "Workflow", "value": source_workflow},
        {"label": "Commit", "value": commit},
        {"label": "Triggered by", "value": trigger},
        {"label": "Verify run", "value": f"{run_id} {run_url}".strip()},
        {"label": "Targets", "value": ", ".join(result["target"] for result in results)},
        {"label": "Generated", "value": generated_text},
    ]
    report_html = render(f"{TEMPLATE_DIR}/report.html.j2", {
        "title": f"{REPORT_TITLE}: {tag}",
        "status": status,
        "status_class": status.lower(),
        "generated_at": generated_text,
        "description": DESCRIPTION,
        "overview": overview,
        "panels_html": "\n".join(panels),
    })
    run_dir.mkdir(parents=True, exist_ok=True)
    (run_dir / "index.html").write_text(report_html, encoding="utf-8")
    sidecar = {
        "href": f"{run_dir.name}/index.html",
        "label": f"{tag} run {run_id}",
        "tag": tag,
        "commit": commit,
        "trigger": trigger,
        "status": status,
        "status_class": status.lower(),
        "generated_at": generated_text,
        "targets": [result["target"] for result in results],
    }
    (run_dir / "index.json").write_text(json.dumps(sidecar, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_index(site_root, render=render, generated_at=generated_text)
    return run_dir / "index.html"


def main(argv: list[str] | None = None) -> int:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results-dir", type=Path, required=True, help="directory holding one JSON result per target")
    parser.add_argument("--expected-targets", default="", help="comma-separated targets the matrix ran")
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-workflow", required=True, help="workflow whose completion triggered the verify run")
    parser.add_argument("--commit", required=True, help="commit the tag names, as resolved by the verify plan")
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--run-url", default="")
    parser.add_argument("--trigger", required=True, help="how the run was triggered, for the report overview")
    parser.add_argument("--site-root", type=Path, default=None, help=f"default: <repo>/{REPORT_ROOT}")
    parser.add_argument("--generated-at", default="", help="ISO-8601 timestamp; default: now (UTC)")
    parser.add_argument("--root", type=Path, default=root, help="repository root that confines template rendering")
    args = parser.parse_args(argv)
    def render(template: str, variables: dict) -> str:
        return sc_compose_render(template, variables, root=args.root)

    try:
        generated_at = datetime.fromisoformat(args.generated_at) if args.generated_at else datetime.now(timezone.utc)
        report = build_report(
            results_dir=args.results_dir,
            expected_targets=[target for target in args.expected_targets.split(",") if target],
            tag=args.tag,
            source_workflow=args.source_workflow,
            commit=args.commit,
            run_id=args.run_id,
            run_url=args.run_url,
            trigger=args.trigger,
            generated_at=generated_at,
            site_root=args.site_root or args.root / REPORT_ROOT,
            render=render,
        )
    except (ReportError, OSError, ValueError) as error:
        print(f"release-native-report: {error}", file=sys.stderr)
        return 1
    print(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
