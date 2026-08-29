#!/usr/bin/env python3
"""Bounded, no-write context collection and configuration planning for sc-lint.

F.2 deliberately observes only conventional paths.  It never parses a
Justfile or workflow, scans source, invokes Cargo, starts a subprocess, or
writes to the consumer repository.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys
from typing import Any

from sc_lint_configure_schema import SchemaProblem
from sc_lint_configure_schema import validate


CONFIGURE_COMMAND = "configure.plan"
CONFIGURE_SCHEMA_ERROR = "CLI.CONFIGURE_UNSUPPORTED_SCHEMA"
CONFIGURE_DOCS = "sc-lint docs configuration"
DEVELOPER_CONTRACT = ["just setup", "just lint", "just test", "just upgrade"]
FAMILY_NAMES = ("baseline", "boundary", "portability", "runtime", "attributes")
PACKAGE_MARKER = re.compile(r"^\s*\[package\]\s*$", flags=re.MULTILINE)
WORKSPACE_MARKER = re.compile(r"^\s*\[workspace\]\s*$", flags=re.MULTILINE)


class ConfigureFailure(Exception):
    """A stable, user-recoverable failure from the bounded planning surface."""

    def __init__(
        self,
        message: str,
        *,
        cause: str,
        pointer: str | None,
        recovery: str,
        recovery_description: str,
    ) -> None:
        super().__init__(message)
        self.message = message
        self.cause = cause
        self.pointer = pointer
        self.recovery = recovery
        self.recovery_description = recovery_description


def collect_context(root: Path) -> dict[str, Any]:
    """Collect only F.1's conventional presence and Cargo-marker facts."""
    if not root.exists():
        raise ConfigureFailure(
            "The requested repository root does not exist.",
            cause="the --root path was not found",
            pointer="/root",
            recovery="select_existing_root",
            recovery_description="Pass an existing repository directory with --root.",
        )
    if not root.is_dir():
        raise ConfigureFailure(
            "The requested repository root is not a directory.",
            cause="the --root path does not identify a directory",
            pointer="/root",
            recovery="select_repository_directory",
            recovery_description="Pass a readable repository directory with --root.",
        )
    if not os.access(root, os.R_OK | os.X_OK):
        raise ConfigureFailure(
            "The requested repository root is not readable.",
            cause="the --root directory lacks read or search permission",
            pointer="/root",
            recovery="repair_root_permissions",
            recovery_description="Grant read and search permission for the repository root, then rerun configure.",
        )

    cargo_path = root / "Cargo.toml"
    cargo_toml: dict[str, Any] = {"present": False}
    if cargo_path.is_file():
        try:
            cargo_contents = cargo_path.read_text(encoding="utf-8")
        except OSError as error:
            raise ConfigureFailure(
                "The root Cargo.toml could not be read.",
                cause=str(error),
                pointer="/context/cargo_toml",
                recovery="repair_cargo_toml_permissions",
                recovery_description="Make the root Cargo.toml readable, then rerun configure.",
            ) from error
        cargo_kind = "workspace" if WORKSPACE_MARKER.search(cargo_contents) else None
        if cargo_kind is None and PACKAGE_MARKER.search(cargo_contents):
            cargo_kind = "package"
        if cargo_kind is not None:
            cargo_toml = {"present": True, "kind": cargo_kind}

    justfile_present = (root / "Justfile").is_file()
    workflows_present = (root / ".github" / "workflows").is_dir()
    context = {
        "schema_version": "v1",
        "context": {
            "cargo_toml": cargo_toml,
            "sc_lint_toml": {"present": (root / "sc-lint.toml").is_file()},
            "justfile": _uninspected_presence(justfile_present),
            "github_workflows": _uninspected_presence(workflows_present),
            "sc_lint_directory": {"present": (root / ".sc-lint").is_dir()},
        },
        "explanation": {
            "developer_contract": DEVELOPER_CONTRACT,
            "uninspected_existing_integration": [
                path
                for path, present in (
                    ("Justfile", justfile_present),
                    (".github/workflows/", workflows_present),
                )
                if present
            ],
        },
    }
    _raise_schema_problems("context", validate("context", context))
    return context


