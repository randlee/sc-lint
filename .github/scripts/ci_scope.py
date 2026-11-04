"""Classify whether a CI event's changed paths require the runtime CI matrix.

Documentation-only pull requests skip the runtime jobs and still report the same
check names. A path is runtime unless it is under an explicitly exempt prefix; Python
sources remain runtime even under documentation prefixes. Unclassified paths run the
full matrix. Pushes to develop and main always run it.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections.abc import Iterable

GIT_TIMEOUT_SECONDS = 60

# This is the canonical non-runtime prefix list.
NON_RUNTIME_PATHS = (
    "site/reports/",  # Rand, 2026-11-04 (lint-085.79): explicitly approved.
    "docs/",  # lint-085.79: documentation-only direction; read files are reclassified.
    ".beads/",  # lint-085.79: generated Beads issue data is development metadata.
    ".atm-bd/",  # lint-085.79: local ATM state is development metadata.
    ".triage/",  # lint-085.79: finding records are development metadata.
)
REVISION_RE = re.compile(r"^(?:[0-9a-fA-F]{40}|[0-9a-fA-F]{64})$")


def is_runtime(path: str) -> bool:
    """Return whether one changed path can change the Rust build, the artifacts or a check."""
    # Python sources are code wherever they sit, so they are never documentation.
    if path.endswith(".py") or path.startswith("docs/sc-lint/") or path == "docs/requirements.md":
        return True
    return not path.startswith(NON_RUNTIME_PATHS)


def requires_runtime(changed_paths: Iterable[str]) -> bool:
    """Return whether any changed path requires the runtime matrix."""
    return any(is_runtime(path) for path in changed_paths if path)


def commit_exists(revision: str) -> bool:
    result = subprocess.run(
        ["git", "cat-file", "-e", revision + "^{commit}"],
        capture_output=True, check=False, timeout=GIT_TIMEOUT_SECONDS,
    )
    return result.returncode == 0


def changed_paths(base: str, head: str) -> list[str]:
    """Every path that differs, with renames split so both the old and new names are classified."""
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "-z", base, head, "--"],
        capture_output=True, check=True, timeout=GIT_TIMEOUT_SECONDS,
    )
    return result.stdout.decode("utf-8", errors="surrogateescape").split("\0")


def scope_requires_runtime(event: str, base: str, head: str) -> bool:
    """Return whether one CI event requires the runtime matrix.

    Only a pull request is compared with its base. Any other event, a missing or all-zero
    base, a base this clone does not contain, or a missing head runs the matrix, because
    its change set cannot be read.
    """
    if event != "pull_request":
        return True
    if not REVISION_RE.fullmatch(base) or set(base) == {"0"}:
        return True
    if not REVISION_RE.fullmatch(head) or set(head) == {"0"}:
        return True
    try:
        if not commit_exists(base) or not commit_exists(head):
            return True
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
