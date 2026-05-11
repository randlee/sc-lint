#!/usr/bin/env python3
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
import argparse
import sys
import time
import tomllib

from lint_common import build_report
from lint_common import discover_repo_root
from lint_common import print_report
from lint_common import workspace_crate_section_lines


TOP_LEVEL_KEYS = {
    "boundary_id",
    "owner_package",
    "owner_crate_path",
    "name",
    "public",
    "implementation",
    "composition",
    "dependencies",
    "references",
    "testing",
    "enforcement",
    "status",
}
PUBLIC_KEYS = {"facade"}
IMPLEMENTATION_KEYS = {"type", "module", "visibility", "constructor"}
COMPOSITION_KEYS = {"roots"}
DEPENDENCIES_KEYS = {"allowed_dependents", "allowed_dependencies", "forbidden_edges"}
REFERENCES_KEYS = {"scope", "forbidden"}
TESTING_KEYS = {"allowed_test_double_paths", "forbidden_test_bypasses"}
ENFORCEMENT_KEYS = {"lint_rules", "review_gates"}
STATUS_KEYS = {"state"}


def boundary_file_paths(repo_root: Path) -> list[Path]:
    boundaries_root = repo_root / "boundaries"
    if not boundaries_root.exists():
        return []
    return sorted(
        path for path in boundaries_root.rglob("*.toml") if path.name != "planning.toml"
    )


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def ensure_exact_keys(
    table: dict, expected: set[str], label: str, path: Path, errors: list[str]
) -> None:
    extra = sorted(set(table) - expected)
    if extra:
        errors.append(f"{path}: unexpected {label} keys: {', '.join(extra)}")


def validate_boundary_file(
    path: Path, boundaries_root: Path, seen_ids: dict[str, Path], errors: list[str]
) -> None:
    try:
        data = load_toml(path)
    except tomllib.TOMLDecodeError as exc:
        errors.append(f"{path}: failed to parse TOML: {exc}")
        return

    ensure_exact_keys(data, TOP_LEVEL_KEYS, "top-level", path, errors)
    if not TOP_LEVEL_KEYS.issubset(data):
        errors.append(f"{path}: missing required top-level keys")
        return

    boundary_id = data["boundary_id"]
    owner_package = data["owner_package"]
    owner_crate_path = data["owner_crate_path"]

    previous = seen_ids.get(boundary_id)
    if previous is not None:
        errors.append(f"duplicate boundary_id `{boundary_id}` in `{previous}` and `{path}`")
    else:
        seen_ids[boundary_id] = path

    owner_dir = path.parent.name
    if owner_dir != owner_package:
        errors.append(
            f"{path}: owner directory `{owner_dir}` does not match owner_package `{owner_package}`"
        )

    expected_crate_path = owner_package.replace("-", "_")
    if owner_crate_path != expected_crate_path:
        errors.append(
            f"{path}: owner_crate_path `{owner_crate_path}` does not match `{expected_crate_path}`"
        )

    relative = path.relative_to(boundaries_root)
    if len(relative.parts) != 2:
        errors.append(f"{path}: expected boundaries/<owner-package>/<boundary>.toml layout")

    public = data["public"]
    implementation = data["implementation"]
    composition = data["composition"]
    dependencies = data["dependencies"]
    references = data["references"]
    testing = data["testing"]
    enforcement = data["enforcement"]
    status = data["status"]

    ensure_exact_keys(public, PUBLIC_KEYS, "public", path, errors)
    ensure_exact_keys(implementation, IMPLEMENTATION_KEYS, "implementation", path, errors)
    ensure_exact_keys(composition, COMPOSITION_KEYS, "composition", path, errors)
    ensure_exact_keys(dependencies, DEPENDENCIES_KEYS, "dependencies", path, errors)
    ensure_exact_keys(references, REFERENCES_KEYS, "references", path, errors)
    ensure_exact_keys(testing, TESTING_KEYS, "testing", path, errors)
    ensure_exact_keys(enforcement, ENFORCEMENT_KEYS, "enforcement", path, errors)
    ensure_exact_keys(status, STATUS_KEYS, "status", path, errors)

    visibility = implementation.get("visibility")
    if visibility not in {"public", "trait_only"}:
        errors.append(f"{path}: unsupported implementation.visibility `{visibility}`")
        return

    if not str(public.get("facade", "")).strip():
        errors.append(f"{path}: public.facade must be non-empty")

    if visibility == "public":
        if not str(implementation.get("type", "")).strip():
            errors.append(f"{path}: implementation.type must be present for public visibility")
        if not str(implementation.get("module", "")).strip():
            errors.append(
                f"{path}: implementation.module must be present for public visibility"
            )
        if implementation.get("constructor") != "none":
            errors.append(
                f"{path}: implementation.constructor must be `none` for public visibility"
            )
    else:
        if "type" in implementation:
            errors.append(f"{path}: trait_only visibility must omit implementation.type")
        if "module" in implementation:
            errors.append(f"{path}: trait_only visibility must omit implementation.module")


