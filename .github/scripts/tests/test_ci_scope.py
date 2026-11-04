"""Deterministic classification tests for the CI scope gate."""
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_scope


@pytest.mark.parametrize("path", [
    "site/reports/release-native-verify/index.html",
    "site/reports/release-native-verify/20261104T180000Z-prerelease-v1.2.3-run4242/index.json",
    "site/index.html",
    "site/announcements/sc-lint-v0.8.2.md",
    "docs/requirements.md",
    "README.md",
    "CHANGELOG.md",
    "AGENTS.md",
    ".claude/skills/publishing/ref/release-state-strategy.md",
    ".beads/issues.jsonl",
    ".triage/phase-ao2/findings/AO210.ttl",
])
def test_documentation_and_metadata_paths_do_not_require_runtime(path):
    assert ci_scope.is_runtime(path) is False


@pytest.mark.parametrize("path", [
    "crates/sc-lint/src/lib.rs",
    "crates/sc-lint-boundary/README.md",
    "bindings/sc-lint-py/python/sc_lint/__init__.py",
    "packages/sc-lint-adoption/README.md",
    "action/index.js",
    "action.yml",
    ".github/workflows/ci.yml",
    ".github/scripts/release_native_gate.py",
    ".sc-lint/bootstrap.py",
    ".just/tests/test_prerelease_tag.py",
    "scripts/release_smoke.py",
    "release/publish-artifacts.toml",
    "docs-bundle/ci.md",
    "docs-bundle/cli-reference/sc-lint-boundary/index.html",
    "tests/docs/test_action_docs.py",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "Justfile",
    "sc-lint.toml",
    "install.py",
])
def test_build_release_and_ci_paths_require_runtime(path):
    assert ci_scope.is_runtime(path) is True


def test_python_under_claude_is_still_runtime():
    assert ci_scope.is_runtime(".claude/skills/triaging-findings/tests/test_qa_triage_prompt.py") is True


def test_unclassified_new_path_fails_safe_to_runtime():
    assert ci_scope.is_runtime("brand-new-tool/config.toml") is True


def test_a_docs_only_change_set_skips_the_runtime_matrix():
    assert ci_scope.requires_runtime([
        "site/reports/release-native-verify/index.html",
        "docs/architecture.md",
        "CHANGELOG.md",
    ]) is False


def test_a_crates_change_in_a_mixed_change_set_requires_runtime():
    assert ci_scope.requires_runtime([
        "site/reports/release-native-verify/index.html",
        "crates/sc-lint/src/lib.rs",
    ]) is True


def test_empty_change_set_does_not_require_runtime():
    assert ci_scope.requires_runtime([]) is False


def _git(repo, *args):
    return subprocess.run(
        ["git", "-c", "user.name=test", "-c", "user.email=test@example.com", *args],
        cwd=repo, capture_output=True, text=True, check=True,
    ).stdout.strip()


def _commit(repo, path, text):
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")
    _git(repo, "add", path)
    _git(repo, "commit", "-q", "-m", f"change {path}")
    return _git(repo, "rev-parse", "HEAD")


def test_main_classifies_a_real_git_range(tmp_path, monkeypatch, capsys):
    repo = tmp_path / "repo"
    repo.mkdir()
    _git(repo, "init", "-q")
    base = _commit(repo, "crates/sc-lint/src/lib.rs", "one\n")
    docs_head = _commit(repo, "site/reports/index.html", "<html></html>\n")
    crates_head = _commit(repo, "crates/sc-lint/src/main.rs", "two\n")
    monkeypatch.chdir(repo)

    assert ci_scope.main(["ci_scope.py", base, docs_head]) == 0
    assert capsys.readouterr().out == "runtime=false\n"
    assert ci_scope.main(["ci_scope.py", base, crates_head]) == 0
    assert capsys.readouterr().out == "runtime=true\n"


def test_main_requires_exactly_two_revisions(capsys):
    assert ci_scope.main(["ci_scope.py", "only-one"]) == 2
    assert "usage: ci_scope.py" in capsys.readouterr().err
