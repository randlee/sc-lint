#!/usr/bin/env python3
"""Normalize hardened Markdown and tracker-copy phase plans for review."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Any


MAX_PLAN_BYTES = 24_000
PHASE_TITLE_RE = re.compile(r"^# Phase ([A-Za-z0-9][A-Za-z0-9-]*)\b", re.MULTILINE)
SPRINT_TITLE_RE = re.compile(r"^# Sprint ([A-Za-z0-9][A-Za-z0-9-]*)\s+[—-]\s+(.+?)\s*$", re.MULTILINE)
HEADING_RE = re.compile(r"^## ([^\n]+?)\s*$", re.MULTILINE)
TABLE_HEADER = ["Sprint", "Authoritative plan", "Tracker", "Relation", "Closure"]
LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
COPY_REF_RE = re.compile(
    r"^plan-copy:(?P<path>[^@]+?)(?:@sha256:(?P<sha256>[0-9a-f]{64}))?$"
)
REQUIRED_HEADINGS = {
    "Scope boundary",
    "Authoritative deliverables",
    "Authoritative acceptance criteria",
    "Authoritative validation",
    "Non-closure",
}


class SourceError(ValueError):
    def __init__(self, code: str, message: str, *, data: Any = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.data = data


def _fail(code: str, message: str, *, data: Any = None) -> None:
    raise SourceError(code, message, data=data)


def canonical_json(value: Any) -> bytes:
    """Serialize exactly as jev_judge.encode_body does at the HTTP boundary."""

    return json.dumps(value, allow_nan=False).encode("utf-8")


def normalized_text(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n").rstrip("\n") + "\n"


def digest_text(text: str) -> str:
    return hashlib.sha256(normalized_text(text).encode("utf-8")).hexdigest()


def repository_file(root: Path, value: str | Path, *, code: str = "SOURCE.PATH") -> Path:
    supplied = Path(value)
    if supplied.is_absolute():
        _fail(code, "source path must be repository-relative")
    lexical = root / supplied
    try:
        resolved = lexical.resolve(strict=True)
        resolved.relative_to(root)
    except (OSError, ValueError):
        _fail(code, "source path is missing or escapes the repository")
    if lexical.is_symlink() or not stat.S_ISREG(resolved.stat().st_mode):
        _fail(code, "source path must resolve to a non-symlink regular file")
    return resolved


def _git_revision(root: Path) -> str | None:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    value = result.stdout.strip()
    return value if result.returncode == 0 and re.fullmatch(r"[0-9a-f]{40,64}", value) else None


def _row(line: str) -> list[str]:
    if not line.strip().startswith("|") or not line.strip().endswith("|"):
        return []
    return [cell.strip() for cell in line.strip()[1:-1].split("|")]


def phase_rows(text: str) -> list[dict[str, str]]:
    lines = text.splitlines()
    matches = [index for index, line in enumerate(lines) if _row(line) == TABLE_HEADER]
    if len(matches) != 1:
        _fail("SOURCE.PHASE_TABLE", "phase index must contain exactly one sprint table")
    start = matches[0]
    separator = _row(lines[start + 1]) if start + 1 < len(lines) else []
    if len(separator) != len(TABLE_HEADER) or not all(re.fullmatch(r":?-{3,}:?", cell) for cell in separator):
        _fail("SOURCE.PHASE_TABLE", "sprint table separator is missing or malformed")
    rows: list[dict[str, str]] = []
    for line in lines[start + 2:]:
        cells = _row(line)
        if not cells:
            break
        if len(cells) != len(TABLE_HEADER):
            _fail("SOURCE.PHASE_TABLE", "sprint table row has the wrong number of columns")
        link = LINK_RE.fullmatch(cells[1])
        if not link:
            _fail("SOURCE.PHASE_TABLE", "authoritative plan cell must be one Markdown link")
        rows.append({
            "id": cells[0],
            "label": link.group(1),
            "path": link.group(2),
            "tracker_id": cells[2].strip("`"),
            "relation": cells[3].strip("`"),
        })
    if not rows:
        _fail("SOURCE.PHASE_TABLE", "sprint table must contain at least one sprint")
    if len({row["id"] for row in rows}) != len(rows):
        _fail("SOURCE.DUPLICATE_ID", "phase sprint identifiers must be unique")
    if len({row["path"] for row in rows}) != len(rows):
        _fail("SOURCE.DUPLICATE_PATH", "phase sprint paths must be unique")
    return rows


def validate_sprint(text: str, expected_id: str | None = None) -> tuple[str, str]:
    normalized = normalized_text(text)
    title_matches = SPRINT_TITLE_RE.findall(normalized)
    if len(title_matches) != 1:
        _fail("SOURCE.SPRINT_TITLE", "sprint document must contain exactly one Sprint title")
    sprint_id, title = title_matches[0]
    if expected_id is not None and sprint_id != expected_id:
        _fail("SOURCE.SPRINT_ID", "sprint title identifier differs from phase table")
    headings = HEADING_RE.findall(normalized)
    for required in REQUIRED_HEADINGS:
        count = headings.count(required)
        if count != 1:
            _fail("SOURCE.SECTION", f"sprint must contain exactly one {required!r} section")
    return sprint_id, title.strip()


def shape(text: str, *, max_bytes: int = MAX_PLAN_BYTES) -> dict[str, Any]:
    normalized = normalized_text(text)
    headings = list(HEADING_RE.finditer(normalized))
    sections: dict[str, int] = {}
    for index, match in enumerate(headings):
        end = headings[index + 1].start() if index + 1 < len(headings) else len(normalized)
        sections[match.group(1)] = len(normalized[match.end():end].encode("utf-8"))
    size = len(normalized.encode("utf-8"))
    evidence = {
        "bytes": size,
        "chars": len(normalized),
        "lines": len(normalized.splitlines()),
        "section_bytes": sections,
        "max_bytes": max_bytes,
        "over_limit": size > max_bytes,
        "truncated": False,
    }
    if size > max_bytes:
        _fail("PLAN.TOO_LARGE", "sprint exceeds the reliable review limit", data=evidence)
    return evidence


def resolve_phase_index(root: Path, target: str | Path) -> Path:
    supplied = Path(target)
    if supplied.is_absolute():
        _fail("SOURCE.PATH", "phase target must be repository-relative")
    lexical = root / supplied
    try:
        resolved = lexical.resolve(strict=True)
        resolved.relative_to(root)
    except (OSError, ValueError):
        _fail("SOURCE.PATH", "phase target is missing or escapes the repository")
    if resolved.is_dir():
        candidates = [path for path in resolved.glob("plan-phase-*.md") if path.is_file() and not path.is_symlink()]
        if len(candidates) != 1:
            _fail("SOURCE.PHASE_INDEX", "phase directory must contain exactly one plan-phase-*.md index")
        return candidates[0].resolve()
    if lexical.is_symlink() or not stat.S_ISREG(resolved.stat().st_mode):
        _fail("SOURCE.PHASE_INDEX", "phase index must be a non-symlink regular file")
    return resolved


def discover_markdown(repository_root: str | Path, target: str | Path) -> dict[str, Any]:
    root = Path(repository_root).resolve(strict=True)
    index = resolve_phase_index(root, target)
    text = index.read_text(encoding="utf-8")
    phases = PHASE_TITLE_RE.findall(text)
    if len(phases) != 1:
        _fail("SOURCE.PHASE_ID", "phase index must contain exactly one Phase title")
    phase_id = "phase-" + phases[0].lower().removeprefix("phase-")
    revision = _git_revision(root)
    descriptors = []
    seen_resolved: set[Path] = set()
    for row in phase_rows(text):
        relative = (index.parent.relative_to(root) / row["path"]).as_posix()
        path = repository_file(root, relative)
        if path in seen_resolved:
            _fail("SOURCE.DUPLICATE_PATH", "multiple sprint links resolve to the same file")
        seen_resolved.add(path)
        item = normalized_text(path.read_text(encoding="utf-8"))
        sprint_id, title = validate_sprint(item, row["id"])
        item_shape = shape(item)
        repo_path = path.relative_to(root).as_posix()
        descriptors.append({
            "id": sprint_id,
            "title": title,
            "text": item,
            "path": repo_path,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "normalized_sha256": digest_text(item),
            "source_kind": "markdown",
            "source_ref": repo_path,
            "source_revision": revision,
            "tracker_id": row["tracker_id"],
            "dependency_relation": row["relation"],
            "shape": item_shape,
        })
    return {
        "schema": "plan-anchor/plan-source/1",
        "phase": phase_id,
        "source_kind": "markdown",
        "source": index.relative_to(root).as_posix(),
        "source_revision": revision,
        "sprints": descriptors,
    }


def _tracker_record(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail("SOURCE.TRACKER", "tracker children must be objects")
    description = value.get("description")
    if not isinstance(description, str):
        _fail("SOURCE.TRACKER", "tracker child description must be a sprint document")
    sprint_id, title = validate_sprint(description)
    item = normalized_text(description)
    external_ref = value.get("external_ref")
    ref_match = COPY_REF_RE.fullmatch(external_ref) if isinstance(external_ref, str) else None
    return {
        "id": sprint_id,
        "title": title,
        "text": item,
        "path": value.get("source_path") or (ref_match.group("path") if ref_match else None),
        "sha256": digest_text(item),
        "normalized_sha256": digest_text(item),
        "source_kind": value.get("tracker") or "tracker",
        "source_ref": str(value.get("id") or ""),
        "source_revision": value.get("revision") or value.get("updated_at"),
        "tracker_id": str(value.get("id") or ""),
        "dependency_relation": value.get("dependency_relation"),
        "_dependencies": value.get("dependencies") or [],
        "recorded_source_sha256": value.get("source_sha256") or (
            ref_match.group("sha256") if ref_match else None
        ),
        "updated_at": value.get("updated_at"),
        "external_ref": external_ref,
        "shape": shape(item),
    }


def discover_tracker(children: Any, *, phase: str, tracker: str, epic: str) -> dict[str, Any]:
    if tracker not in {"bd", "br"}:
        _fail("SOURCE.TRACKER", "tracker must be bd or br")
    if not isinstance(children, list):
        _fail("SOURCE.TRACKER", "tracker children output must be an array")
    records = []
    for child in children:
        copy = dict(child) if isinstance(child, dict) else child
        if isinstance(copy, dict):
            copy["tracker"] = tracker
        records.append(_tracker_record(copy))
    ids = [record["id"] for record in records]
    if len(set(ids)) != len(ids):
        _fail("SOURCE.DUPLICATE_ID", "tracker sprint identifiers must be unique")
    records.sort(key=lambda record: _sprint_order(record["id"]))
    tracker_to_sprint = {record["tracker_id"]: record["id"] for record in records}
    for record in records:
        if not record["dependency_relation"]:
            blockers = [
                dependency.get("id") or dependency.get("depends_on_id")
                for dependency in record["_dependencies"]
                if isinstance(dependency, dict)
                and (dependency.get("dependency_type") or dependency.get("type")) == "blocks"
            ]
            blockers = [value for value in blockers if isinstance(value, str) and value]
            if blockers:
                names = [tracker_to_sprint.get(value, value) for value in blockers]
                record["dependency_relation"] = "must_follow " + ", ".join(names)
            else:
                record["dependency_relation"] = "root"
        del record["_dependencies"]
    return {
        "schema": "plan-anchor/plan-source/1",
        "phase": phase,
        "source_kind": tracker,
        "source": epic,
        "source_revision": None,
        "sprints": records,
    }


def _tracker_json(
    repository_root: str | Path, tracker: str, arguments: list[str]
) -> Any:
    """Run one fixed tracker argv and decode JSON; never invoke a shell."""

    if tracker not in {"bd", "br"}:
        _fail("SOURCE.TRACKER", "tracker must be bd or br")
    binary = shutil.which(tracker)
    if binary is None:
        _fail("PREREQ.MISSING", f"{tracker} is not installed")
    root = Path(repository_root).resolve(strict=True)
    result = subprocess.run(
        [binary, *arguments, "--json"],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if result.returncode != 0:
        _fail("SOURCE.TRACKER_COMMAND", f"{tracker} command failed")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        _fail("SOURCE.TRACKER_OUTPUT", f"{tracker} returned invalid JSON")


def discover_tracker_epic(
    repository_root: str | Path, tracker: str, epic: str
) -> dict[str, Any]:
    """Discover an epic and its direct planning children through bd or br."""

    epic_rows = _tracker_json(repository_root, tracker, ["show", epic])
    if not isinstance(epic_rows, list) or len(epic_rows) != 1:
        _fail("SOURCE.TRACKER_EPIC", "tracker epic lookup must return exactly one issue")
    epic_row = epic_rows[0]
    if not isinstance(epic_row, dict) or epic_row.get("id") != epic:
        _fail("SOURCE.TRACKER_EPIC", "tracker epic lookup returned the wrong issue")
    if epic_row.get("issue_type") != "epic":
        _fail("SOURCE.TRACKER_EPIC", "selected tracker issue is not an epic")
    phase_matches = PHASE_TITLE_RE.findall(str(epic_row.get("description") or ""))
    if len(phase_matches) != 1:
        _fail("SOURCE.PHASE_ID", "tracker epic must contain exactly one Phase title")
    phase = "phase-" + phase_matches[0].lower().removeprefix("phase-")
    dependents = epic_row.get("dependents")
    if not isinstance(dependents, list):
        _fail("SOURCE.TRACKER_EPIC", "tracker epic lacks a child list")
    child_ids = [
        value.get("id")
        for value in dependents
        if isinstance(value, dict) and value.get("dependency_type") == "parent-child"
    ]
    if not child_ids or any(not isinstance(value, str) or not value for value in child_ids):
        _fail("SOURCE.TRACKER_EPIC", "tracker epic has no valid planning children")
    child_rows = _tracker_json(repository_root, tracker, ["show", *child_ids])
    plan = discover_tracker(child_rows, phase=phase, tracker=tracker, epic=epic)
    plan["source_revision"] = epic_row.get("updated_at")
    return plan


def _sprint_order(identifier: str) -> tuple[str, int, str]:
    match = re.fullmatch(r"([A-Za-z]+)(\d+)", identifier)
    return (match.group(1), int(match.group(2)), "") if match else (identifier, -1, identifier)


def compare_parity(markdown: dict[str, Any], tracker: dict[str, Any]) -> dict[str, Any]:
    left = {item["id"]: item for item in markdown["sprints"]}
    right = {item["id"]: item for item in tracker["sprints"]}
    if list(left) != list(right):
        _fail("SOURCE.PARITY_SET", "Markdown and tracker sprint order or identifiers differ")
    drift = []
    for sprint_id in left:
        if left[sprint_id]["normalized_sha256"] != right[sprint_id]["normalized_sha256"]:
            drift.append(sprint_id)
    if drift:
        _fail("SOURCE.PARITY_DRIFT", "tracker copy differs from authoritative Markdown", data={"sprints": drift})
    return {"equal": True, "sprints": list(left), "digests": {key: left[key]["normalized_sha256"] for key in left}}


def safe_copy_update(markdown_item: dict[str, Any], tracker_item: dict[str, Any] | None) -> dict[str, Any]:
    """Return a typed tracker update, refusing an independently changed copy."""

    if tracker_item is not None:
        recorded = tracker_item.get("recorded_source_sha256")
        current = tracker_item.get("normalized_sha256")
        authoritative = markdown_item["normalized_sha256"]
        if current != authoritative and (recorded is None or current != recorded):
            _fail("SOURCE.COPY_CONFLICT", "tracker copy changed independently; refusing overwrite")
    return {
        "tracker_id": tracker_item.get("tracker_id") if tracker_item else None,
        "sprint_id": markdown_item["id"],
        "description": markdown_item["text"],
        "source_path": markdown_item["path"],
        "source_sha256": markdown_item["normalized_sha256"],
    }


def copy_plan_to_tracker(
    repository_root: str | Path,
    markdown: dict[str, Any],
    tracker_plan: dict[str, Any],
    *,
    apply: bool = False,
) -> dict[str, Any]:
    """Verify or safely refresh exact tracker copies from authoritative Markdown."""

    tracker = tracker_plan.get("source_kind")
    if tracker not in {"bd", "br"}:
        _fail("SOURCE.TRACKER", "tracker plan must come from bd or br")
    left = {item["id"]: item for item in markdown.get("sprints", [])}
    right = {item["id"]: item for item in tracker_plan.get("sprints", [])}
    if list(left) != list(right):
        _fail("SOURCE.PARITY_SET", "Markdown and tracker sprint order or identifiers differ")
    actions = []
    for sprint_id, markdown_item in left.items():
        tracker_item = right[sprint_id]
        update = safe_copy_update(markdown_item, tracker_item)
        expected_ref = (
            f"plan-copy:{markdown_item['path']}"
            f"@sha256:{markdown_item['normalized_sha256']}"
        )
        content_equal = (
            markdown_item["normalized_sha256"] == tracker_item["normalized_sha256"]
        )
        metadata_equal = tracker_item.get("external_ref") == expected_ref
        action = {
            "sprint_id": sprint_id,
            "tracker_id": tracker_item["tracker_id"],
            "status": "verified" if content_equal and metadata_equal else "update_required",
            "content_equal": content_equal,
            "metadata_equal": metadata_equal,
            "source_path": update["source_path"],
            "source_sha256": update["source_sha256"],
        }
        if apply and action["status"] == "update_required":
            if not tracker_item.get("tracker_id") or not tracker_item.get("updated_at"):
                _fail("SOURCE.COPY_METADATA", "tracker copy lacks identity or revision")
            with tempfile.TemporaryDirectory(prefix="plan-anchor-copy-") as temporary:
                description = Path(temporary) / "description.md"
                description.write_text(update["description"], encoding="utf-8")
                _tracker_json(
                    repository_root,
                    tracker,
                    [
                        "update",
                        tracker_item["tracker_id"],
                        "--description-file",
                        str(description),
                        "--external-ref",
                        expected_ref,
                        "--if-unchanged",
                        tracker_item["updated_at"],
                    ],
                )
            action["status"] = "updated"
        actions.append(action)
    return {
        "equal": all(action["content_equal"] for action in actions),
        "applied": apply,
        "actions": actions,
    }


def canonical_jev_body(model: str, anchor: dict[str, Any], item: dict[str, Any], questions: dict[str, Any]) -> bytes:
    """Build the exact transport body; source metadata is intentionally absent."""

    body = {
        "model": model,
        "state": {
            "anchor": {
                "id": anchor["id"],
                "statement": anchor["statement"],
                "scope": anchor.get("scope") or "",
            },
            "item": {"id": item["id"], "title": item.get("title") or "", "text": normalized_text(item["text"])},
        },
        "questions": questions,
    }
    return canonical_json(body)


def run_binding(plan: dict[str, Any], item: dict[str, Any]) -> dict[str, Any]:
    return {
        "phase": plan["phase"],
        "phase_source": plan["source"],
        "sprint_id": item["id"],
        "sprint_source": item["source_ref"],
        "source_kind": item["source_kind"],
        "source_revision": item["source_revision"],
        "normalized_sha256": item["normalized_sha256"],
    }


def _emit(value: dict[str, Any]) -> None:
    print(json.dumps(value, sort_keys=True, separators=(",", ":")))


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    markdown = sub.add_parser("discover-markdown")
    markdown.add_argument("target")
    markdown.add_argument("--repository-root", default=".")
    tracker = sub.add_parser("discover-tracker")
    tracker.add_argument("--children-json")
    tracker.add_argument("--tracker", required=True, choices=("bd", "br"))
    tracker.add_argument("--epic", required=True)
    tracker.add_argument("--phase")
    tracker.add_argument("--repository-root", default=".")
    parity = sub.add_parser("parity")
    parity.add_argument("--markdown-json", required=True)
    parity.add_argument("--tracker-json", required=True)
    shape_parser = sub.add_parser("shape")
    shape_parser.add_argument("path")
    shape_parser.add_argument("--repository-root", default=".")
    copy_parser = sub.add_parser("copy")
    copy_parser.add_argument("target")
    copy_parser.add_argument("--repository-root", default=".")
    copy_parser.add_argument("--tracker", required=True, choices=("bd", "br"))
    copy_parser.add_argument("--epic", required=True)
    copy_parser.add_argument("--apply", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "discover-markdown":
            data = discover_markdown(args.repository_root, args.target)
        elif args.command == "discover-tracker":
            if args.children_json:
                if not args.phase:
                    _fail("SOURCE.PHASE_ID", "--phase is required with --children-json")
                data = discover_tracker(
                    json.loads(Path(args.children_json).read_text(encoding="utf-8")),
                    phase=args.phase,
                    tracker=args.tracker,
                    epic=args.epic,
                )
            else:
                data = discover_tracker_epic(
                    args.repository_root, args.tracker, args.epic
                )
        elif args.command == "parity":
            left = json.loads(Path(args.markdown_json).read_text(encoding="utf-8"))
            right = json.loads(Path(args.tracker_json).read_text(encoding="utf-8"))
            data = compare_parity(left, right)
        elif args.command == "shape":
            root = Path(args.repository_root).resolve(strict=True)
            source = repository_file(root, args.path)
            data = shape(source.read_text(encoding="utf-8"))
        else:
            markdown = discover_markdown(args.repository_root, args.target)
            tracker_plan = discover_tracker_epic(
                args.repository_root, args.tracker, args.epic
            )
            data = copy_plan_to_tracker(
                args.repository_root, markdown, tracker_plan, apply=args.apply
            )
        _emit({"success": True, "data": data, "error": None})
        return 0
    except SourceError as exc:
        _emit({"success": False, "data": exc.data, "error": {"code": exc.code, "message": exc.message, "recoverable": False, "suggested_action": None}})
        return 2
    except (OSError, UnicodeError, json.JSONDecodeError):
        _emit({"success": False, "data": None, "error": {"code": "SOURCE.INPUT", "message": "source input could not be read", "recoverable": False, "suggested_action": None}})
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
