"""Render the native verification report with sc-compose: one panel per result JSON, the report and its index."""
from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

TEMPLATES = Path(__file__).resolve().parents[2] / "release" / "native-verify"


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("--results", "--output", "--tag", "--commit", "--run-url"):
        parser.add_argument(name, required=True)
    parser.add_argument("--renderer", default="sc-compose")
    args = parser.parse_args(argv)
    results, out = sorted(Path(args.results).glob("*.json")), Path(args.output)
    (out / "panels").mkdir(parents=True, exist_ok=True)

    def compose(template: str, variables: Path | dict, output: Path) -> None:
        if isinstance(variables, dict):
            (var_file := out / f"{output.stem}.vars.json").write_text(json.dumps(variables), encoding="utf-8")
        subprocess.run([args.renderer, "render", "--strict", "--root", str(TEMPLATES), "--file", str(TEMPLATES / template),
                        "--var-file", str(var_file if isinstance(variables, dict) else variables), "--output", str(output)],
                       check=True, stdout=subprocess.DEVNULL)

    for result in results:
        compose("panel.xhtml.j2", result, out / "panels" / f"{result.stem}.xhtml")
    status = "PASS" if results and all(json.loads(r.read_text(encoding="utf-8"))["status"] == "PASS" for r in results) else "FAIL"
    run = {"tag": args.tag, "commit": args.commit, "status": status, "status_class": status.lower(),
           "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds")}
    compose("report.html.j2", {**run, "title": f"Release native verification: {args.tag}",
                               "description": f"Native matrix verification of {args.tag} at {args.commit}: {args.run_url}",
                               "overview": [{"label": key.title(), "value": run[key]} for key in ("tag", "commit")],
                               "panels_html": "\n".join((out / "panels" / f"{r.stem}.xhtml").read_text(encoding="utf-8") for r in results)},
            out / "report.html")
    compose("index.html.j2", {"generated_at": run["generated_at"],
                              "runs": [{**run, "href": "report.html", "label": args.tag, "trigger": args.run_url}]}, out / "index.html")


if __name__ == "__main__":
    main()