def build_plan(context: dict[str, Any], request: dict[str, Any], root: Path | None = None) -> dict[str, Any]:
    """Return a deterministic advisory plan with no repository mutation."""
    _raise_schema_problems("request", validate("request", request))
    observations = context["context"]
    choices = request["request"]
    operations: list[dict[str, Any]] = []

    selected_families = ",".join(
        family
        for family in FAMILY_NAMES
        if choices["lint_families"][family]["state"] != "disabled"
    )
    config_reason = f"recommended_profiles:{selected_families or 'none'}"
    if observations["sc_lint_toml"]["present"]:
        operations.append(
            _confirmation(
                "sc-lint-config",
                "sc-lint.toml",
                "existing_sc_lint_config_not_rewritten",
            )
        )
    else:
        operations.append(
            {
                "operation_id": "sc-lint-config",
                "path": "sc-lint.toml",
                "kind": "propose_create",
                "reason": config_reason,
            }
        )
        operations.extend(
            [
                {
                    "operation_id": "bootstrap-posix",
                    "path": ".sc-lint/bootstrap",
                    "kind": "propose_create",
                    "reason": "managed_consumer_bootstrap",
                },
                {
                    "operation_id": "bootstrap-windows",
                    "path": ".sc-lint/bootstrap.ps1",
                    "kind": "propose_create",
                    "reason": "managed_consumer_bootstrap",
                },
            ]
        )

    _append_just_operations(operations, observations, choices["just"]["mode"])
    _append_workflow_operations(operations, observations, choices["ci"]["mode"])

    preconditions = _preconditions(root, operations) if root is not None else []
    plan = {
        "schema_version": "v1",
        "plan_id": _plan_id(context, request, preconditions),
        "operations": operations,
        "preconditions": preconditions,
        "conflicts": [],
        "manual_steps": [],
    }
    _raise_schema_problems("plan", validate("plan", plan))
    return plan


def plan_result(root: Path, request: dict[str, Any]) -> dict[str, Any]:
    """Collect context and render the F.1 result envelope for a plan request."""
    context = collect_context(root)
    plan = build_plan(context, request, root)
    result = {"ok": True, "command": CONFIGURE_COMMAND, "data": plan, "diagnostics": []}
    _raise_schema_problems("result", validate("result", result))
    return result


def load_request(request_path: str) -> dict[str, Any]:
    """Parse the request exactly once from a file or standard input."""
    try:
        raw = sys.stdin.read() if request_path == "-" else Path(request_path).read_text(encoding="utf-8")
    except OSError as error:
        raise ConfigureFailure(
            "The configuration request could not be read.",
            cause=str(error),
            pointer="/request",
            recovery="supply_readable_request",
            recovery_description="Pass a readable JSON request file or use - for standard input.",
        ) from error
    try:
        loaded = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ConfigureFailure(
            "The configuration request is not valid JSON.",
            cause=error.msg,
            pointer="/request",
            recovery="repair_request_json",
            recovery_description="Correct the JSON request, then rerun configure.",
        ) from error
    if not isinstance(loaded, dict):
        raise ConfigureFailure(
            "The configuration request must be a JSON object.",
            cause="the request root is not an object",
            pointer="/request",
            recovery="supply_request_object",
            recovery_description="Supply a JSON object that conforms to the v1 request schema.",
        )
    return loaded


