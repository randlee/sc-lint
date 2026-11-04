"""Deterministic tests for the CI scope gate.

The git-backed tests run in a throwaway repository with a private HOME, no inherited
GIT_* variables, fixed commit dates, and no global, system or hook configuration, so
their results never depend on the developer's or the runner's git setup.
"""
import glob
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_scope

FIXED_DATE = "2026-11-04T12:00:00+0000"
EXCLUDED_DIRS = {".git", "target", ".venv", "node_modules", "__pycache__"}


@pytest.mark.parametrize("path", [
    "site/reports/release-native-verify/index.html",
    "site/reports/release-native-verify/20261104T180000Z-prerelease-v1.2.3-run4242/index.json",
    "docs/requirements.md",
    "docs/project-plan.md",
    ".beads/issues.jsonl",
    ".atm-bd/sc-lint.bd-cache.sqlite3",
    ".triage/phase-ao2/findings/AO210.ttl",
])
def test_documentation_and_metadata_paths_do_not_require_runtime(path):
    assert ci_scope.is_runtime(path) is False


@pytest.mark.parametrize("path", [
    "site/index.html",
    "site/announcements/sc-lint-v0.8.2.md",
    "README.md",
    "AGENTS.md",
    "CHANGELOG.md",
    "docs/sc-lint/adoption.md",
    "docs/sc-lint/cli-contract.md",
    ".atm.toml",
    ".claude/agents/publisher.md",
    ".claude/skills/publishing/ref/release-state-strategy.md",
    ".claude/skills/publishing/preflight.xml.j2",
    ".claude-plugin/marketplace.json",
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
    "brand-new-tool/config.toml",
])
def test_every_other_path_requires_runtime(path):
    assert ci_scope.is_runtime(path) is True


def test_python_under_a_documentation_prefix_is_still_runtime():
    assert ci_scope.is_runtime("docs/examples/check_links.py") is True


def test_a_docs_only_change_set_skips_the_runtime_matrix():
    assert ci_scope.requires_runtime([
        "site/reports/release-native-verify/index.html",
        "docs/architecture.md",
        ".beads/issues.jsonl",
    ]) is False


def test_a_readme_or_changelog_change_requires_runtime():
    assert ci_scope.requires_runtime(["site/reports/index.html", "CHANGELOG.md"]) is True


def test_a_crates_change_in_a_mixed_change_set_requires_runtime():
    assert ci_scope.requires_runtime([
        "site/reports/release-native-verify/index.html",
        "crates/sc-lint/src/lib.rs",
    ]) is True


def test_empty_change_set_does_not_require_runtime():
    assert ci_scope.requires_runtime([]) is False


def test_a_push_to_develop_or_main_always_requires_runtime(monkeypatch):
    def no_git(*_args, **_kwargs):
        raise AssertionError("a push must not read the change set")

    monkeypatch.setattr(ci_scope, "changed_paths", no_git)
    assert ci_scope.scope_requires_runtime("push", "", "") is True
    assert ci_scope.scope_requires_runtime("push", "a" * 40, "b" * 40) is True


@pytest.mark.parametrize("base", ["", "0" * 40])
def test_a_pull_request_without_a_real_base_requires_runtime(base, monkeypatch):
    monkeypatch.setattr(ci_scope, "commit_exists", lambda _revision: False)
    assert ci_scope.scope_requires_runtime("pull_request", base, "b" * 40) is True


def test_a_pull_request_whose_base_is_not_in_the_clone_requires_runtime(monkeypatch):
    monkeypatch.setattr(ci_scope, "commit_exists", lambda _revision: False)
    assert ci_scope.scope_requires_runtime("pull_request", "a" * 40, "b" * 40) is True


def test_a_pull_request_without_a_head_requires_runtime(monkeypatch):
    monkeypatch.setattr(ci_scope, "commit_exists", lambda _revision: True)
    assert ci_scope.scope_requires_runtime("pull_request", "a" * 40, "") is True


@pytest.fixture
def hermetic_repo(tmp_path, monkeypatch):
    """A throwaway repository whose git behaviour depends only on this test."""
    for name in [name for name in os.environ if name.startswith("GIT_")]:
        monkeypatch.delenv(name)
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setenv("HOME", str(home))
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_SYSTEM", os.devnull)
    monkeypatch.setenv("GIT_AUTHOR_DATE", FIXED_DATE)
    monkeypatch.setenv("GIT_COMMITTER_DATE", FIXED_DATE)
    repo = tmp_path / "repo"
    repo.mkdir()
    monkeypatch.chdir(repo)
    _git(repo, "init", "-q")
    return repo


def _git(repo, *args):
    return subprocess.run(
        ["git", "-c", "user.name=test", "-c", "user.email=test@example.com",
         "-c", "commit.gpgsign=false", "-c", "core.hooksPath=", *args],
        cwd=repo, capture_output=True, text=True, check=True, timeout=60,
    ).stdout.strip()


def _commit(repo, path, text):
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")
    _git(repo, "add", path)
    _git(repo, "commit", "-q", "-m", f"change {path}")
    return _git(repo, "rev-parse", "HEAD")


