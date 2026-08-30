#!/usr/bin/env python3
"""Install the repository-agnostic sc-lint adoption kit."""
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MARKER_START = "# >>> sc-lint managed integration >>>"
MARKER_END = "# <<< sc-lint managed integration <<<"
MANIFEST = Path(".sc-lint/installed.json")
RENAMED = {Path("README.md"): Path("README.sc-lint.md")}
TEMPLATES = {
    Path("templates/sc-lint.toml.j2"): Path("sc-lint.toml"),
    Path("templates/Justfile.import.j2"): Path("Justfile"),
    Path("templates/sc-lint-workflow.yml.j2"): Path(".github/workflows/sc-lint.yml"),
}


def _schema_value(value: object, schema: dict, label: str) -> None:
    kind = schema.get("type")
    if kind == "object" and not isinstance(value, dict): raise ValueError(f"{label} must be an object")
    if kind == "string" and not isinstance(value, str): raise ValueError(f"{label} must be a string")
    if kind == "boolean" and type(value) is not bool: raise ValueError(f"{label} must be a boolean")
    if "pattern" in schema and isinstance(value, str) and not re.fullmatch(schema["pattern"], value): raise ValueError(f"{label} does not match {schema['pattern']}")


def values(path: Path) -> dict:
    data = json.loads(path.read_text())
    schema = json.loads((ROOT / "install.schema.json").read_text())
    _schema_value(data, schema, "install input")
    properties = schema["properties"]
    unknown = set(data) - set(properties)
    missing = set(schema["required"]) - set(data)
    if unknown or missing: raise ValueError(f"install input has missing or unknown fields: {sorted(missing | unknown)}")
    for key, value in data.items(): _schema_value(value, properties[key], key)
    return data


def toml_value(value: object) -> str:
    if isinstance(value, dict): return "{ " + ", ".join(f"{key} = {toml_value(item)}" for key, item in value.items()) + " }"
    if isinstance(value, list): return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, bool): return str(value).lower()
    return json.dumps(value)


def render(template: Path, data: dict) -> str:
    analyzers = "\n".join(f"{name} = {toml_value(value)}" for name, value in data["analyzers"].items())
    profiles = "\n\n".join(f'[[tool.sc-lint.lint]]\nname = {json.dumps(name)}\ncommand = {json.dumps(command)}' for name, command in data["profiles"].items())
    layers = "\n\n".join(f'[[tool.sc-lint.test]]\nname = {json.dumps(name)}\ncommand = {json.dumps(command)}' for name, command in data.get("test", {}).items())
    runners = [ {"linux":"ubuntu-latest", "macos":"macos-latest", "windows":"windows-latest"}.get(item, item) for item in data["ci"]["os"] ]
    return (template.read_text().replace("{{ minimum_version }}", data["minimum_version"]).replace("{{ analyzers }}", analyzers)
            .replace("{{ profiles }}", profiles).replace("{{ test_layers }}", layers).replace("{{ ci_os }}", json.dumps(runners)))


def package_files() -> dict[Path, bytes]:
    return {Path("plugins/sc-lint") / source.relative_to(ROOT): source.read_bytes() for source in ROOT.rglob("*") if source.is_file()}


def desired(data: dict, repo: Path) -> dict[Path, bytes]:
    result = package_files()
    for source in ROOT.rglob("*"):
        if not source.is_file() or source.relative_to(ROOT) in TEMPLATES or source.name in {"install.py", "install.schema.json"} or source.relative_to(ROOT).parts[0] in {"templates", ".claude-plugin"}: continue
        result[RENAMED.get(source.relative_to(ROOT), source.relative_to(ROOT))] = source.read_bytes()
    for source, target in TEMPLATES.items():
        if source.name == "sc-lint-workflow.yml.j2" and not data["ci"]["enabled"]: continue
        result[target] = render(ROOT / source, data).encode()
    justfile = repo / "Justfile"; original = justfile.read_text() if justfile.exists() else ""
    if original.count(MARKER_START) != original.count(MARKER_END) or original.count(MARKER_START) > 1: raise RuntimeError(f"{justfile}: marker conflict")
    block = render(ROOT / Path("templates/Justfile.import.j2"), data).strip()
    result[Path("Justfile")] = (re.sub(re.escape(MARKER_START) + r".*?" + re.escape(MARKER_END), block, original, flags=re.S) if MARKER_START in original else (original.rstrip() + "\n\n" + block).lstrip("\n")).encode()
    return result


def digest(content: bytes) -> str: return hashlib.sha256(content).hexdigest()


def load_manifest(repo: Path) -> dict[str, str] | None:
    path = repo / MANIFEST
    if not path.exists(): return None
    data = json.loads(path.read_text())
    if not isinstance(data.get("managed"), dict): raise RuntimeError(f"{path}: invalid managed-file manifest")
    return data["managed"]


def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--input", type=Path, required=True); parser.add_argument("--dry-run", action="store_true"); parser.add_argument("repo", type=Path); args = parser.parse_args()
    try: data = values(args.input); targets = desired(data, args.repo); prior = load_manifest(args.repo)
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error: print(f"conflict: {error}", file=sys.stderr); return 2
    changes = []
    for relative, expected in targets.items():
        path = args.repo / relative; current = path.read_bytes() if path.exists() else b""
        if current != expected:
            if not args.dry_run and prior and relative.as_posix() in prior and digest(current) != prior[relative.as_posix()]: print(f"conflict: {path} was modified outside sc-lint", file=sys.stderr); return 2
            changes.append((path, current, expected))
    if args.dry_run:
        for path, old, new in changes: print("".join(difflib.unified_diff(old.decode(errors="replace").splitlines(True), new.decode(errors="replace").splitlines(True), fromfile=str(path), tofile=str(path))))
        return 1 if changes else 0
    manifest = {relative.as_posix(): digest(content) for relative, content in targets.items()}
    changes.append((args.repo / MANIFEST, (args.repo / MANIFEST).read_bytes() if (args.repo / MANIFEST).exists() else b"", json.dumps({"managed": manifest}, indent=2, sort_keys=True).encode() + b"\n"))
    for path, _, content in changes:
        path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(content)
        if path.relative_to(args.repo) == Path(".sc-lint/bootstrap"): path.chmod(0o755)
    return 0


if __name__ == "__main__": raise SystemExit(main())
