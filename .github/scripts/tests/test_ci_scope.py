"""Small regression suite for docs-only CI classification."""

from pathlib import Path
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_scope


@pytest.mark.parametrize(("path", "runtime"), [
    ("docs/guide.md", False),
    ("site/reports/release/index.html", False),
    ("docs/examples/check.py", True),
    ("docs/sc-lint/cli-contract.md", True),
    ("Cargo.toml", True),
    ("crates/sc-lint/src/lib.rs", True),
    ("brand-new/path.txt", True),
])
def test_path_classification(path, runtime):
    assert ci_scope.is_runtime(path) is runtime


def test_docs_only_and_mixed_changesets():
    assert ci_scope.requires_runtime(["docs/guide.md", "site/reports/index.html"]) is False
    assert ci_scope.requires_runtime(["docs/guide.md", "crates/sc-lint/src/lib.rs"]) is True
