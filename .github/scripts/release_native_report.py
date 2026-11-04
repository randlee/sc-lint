"""Render native verification JSON through the repository's sc-compose templates."""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

TEMPLATE_DIR = "release/native-verify"
REPORT_ROOT = Path("site/reports/release-native-verify")
REPORT_TITLE = "Release native verification"
DESCRIPTION = (
    "Each native target checks its archive and wheels on a GitHub-hosted runner for that target. "
    "The prerelease matrix gates publication while its commit is an ancestor of the release candidate "
    "and no artifact input changed."
)
Render = Callable[[str, dict], str]


def sc_compose_render(template: str, variables: dict, *, root: Path) -> str:
    import sc_compose

    request = sc_compose.ComposeRequest(
        root=root,
        mode=sc_compose.ComposeMode.file(template),
        vars_input=variables,
        policy=sc_compose.ComposePolicy(strict_undeclared_variables=True),
    )
    return sc_compose.compose_file(request).rendered_text


def read_results(directory: Path, targets: list[str], tag: str) -> list[dict]:
    results = []
    for target in targets:
        path = directory / f"{target}.json"
        result = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {
            "schema_version": 1, "tag": tag, "target": target, "checks": [],
            "error": "no result JSON was produced for this target; see its Native job log",
        }
        failed = bool(result["error"]) or any(check["status"] == "fail" for check in result["checks"])
        results.append({**result, "status": "FAIL" if failed else "PASS"})
    return results


def build_report(
    *, results_dir: Path, expected_targets: list[str], tag: str, source_workflow: str,
    commit: str, run_id: str, run_url: str, trigger: str, generated_at: datetime,
    site_root: Path, render: Render,
) -> Path:
    generated = generated_at.astimezone(timezone.utc)
    generated_text = generated.isoformat(timespec="seconds")
    results = read_results(results_dir, expected_targets, tag)
    status = "FAIL" if not results or any(result["status"] == "FAIL" for result in results) else "PASS"
    run_dir = site_root / f"{generated:%Y%m%dT%H%M%SZ}-{tag.replace('/', '-')}-run{run_id}"
    (run_dir / "panels").mkdir(parents=True, exist_ok=True)
    (run_dir / "results").mkdir(parents=True, exist_ok=True)
    panels = []
    for result in results:
        variables = {**result, "title": result["target"], "status_class": result["status"].lower()}
        panel = render(f"{TEMPLATE_DIR}/panel.xhtml.j2", variables)
        panels.append(panel)
        (run_dir / "panels" / f"{result['target']}.xhtml").write_text(panel, encoding="utf-8")
        (run_dir / "results" / f"{result['target']}.json").write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    overview = [
        {"label": "Tag", "value": tag}, {"label": "Workflow", "value": source_workflow},
        {"label": "Commit", "value": commit}, {"label": "Triggered by", "value": trigger},
        {"label": "Verify run", "value": f"{run_id} {run_url}".strip()},
        {"label": "Targets", "value": ", ".join(expected_targets)}, {"label": "Generated", "value": generated_text},
    ]
    report_html = render(f"{TEMPLATE_DIR}/report.html.j2", {
        "title": f"{REPORT_TITLE}: {tag}", "status": status, "status_class": status.lower(),
        "generated_at": generated_text, "description": DESCRIPTION, "overview": overview,
        "panels_html": "\n".join(panels),
    })
    report = run_dir / "index.html"
    report.write_text(report_html, encoding="utf-8")
    sidecar = {
        "href": f"{run_dir.name}/index.html", "label": f"{tag} run {run_id}", "tag": tag,
        "commit": commit, "trigger": trigger, "status": status, "status_class": status.lower(),
        "generated_at": generated_text, "targets": expected_targets,
    }
    (run_dir / "index.json").write_text(json.dumps(sidecar, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    runs = [json.loads(path.read_text(encoding="utf-8")) for path in site_root.glob("*/index.json")]
    runs.sort(key=lambda row: row["generated_at"], reverse=True)
    (site_root / "index.html").write_text(
        render(f"{TEMPLATE_DIR}/index.html.j2", {"generated_at": generated_text, "runs": runs}),
        encoding="utf-8",
    )
    return report


def main(argv: list[str] | None = None) -> int:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results-dir", type=Path, required=True)
    parser.add_argument("--expected-targets", default="")
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-workflow", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--run-url", default="")
    parser.add_argument("--trigger", required=True)
    parser.add_argument("--site-root", type=Path)
    parser.add_argument("--generated-at", default="")
    parser.add_argument("--root", type=Path, default=root)
    args = parser.parse_args(argv)
    render = lambda template, variables: sc_compose_render(template, variables, root=args.root)
    generated_at = datetime.fromisoformat(args.generated_at) if args.generated_at else datetime.now(timezone.utc)
    report = build_report(
        results_dir=args.results_dir,
        expected_targets=[target for target in args.expected_targets.split(",") if target],
        tag=args.tag, source_workflow=args.source_workflow, commit=args.commit, run_id=args.run_id,
        run_url=args.run_url, trigger=args.trigger, generated_at=generated_at,
        site_root=args.site_root or args.root / REPORT_ROOT, render=render,
    )
    print(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
