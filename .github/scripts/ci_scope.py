"""Classify whether a CI event's changed paths require the runtime CI matrix.

Only paths that can change the Rust build, the shipped artifacts or a CI check
are runtime. Pull requests that touch nothing else keep the same check names
but skip the runtime jobs. Any other event runs the full matrix, and so does
any unreadable revision range.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from collections.abc import Iterable

RUNTIME_PATHS = (
    "crates/",
    "bindings/",
    "docs-bundle/",
    "release/",
    ".github/",
    ".cargo/",
    ".sc-lint/",
)
RUNTIME_FILES = {"Cargo.toml", "Cargo.lock", "Justfile", "sc-lint.toml", "pyproject.toml"}


def is_runtime(path: str) -> bool:
    """Return whether one changed path can change the Rust build, the artifacts or a check."""
    return (
        path in RUNTIME_FILES
        or path.startswith("rust-toolchain")
        or path.endswith(".py")
        or path.startswith(RUNTIME_PATHS)
    )


def requires_runtime(changed_paths: Iterable[str]) -> bool:
    """Return whether any changed path requires the runtime matrix."""
    return any(is_runtime(path) for path in changed_paths if path)


def changed_paths(base: str, head: str) -> list[str]:
    """Return paths changed between the pull request base and head."""
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", base, head, "--"],
        capture_output=True, check=True, text=True,
    )
    return result.stdout.splitlines()


def scope_requires_runtime(event: str, base: str, head: str) -> bool:
    """Return whether one CI event requires the runtime matrix.

    Only a pull request is compared with its base. Any other event runs the matrix.
    """
    if event != "pull_request":
        return True
    try:
        return requires_runtime(changed_paths(base, head))
    except (OSError, subprocess.SubprocessError):
        # Any unreadable revision range must run CI, never skip it.
        return True


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="ci_scope.py", description="Print runtime=true or runtime=false.")
    parser.add_argument("--event", required=True, help="GitHub event name, for example pull_request or push")
    parser.add_argument("--base", default="", help="pull request base commit; empty when unknown")
    parser.add_argument("--head", default="", help="commit under test; empty when unknown")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    runtime = scope_requires_runtime(args.event, args.base, args.head)
    print(f"runtime={str(runtime).lower()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
