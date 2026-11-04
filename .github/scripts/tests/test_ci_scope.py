"""Deterministic tests for the CI scope gate.

The git-backed tests run in a throwaway repository with a private HOME, no inherited
GIT_* variables, fixed commit dates, and no global, system or hook configuration, so
their results never depend on the developer's or the runner's git setup.
"""
import os
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ci_scope
from ci_scope_test_helpers import (
    _pytest_root_read_paths,
    _rust_read_expression_paths,
    parse_workflow,
    prefix_literal_read_paths,
    read_paths,
    tracked_paths,
)

FIXED_DATE = "2026-11-04T12:00:00+0000"


@pytest.mark.parametrize("path", [
    "site/reports/release-native-verify/index.html",
    "site/reports/release-native-verify/20261104T180000Z-prerelease-v1.2.3-run4242/index.json",
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
    "docs/requirements.md",
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


@pytest.mark.parametrize("base, head", [("not-a-sha", "b" * 40), ("a" * 40, "--output=/tmp/out"), ("0" * 40, "b" * 40)])
def test_a_pull_request_with_invalid_sha_inputs_requires_runtime(base, head, monkeypatch):
    def should_not_run(*_args, **_kwargs):
        raise AssertionError("invalid SHA inputs must not reach git")

    monkeypatch.setattr(ci_scope, "commit_exists", should_not_run)
    assert ci_scope.scope_requires_runtime("pull_request", base, head) is True


def test_a_pull_request_with_an_unreadable_range_requires_runtime(monkeypatch):
    def timeout(*_args, **_kwargs):
        raise subprocess.TimeoutExpired("git", 60)

    monkeypatch.setattr(ci_scope, "commit_exists", lambda _revision: True)
    monkeypatch.setattr(ci_scope, "changed_paths", timeout)
    assert ci_scope.scope_requires_runtime("pull_request", "a" * 40, "b" * 40) is True


def test_git_diff_places_revision_delimiter_after_valid_revisions(monkeypatch):
    captured = {}

    def run(command, **_kwargs):
        captured["command"] = command
        return subprocess.CompletedProcess(command, 0, stdout=b"docs/example.md\0")

    monkeypatch.setattr(ci_scope.subprocess, "run", run)
    assert ci_scope.changed_paths("a" * 40, "b" * 40) == ["docs/example.md", ""]
    assert captured["command"][-3:] == ["a" * 40, "b" * 40, "--"]


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
    monkeypatch.setenv("USERPROFILE", str(home))
    xdg_config = home / ".config"
    xdg_config.mkdir()
    monkeypatch.setenv("XDG_CONFIG_HOME", str(xdg_config))
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_SYSTEM", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
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
    ("docs/requirements.md", True),
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


def test_hermetic_git_fixture_sets_cross_platform_config_paths(hermetic_repo):
    assert os.environ["USERPROFILE"] == str(hermetic_repo.parent / "home")
    assert os.environ["XDG_CONFIG_HOME"] == str(hermetic_repo.parent / "home" / ".config")
    assert os.environ["GIT_CONFIG_NOSYSTEM"] == "1"
    assert os.environ["GIT_CONFIG_SYSTEM"] == os.devnull


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


def test_every_path_read_by_tests_or_lints_requires_runtime():
    reads = {}
    for source, paths in read_paths().items():
        for path in paths:
            reads.setdefault(path, set()).add(source)
    not_runtime = {path: sorted(sources) for path, sources in sorted(reads.items()) if not ci_scope.is_runtime(path)}
    assert not not_runtime, f"read by tests or lints but classified non-runtime: {not_runtime}"


def test_ci_workflow_parses_and_pins_conditions_runners_and_scope_env():
    workflow = parse_workflow()
    assert workflow["jobs"]["ci-scope"]["runs-on"] == "ubuntu-latest"
    assert "if" not in workflow["jobs"]["adoption"]
    assert "if" not in workflow["jobs"]["release-smoke"]

    expected_job_conditions = {
        "ci-scope": None,
        "just-lint": None,
        "test": None,
        "adoption": None,
        "wheel-runtime": "${{ needs.ci-scope.outputs.runtime != 'false' && (github.event_name == 'push' || github.base_ref == 'develop' || github.base_ref == 'main') }}",
        "release-smoke": None,
    }
    assert {name: job.get("if") for name, job in workflow["jobs"].items()} == expected_job_conditions

    runner = "${{ needs.ci-scope.outputs.runtime != 'false' && matrix.os || 'ubuntu-latest' }}"
    assert workflow["jobs"]["just-lint"]["runs-on"] == runner
    assert workflow["jobs"]["test"]["runs-on"] == runner
    assert workflow["jobs"]["adoption"]["runs-on"] == "${{ matrix.os }}"
    assert workflow["jobs"]["release-smoke"]["runs-on"] == "${{ matrix.os }}"
    assert workflow["jobs"]["wheel-runtime"]["runs-on"] == "ubuntu-latest"
    assert workflow["jobs"]["just-lint"]["name"] == "Just lint (${{ matrix.os }})"
    assert workflow["jobs"]["test"]["name"] == "Test (${{ matrix.os }})"
    expected_os_matrix = ["ubuntu-latest", "macos-latest", "windows-latest"]
    for job_name in ("just-lint", "test", "adoption", "release-smoke"):
        assert workflow["jobs"][job_name]["strategy"]["matrix"]["os"] == expected_os_matrix

    scope = next(step for step in workflow["jobs"]["ci-scope"]["steps"] if step.get("id") == "scope")
    assert scope["env"] == {
        "EVENT_NAME": "${{ github.event_name }}",
        "BASE_SHA": "${{ github.event.pull_request.base.sha }}",
        "HEAD_SHA": "${{ github.sha }}",
    }
    assert "${{" not in scope["run"]
    assert '--base "$BASE_SHA" --head "$HEAD_SHA"' in scope["run"]

    lint_steps = workflow["jobs"]["just-lint"]["steps"]
    spell_check = next(step for step in lint_steps if step.get("name") == "Documentation-only spell check")
    assert spell_check["run"].strip() == "python -m pip install codespell\ncodespell"
    assert spell_check["if"] == "${{ needs.ci-scope.outputs.runtime == 'false' }}"
    runtime_true = "${{ needs.ci-scope.outputs.runtime != 'false' }}"
    runtime_false = "${{ needs.ci-scope.outputs.runtime == 'false' }}"
    push_only = "${{ github.event_name == 'push' || github.base_ref == 'develop' || github.base_ref == 'main' }}"
    runtime_push = "${{ needs.ci-scope.outputs.runtime != 'false' && (github.event_name == 'push' || github.base_ref == 'develop' || github.base_ref == 'main') }}"
    assert [step.get("if") for step in workflow["jobs"]["ci-scope"]["steps"]] == [None, None]
    assert [step.get("if") for step in lint_steps] == [runtime_false, None, runtime_false, runtime_false, *([runtime_true] * 9)]
    assert [step.get("if") for step in workflow["jobs"]["test"]["steps"]] == [runtime_false, *([runtime_true] * 9)]
    assert [step.get("if") for step in workflow["jobs"]["adoption"]["steps"]] == [runtime_false, *([runtime_true] * 6)]
    assert [step.get("if") for step in workflow["jobs"]["wheel-runtime"]["steps"]] == [None, None, None, push_only, None]
    archive_only = "${{ needs.ci-scope.outputs.runtime != 'false' && github.event_name != 'push' && github.base_ref != 'develop' && github.base_ref != 'main' }}"
    assert [step.get("if") for step in workflow["jobs"]["release-smoke"]["steps"]] == [
        runtime_false, *([runtime_true] * 6), runtime_push, runtime_true, archive_only, runtime_push,
    ]


@pytest.mark.parametrize("job_name", ["adoption", "release-smoke"])
def test_docs_only_jobs_keep_matrix_names_with_step_level_skip(job_name):
    job = parse_workflow()["jobs"][job_name]
    steps = job["steps"]
    assert steps[0]["name"] == "Documentation-only skip"
    assert steps[0]["if"] == "${{ needs.ci-scope.outputs.runtime == 'false' }}"
    assert all(
        step.get("if", "").startswith("${{ needs.ci-scope.outputs.runtime != 'false'")
        for step in steps[1:]
    )


@pytest.mark.parametrize("source", ["tracked-source prefix literals", "Rust include/fs reads", "pytest repository-root reads", "lint files globs"])
def test_each_read_source_finds_paths_in_this_repository(source):
    assert read_paths()[source], f"no reads found for {source}; the scanner is out of date"


def test_closure_scanner_uses_tracked_files_and_tolerates_non_utf8(tmp_path, monkeypatch):
    for name in [name for name in os.environ if name.startswith("GIT_")]:
        monkeypatch.delenv(name)
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_SYSTEM", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    repo = tmp_path / "repo"
    repo.mkdir()
    monkeypatch.chdir(repo)
    _git(repo, "init", "-q")
    tracked = repo / "crates" / "vendor.rs"
    tracked.parent.mkdir()
    tracked.write_bytes(b"// invalid: \xff\n")
    _git(repo, "add", "--all")
    untracked = repo / "scratchpad" / "test_untracked.py"
    untracked.parent.mkdir()
    untracked.write_text('REPO / "docs" / "requirements.md"', encoding="utf-8")
    paths = tracked_paths(repo)
    assert paths == [tracked]
    assert "docs/requirements.md" not in prefix_literal_read_paths(repo)


def test_rust_read_scanner_covers_fs_reads_manifest_joins_and_concat(hermetic_repo):
    repo = hermetic_repo
    crate = repo / "crates" / "sample"
    source = crate / "src" / "tests.rs"
    source.parent.mkdir(parents=True)
    (crate / "Cargo.toml").write_text("[package]\nname='sample'\nversion='0.1.0'\n", encoding="utf-8")
    (repo / "docs" / "sc-lint").mkdir(parents=True)
    expected = repo / "docs" / "sc-lint" / "contract.md"
    expected.write_text("contract\n", encoding="utf-8")
    source.write_text(
        """
const A: Vec<u8> = fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/sc-lint/contract.md")).unwrap();
const B: String = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .join("docs")
    .join("sc-lint")
    .join("contract.md")).unwrap();
""",
        encoding="utf-8",
    )
    _git(repo, "add", "--all")
    assert _rust_read_expression_paths(repo) == {"docs/sc-lint/contract.md"}


def test_python_read_scanner_covers_file_parent_roots_and_joinpath(hermetic_repo):
    repo = hermetic_repo
    source = repo / "tests" / "unit" / "test_reader.py"
    source.parent.mkdir(parents=True)
    contract = repo / "docs" / "sc-lint" / "contract.md"
    contract.parent.mkdir(parents=True)
    contract.write_text("contract\n", encoding="utf-8")
    source.write_text(
        """
from pathlib import Path
ROOTED = Path(__file__).resolve().parents[2] / "docs" / "sc-lint" / "contract.md"
JOINED = Path(__file__).resolve().parents[2].joinpath("docs", "sc-lint", "contract.md")
""",
        encoding="utf-8",
    )
    _git(repo, "add", "--all")
    assert _pytest_root_read_paths(repo) == {"docs/sc-lint/contract.md"}


def test_ci_workflow_skips_only_when_the_scope_output_is_false():
    workflow = parse_workflow()
    for name in ("just-lint", "test"):
        conditions = [step.get("if") for step in workflow["jobs"][name]["steps"] if step.get("if")]
        assert all(condition in {
            "${{ needs.ci-scope.outputs.runtime == 'false' }}",
            "${{ needs.ci-scope.outputs.runtime != 'false' }}",
        } for condition in conditions)
        assert "${{ needs.ci-scope.outputs.runtime == 'false' }}" in conditions
        assert "${{ needs.ci-scope.outputs.runtime != 'false' }}" in conditions
