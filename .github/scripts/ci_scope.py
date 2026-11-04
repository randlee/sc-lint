"""Classify whether changed repository paths require the runtime CI matrix.

Documentation-only changes skip the runtime jobs. A path is runtime unless it is
explicitly documentation or development metadata, so an unclassified new path runs
the full matrix.
"""

from __future__ import annotations

import subprocess
import sys
from collections.abc import Iterable


# Paths that change the Rust build, the released artifacts, or the CI runtime. The
# docs-bundle is packed into release archives and the release smoke depends on it,
# so it stays runtime even though it is documentation.
RUNTIME_PATHS = (
    "crates/",
    "bindings/",
    "packages/",
    "action/",
    "boundaries/",
    "docs-bundle/",
    "release/",
    "scripts/",
    "tests/",
    "tools/",
    ".github/workflows/",
    ".github/scripts/",
    ".sc-lint/",
    ".just/",
)
RUNTIME_FILES = frozenset({
    "action.yml",
    "Cargo.toml",
    "Cargo.lock",
    "Justfile",
    "deny.toml",
    "install.py",
    "rust-toolchain.toml",
    "sc-lint.toml",
})

# Documentation, the GitHub Pages site and development metadata. site/reports/ is
# listed explicitly so that published reports never trigger the runtime matrix.
NON_RUNTIME_PATHS = (
    "site/reports/",
    "site/",
    "docs/",
    ".claude/",
    ".claude-plugin/",
    ".agents/",
    ".codex/",
    ".cursor/",
    ".beads/",
    ".atm-bd/",
    ".triage/",
)


def is_runtime(path: str) -> bool:
    """Return whether one changed path can change the Rust build or the CI runtime."""
    if path in RUNTIME_FILES or path.startswith("rust-toolchain") or path.startswith(RUNTIME_PATHS):
        return True
    # Python sources are runtime everywhere, including the skill scripts under .claude/.
    if path.endswith(".py"):
        return True
    if path.startswith(NON_RUNTIME_PATHS) or path.endswith(".md"):
        return False
    return True


def requires_runtime(changed_paths: Iterable[str]) -> bool:
    """Return whether any changed path requires the runtime matrix."""
    return any(is_runtime(path) for path in changed_paths if path)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: ci_scope.py <range-start> <head>", file=sys.stderr)
        return 2
    changed = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "-z", argv[1], argv[2]],
        check=True,
        capture_output=True,
    ).stdout
    paths = changed.decode("utf-8", errors="surrogateescape").split("\0")
    print(f"runtime={str(requires_runtime(paths)).lower()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