@pytest.mark.parametrize("path, runtime", [
    ("site/reports/release-native-verify/index.html", False),
    ("docs/requirements.md", False),
    ("docs/sc-lint/adoption.md", True),
    ("README.md", True),
    ("CHANGELOG.md", True),
    (".claude/agents/publisher.md", True),
    ("crates/sc-lint/src/lib.rs", True),
])
def test_main_classifies_a_real_pull_request_range(hermetic_repo, capsys, path, runtime):
    base = _commit(hermetic_repo, "crates/sc-lint/src/base.rs", "base\n")
    head = _commit(hermetic_repo, path, "change\n")
    assert ci_scope.main(["--event", "pull_request", "--base", base, "--head", head]) == 0
    assert capsys.readouterr().out == f"runtime={str(runtime).lower()}\n"


def test_main_classifies_a_range_with_a_docs_commit_then_a_crates_commit(hermetic_repo, capsys):
    base = _commit(hermetic_repo, "crates/sc-lint/src/lib.rs", "one\n")
    docs_head = _commit(hermetic_repo, "site/reports/index.html", "<html></html>\n")
    crates_head = _commit(hermetic_repo, "crates/sc-lint/src/main.rs", "two\n")
    assert ci_scope.main(["--event", "pull_request", "--base", base, "--head", docs_head]) == 0
    assert capsys.readouterr().out == "runtime=false\n"
    assert ci_scope.main(["--event", "pull_request", "--base", base, "--head", crates_head]) == 0
    assert capsys.readouterr().out == "runtime=true\n"


def test_main_runs_the_matrix_when_the_base_is_all_zeros(hermetic_repo, capsys):
    head = _commit(hermetic_repo, "site/reports/index.html", "<html></html>\n")
    assert ci_scope.main(["--event", "pull_request", "--base", "0" * 40, "--head", head]) == 0
    assert capsys.readouterr().out == "runtime=true\n"


def test_main_runs_the_matrix_for_a_push_without_reading_git(capsys):
    assert ci_scope.main(["--event", "push", "--base", "", "--head", "f" * 40]) == 0
    assert capsys.readouterr().out == "runtime=true\n"


def test_main_requires_an_event(capsys):
    with pytest.raises(SystemExit) as exit_info:
        ci_scope.main([])
    assert exit_info.value.code == 2
    assert "--event" in capsys.readouterr().err


def _file_globs(node):
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "files" and isinstance(value, list):
                yield from value
            else:
                yield from _file_globs(value)
    elif isinstance(node, list):
        for item in node:
            yield from _file_globs(item)


def _rel(path):
    return path.relative_to(REPO).as_posix()


def _walk_files(top):
    """Every file under top, pruning build output and VCS directories before descending."""
    for root, dirs, names in os.walk(top):
        dirs[:] = [name for name in dirs if name not in EXCLUDED_DIRS]
        for name in names:
            yield Path(root, name)


INCLUDE_READ = re.compile(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)')


def include_read_paths():
    """Paths named by include_str!/include_bytes! in crates, resolved from the source file."""
    for source in _walk_files(REPO / "crates"):
        if source.suffix != ".rs":
            continue
        for match in INCLUDE_READ.finditer(source.read_text(encoding="utf-8")):
            target = (source.parent / match.group(1)).resolve()
            if target.is_file():
                yield _rel(target)


def lint_glob_read_paths():
    """Files matched by a `files` glob in the lint configuration."""
    config = tomllib.loads((REPO / ".just/lint-config.toml").read_text(encoding="utf-8"))
    for pattern in _file_globs(config):
        for match in glob.glob(str(REPO / pattern), recursive=True):
            if Path(match).is_file():
                yield _rel(Path(match))


PYTEST_READ = re.compile(r'\b(?:ROOT|REPO_ROOT|PACKAGE_ROOT|repo_root\(\))\s*((?:/\s*"[^"]+"\s*)+)')


def pytest_read_paths():
    """Repo paths that pytest suites join onto their root, resolved against the repository root."""
    for source in _walk_files(REPO):
        if source.suffix != ".py":
            continue
        parts = source.relative_to(REPO).parts
        if not (source.name.startswith("test_") or source.name == "conftest.py" or "tests" in parts):
            continue
        for match in PYTEST_READ.finditer(source.read_text(encoding="utf-8")):
            segments = re.findall(r'"([^"]+)"', match.group(1))
            target = REPO.joinpath(*segments)
            if target.is_file():
                yield _rel(target)
            elif target.is_dir():
                for child in _walk_files(target):
                    yield _rel(child)


READ_SOURCES = {
    "include_str!/include_bytes! in crates": include_read_paths,
    "files glob in .just/lint-config.toml": lint_glob_read_paths,
    "pytest repo-root read": pytest_read_paths,
}


@pytest.mark.parametrize("source", list(READ_SOURCES))
def test_each_read_source_finds_paths_in_this_repository(source):
    assert set(READ_SOURCES[source]()), f"no reads found for {source}; the scanner is out of date"


def test_every_path_read_by_tests_or_lints_requires_runtime():
    reads = {}
    for source, finder in READ_SOURCES.items():
        for path in finder():
            reads.setdefault(path, set()).add(source)
    not_runtime = {path: sorted(sources) for path, sources in sorted(reads.items()) if not ci_scope.is_runtime(path)}
    assert not not_runtime, f"read by tests or lints but classified non-runtime: {not_runtime}"


def test_ci_workflow_skips_only_when_the_scope_output_is_false():
    workflow = (REPO / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    assert "outputs.runtime == 'true'" not in workflow
    assert "outputs.runtime != 'true'" not in workflow
