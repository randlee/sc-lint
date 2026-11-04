"""Small regression suite for docs-only CI classification and workflow skips."""

from pathlib import Path
import sys

import pytest
import yaml

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_scope


@pytest.mark.parametrize(("path", "runtime"), [
    ("docs/guide.md", False),
    ("site/reports/release/index.html", False),
    ("docs/examples/check.py", True),
    ("docs/sc-lint/cli-contract.md", True),
    ("docs/requirements.md", True),
    ("Cargo.toml", True),
    ("crates/sc-lint/src/lib.rs", True),
    ("brand-new/path.txt", True),
])
def test_path_classification(path, runtime):
    assert ci_scope.is_runtime(path) is runtime


def test_docs_only_and_mixed_changesets():
    assert ci_scope.requires_runtime(["docs/guide.md", "site/reports/index.html"]) is False
    assert ci_scope.requires_runtime(["docs/guide.md", "crates/sc-lint/src/lib.rs"]) is True


def test_workflow_gates_docs_skips_and_avoids_non_ubuntu_runners():
    workflow_path = REPO / ".github/workflows/ci.yml"
    workflow = yaml.safe_load(workflow_path.read_text(encoding="utf-8"))
    jobs = workflow["jobs"]
    runner = "${{ needs.ci-scope.outputs.runtime != 'false' && matrix.os || 'ubuntu-latest' }}"
    for job_name in ("just-lint", "test", "adoption", "release-smoke"):
        job = jobs[job_name]
        assert job["runs-on"] == runner
        skip = next(step for step in job["steps"] if "Documentation-only" in step.get("name", ""))
        assert skip["if"] == "${{ needs.ci-scope.outputs.runtime == 'false' }}"
