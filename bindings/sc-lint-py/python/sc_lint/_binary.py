"""Locate the installed sc-lint product binary.

Resolution order mirrors `.sc-lint/bootstrap`: `SC_LINT_BIN`, the managed
install directory (`SC_LINT_INSTALL_DIR`, else `$XDG_DATA_HOME/sc-lint/bin`,
else `~/.local/share/sc-lint/bin`), then `PATH`.
"""
from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

BINARY_NAME = "sc-lint.exe" if sys.platform == "win32" else "sc-lint"


def managed_install_dir() -> Path:
    override = os.environ.get("SC_LINT_INSTALL_DIR")
    if override:
        return Path(override)
    data_home = os.environ.get("XDG_DATA_HOME") or str(Path.home() / ".local" / "share")
    return Path(data_home) / "sc-lint" / "bin"


def binary_path() -> str:
    """Return the sc-lint binary path, raising FileNotFoundError when absent."""
    explicit = os.environ.get("SC_LINT_BIN")
    if explicit:
        return explicit
    managed = managed_install_dir() / BINARY_NAME
    if managed.is_file():
        return str(managed)
    found = shutil.which("sc-lint")
    if found:
        return found
    raise FileNotFoundError(
        "sc-lint binary not found: set SC_LINT_BIN, run `.sc-lint/bootstrap setup`, or add sc-lint to PATH"
    )