def _append_just_operations(
    operations: list[dict[str, Any]], observations: dict[str, Any], mode: str
) -> None:
    if mode != "generate_managed_import":
        return
    if observations["justfile"]["present"]:
        operations.append(
            {
                "operation_id": "managed-justfile",
                "path": ".sc-lint/justfile",
                "kind": "propose_create",
                "reason": "managed_consumer_recipes",
            }
        )
        operations.append(
            _confirmation("root-justfile", "Justfile", "existing_integration_uninspected")
        )
        return
    operations.extend(
        [
            {
                "operation_id": "managed-justfile",
                "path": ".sc-lint/justfile",
                "kind": "propose_create",
                "reason": "managed_consumer_recipes",
            },
            _confirmation("root-justfile", "Justfile", "managed_import_requires_confirmation"),
        ]
    )


def _append_workflow_operations(
    operations: list[dict[str, Any]], observations: dict[str, Any], mode: str
) -> None:
    if observations["github_workflows"]["present"]:
        operations.append(
            _confirmation(
                "github-workflow", ".github/workflows/sc-lint.yml", "existing_integration_uninspected"
            )
        )
        return
    if mode in {"disabled", "keep_existing"}:
        return
    operations.append(
        _confirmation(
            "github-workflow", ".github/workflows/sc-lint.yml", "workflow_generation_requires_confirmation"
        )
    )


def _confirmation(operation_id: str, path: str, reason: str) -> dict[str, Any]:
    return {
        "operation_id": operation_id,
        "path": path,
        "kind": "needs_confirmation",
        "reason": reason,
        "choices": ["keep_existing", "generate_managed_import", "review_patch"],
    }


def _uninspected_presence(present: bool) -> dict[str, Any]:
    return {"present": True, "inspected": False} if present else {"present": False}


def _preconditions(root: Path, operations: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Capture operation-target content digests for a reviewed apply plan."""
    preconditions = []
    for path in dict.fromkeys(operation["path"] for operation in operations):
        candidate = root / path
        digest = None
        if candidate.is_file():
            digest = f"sha256:{hashlib.sha256(candidate.read_bytes()).hexdigest()}"
        preconditions.append({"path": path, "source_digest": digest})
    return preconditions


def _plan_id(context: dict[str, Any], request: dict[str, Any], preconditions: list[dict[str, Any]]) -> str:
    canonical = json.dumps(
        {"context": context, "request": request, "preconditions": preconditions}, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return f"sha256:{hashlib.sha256(canonical).hexdigest()[:16]}"


def _raise_schema_problems(contract: str, problems: list[SchemaProblem]) -> None:
    if not problems:
        return
    problem = problems[0]
    raise ConfigureFailure(
        f"The {contract} payload does not conform to the v1 configure schema.",
        cause=problem.message,
        pointer=problem.pointer,
        recovery="repair_request_schema",
        recovery_description="Correct the field identified by pointer and rerun configure.",
    )


def render_failure(error: ConfigureFailure) -> dict[str, Any]:
    """Render stable, F.1-compatible machine recovery data for every rejection."""
    return {
        "ok": False,
        "command": CONFIGURE_COMMAND,
        "error": {
            "code": CONFIGURE_SCHEMA_ERROR,
            "message": error.message,
            "cause": error.cause,
            "pointer": error.pointer,
            "recovery": error.recovery,
            "recovery_description": error.recovery_description,
            "docs_ref": CONFIGURE_DOCS,
        },
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Create a no-write sc-lint configure plan.")
    parser.add_argument("--request", required=True, help="JSON request file path, or - for standard input.")
    parser.add_argument("--root", required=True, help="Consumer repository root to observe.")
    parser.add_argument("--dry-run", action="store_true", help="Accepted explicitly; F.2 never writes files.")
    parser.add_argument("--json", action="store_true", help="Emit the stable JSON result envelope.")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    try:
        args = parse_args(argv)
        request = load_request(args.request)
        result = plan_result(Path(args.root), request)
        print(json.dumps(result, sort_keys=True, separators=(",", ":")))
        return 0
    except ConfigureFailure as error:
        print(json.dumps(render_failure(error), sort_keys=True, separators=(",", ":")))
        return 3


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
