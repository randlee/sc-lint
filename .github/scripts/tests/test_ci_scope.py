"""Small regression suite for docs-only CI classification."""

from pathlib import Path
import subprocess
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_scope


@pytest.mark.parametrize(("path", "runtime"), [
    ("docs/guide.md", False),
    ("README.md", False),
    ("site/reports/release/index.html", False),
    ("docs/examples/check.py", True),
    ("Cargo.toml", True),
    ("crates/sc-lint/src/lib.rs", True),
    ("rust-toolchain.toml", True),
])
def test_path_classification(path, runtime):
    assert ci_scope.is_runtime(path) is runtime


def test_docs_only_and_mixed_changesets():
    assert ci_scope.requires_runtime(["docs/guide.md", "site/reports/index.html"]) is False
    assert ci_scope.requires_runtime(["docs/guide.md", "crates/sc-lint/src/lib.rs"]) is True


def test_changed_paths_disables_rename_detection(monkeypatch):
    # A rename must list both old and new paths, so the old path cannot hide a runtime file.
    calls = []

    def fake_run(argv, **kwargs):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 0, stdout="docs/old.md\ncrates/sc-lint/src/new.rs\n")

    monkeypatch.setattr(ci_scope.subprocess, "run", fake_run)
    assert ci_scope.changed_paths("base", "head") == ["docs/old.md", "crates/sc-lint/src/new.rs"]
    assert "--no-renames" in calls[0]
    assert ci_scope.scope_requires_runtime("pull_request", "base", "head") is True
