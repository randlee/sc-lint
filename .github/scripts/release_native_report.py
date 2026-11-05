"""Render native verification panels, report, and index with sc-compose."""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

TEMPLATES = Path(__file__).resolve().parents[2] / "release" / "native-verify"


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("--results", "--output", "--tag", "--commit", "--run-url"):
        parser.add_argument(name, required=True)
    args = parser.parse_args(argv)
    results, out = sorted(Path(args.results).glob("*.json")), Path(args.output)
    panels = out / "panels"
    panels.mkdir(parents=True, exist_ok=True)
    for result in results:
        panel = panels / f"{result.stem}.xhtml"
        subprocess.run(["sc-compose", "render", "--strict", "--root", str(TEMPLATES), "--file", str(TEMPLATES / "panel.xhtml.j2"),
                        "--var-file", str(result), "--output", str(panel)], check=True, stdout=subprocess.DEVNULL)
    vars_file = out / "report.json"
    vars_file.write_text(json.dumps({"title": f"Release native verification: {args.tag}",
                                     "panels_html": "\n".join(panel.read_text(encoding="utf-8") for panel in sorted(panels.glob("*.xhtml"))),
                                     "runs": [{"href": "report.html", "label": args.tag, "tag": args.tag,
                                               "commit": args.commit, "trigger": args.run_url}]}), encoding="utf-8")
    for name in ("report", "index"):
        subprocess.run(["sc-compose", "render", "--strict", "--root", str(TEMPLATES), "--file", str(TEMPLATES / f"{name}.html.j2"),
                        "--var-file", str(vars_file), "--output", str(out / f"{name}.html")], check=True, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