def validate_planning(repo_root: Path, errors: list[str]) -> None:
    planning_path = repo_root / "boundaries" / "planning.toml"
    try:
        planning = load_toml(planning_path)
    except FileNotFoundError:
        errors.append(f"{planning_path}: missing planning.toml")
        return
    except tomllib.TOMLDecodeError as exc:
        errors.append(f"{planning_path}: failed to parse TOML: {exc}")
        return

    planning_header = planning.get("planning")
    if not isinstance(planning_header, dict) or not str(
        planning_header.get("current_sprint", "")
    ).strip():
        errors.append(f"{planning_path}: [planning].current_sprint must be non-empty")

    planned_items = planning.get("planned_items", {})
    if not isinstance(planned_items, dict):
        errors.append(f"{planning_path}: [planned_items] must be a table")
        return

    for key, item in planned_items.items():
        if not key.startswith("BOUNDARY-") or "." not in key:
            errors.append(
                f"{planning_path}: planning item key `{key}` must use <boundary_id>.<section>.<field>[.<subfield>] shape"
            )
            continue
        if not isinstance(item, dict):
            errors.append(f"{planning_path}: planning item `{key}` must be a table")
            continue
        if not str(item.get("scheduled_sprint", "")).strip() or not str(
            item.get("tracking_id", "")
        ).strip():
            errors.append(
                f"{planning_path}: planning item `{key}` must define scheduled_sprint and tracking_id"
            )


def validate_inventory(repo_root: Path) -> list[str]:
    errors: list[str] = []
    boundaries_root = repo_root / "boundaries"
    seen_ids: dict[str, Path] = {}
    for path in boundary_file_paths(repo_root):
        validate_boundary_file(path, boundaries_root, seen_ids, errors)
    validate_planning(repo_root, errors)
    return errors


def run(repo_root: Path) -> int:
    started_at = datetime.now(timezone.utc)
    start_time = time.perf_counter()
    errors = validate_inventory(repo_root)
    duration_seconds = time.perf_counter() - start_time

    transcript = [
        *workspace_crate_section_lines(repo_root, title="inventory:"),
        f"boundary_files: {len(boundary_file_paths(repo_root))}",
        f"errors: {len(errors)}",
    ]

    report = build_report(
        lint_name="boundaries",
        repo_root=repo_root,
        passed=not errors,
        summary=(
            "boundary inventory validated"
            if not errors
            else f"boundary inventory validation failed ({len(errors)} errors)"
        ),
        findings=errors,
        transcript_lines=transcript,
        started_at=started_at,
        duration_seconds=duration_seconds,
    )
    print_report(report, repo_root=repo_root, preview_limit=3, direct_threshold=3)
    return 0 if not errors else 1


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Validate canonical boundary inventory TOML as the Python parity oracle."
    )
    parser.add_argument("--root", help="Repo root to inspect.")
    args = parser.parse_args(argv[1:])
    repo_root = discover_repo_root(args.root)
    return run(repo_root)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
