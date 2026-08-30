#!/usr/bin/env python3
"""Install the repository-agnostic sc-lint adoption kit."""
from __future__ import annotations

import argparse
import difflib
import json
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MARKER_START = "# >>> sc-lint managed integration >>>"
MARKER_END = "# <<< sc-lint managed integration <<<"
RENAMED = {Path("README.md"): Path("README.sc-lint.md")}
TEMPLATES = {Path("templates/sc-lint.toml.j2"): Path("sc-lint.toml"), Path("templates/Justfile.import.j2"): Path("Justfile")}

def values(path: Path) -> dict:
    data = json.loads(path.read_text())
    required = {"minimum_version", "profiles", "ci", "analyzers"}
    if set(data) - {"minimum_version", "profiles", "ci", "analyzers", "test"} or required - set(data):
        raise ValueError("install input has missing or unknown fields")
    if not re.fullmatch(r"\d+\.\d+\.\d+", data["minimum_version"]):
        raise ValueError("minimum_version must be SemVer")
    return data

def render(template: Path, data: dict) -> str:
    analyzers = "\n".join(f"{name} = {json.dumps(value)}" for name, value in data["analyzers"].items())
    profiles = "\n".join(f"{name} = {json.dumps(value)}" for name, value in data["profiles"].items())
    layers = "\n".join(f"{name} = {json.dumps(value)}" for name, value in data.get("test", {}).items())
    return template.read_text().replace("{{ minimum_version }}", data["minimum_version"]).replace("{{ analyzers }}", analyzers).replace("{{ profiles }}", profiles).replace("{{ test_layers }}", layers)

def desired(data: dict, repo: Path) -> dict[Path, str | bytes]:
    result: dict[Path, str | bytes] = {}
    for source in ROOT.rglob("*"):
        if not source.is_file() or source.name == "install.py" or source.relative_to(ROOT).parts[0] in {"templates", ".claude-plugin"}:
            continue
        result[RENAMED.get(source.relative_to(ROOT), source.relative_to(ROOT))] = source.read_bytes()
    result[Path("sc-lint.toml")] = render(ROOT / "templates/sc-lint.toml.j2", data)
    justfile = repo / "Justfile"
    original = justfile.read_text() if justfile.exists() else ""
    if original.count(MARKER_START) != original.count(MARKER_END) or original.count(MARKER_START) > 1:
        raise RuntimeError("Justfile marker conflict")
    block = render(ROOT / "templates/Justfile.import.j2", data)
    if MARKER_START in original:
        original = re.sub(re.escape(MARKER_START) + r".*?" + re.escape(MARKER_END), block.strip(), original, flags=re.S)
    else:
        original = (original.rstrip() + "\n\n" + block).lstrip("\n")
    result[Path("Justfile")] = original
    return result

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("repo", type=Path)
    args = parser.parse_args()
    try: data, targets = values(args.input), desired(values(args.input), args.repo)
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"conflict: {error}", file=sys.stderr); return 2
    changes = []
    for relative, content in targets.items():
        destination = args.repo / relative
        current = destination.read_bytes() if destination.exists() else b""
        expected = content.encode() if isinstance(content, str) else content
        if current != expected:
            changes.append((destination, current.decode(errors="replace"), expected.decode(errors="replace")))
    if args.dry_run:
        for path, old, new in changes:
            print("".join(difflib.unified_diff(old.splitlines(True), new.splitlines(True), fromfile=str(path), tofile=str(path))))
        return 1 if changes else 0
    for path, _, new in changes:
        path.parent.mkdir(parents=True, exist_ok=True); path.write_text(new)
    return 0
if __name__ == "__main__": raise SystemExit(main())
