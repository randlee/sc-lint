"""Classify whether a CI event's changed paths require the runtime CI matrix.

Documentation-only pull requests skip the runtime jobs and still report the same
check names. A path is runtime unless it is explicitly documentation or development
metadata, so an unclassified new path, a README, a CHANGELOG or any file a test or lint
reads runs the full matrix. Pushes to develop and main always run it.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from collections.abc import Iterable

GIT_TIMEOUT_SECONDS = 60

# Documentation and development metadata that no test or lint reads. site/reports/ is
# the only part of site/ that is exempt: the rest of the Pages site is runtime.
NON_RUNTIME_PATHS = (
    "site/reports/",
    "docs/",
    ".beads/",
    ".atm-bd/",
    ".triage/",
)
# Paths under a non-runtime prefix that tests or lints read, so they stay runtime.
RUNTIME_EXCEPTIONS = ("docs/sc-lint/",)


def is_runtime(path: str) -> bool:
    """Return whether one changed path can change the Rust build, the artifacts or a check."""
    # Python sources are code wherever they sit, so they are never documentation.
    if path.endswith(".py") or path.startswith(RUNTIME_EXCEPTIONS):
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
        ["git", "diff", "--name-only", "--no-renames", "-z", base, head],
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
    if not base or set(base) == {"0"} or not head or not commit_exists(base):
        return True
    return requires_runtime(changed_paths(base, head))


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
