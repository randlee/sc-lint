"""Deterministic fixture tests for the native-matrix publish gate.

The git tests run in a scrubbed environment: no GIT_* variables from the caller, no global or
system config, a private HOME, unsigned commits, hooks disabled and fixed commit dates.
"""
import ast
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_native_common as common
import release_native_gate as gate
import release_native_matrix as matrix

ROOT = Path(__file__).resolve().parents[3]
MANIFEST = ROOT / common.MANIFEST_PATH
TAG = "prerelease/v1.2.3"
VERIFIED = "a" * 40
CANDIDATE = "c" * 40
TITLE = common.verify_run_title(TAG, VERIFIED)
TARGETS = [row["target"] for row in matrix.native_matrix(tomllib.loads(MANIFEST.read_text(encoding="utf-8")))["include"]]
PROVEN = {common.native_job(target): "success" for target in TARGETS} | {common.REPORT_JOB: "success"}
DEFAULT_BRANCH = "main"


@pytest.fixture(autouse=True)
def hermetic_git(monkeypatch, tmp_path):
    for name in [name for name in os.environ if name.startswith("GIT_")]:
        monkeypatch.delenv(name)
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setenv("HOME", str(home))
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", os.devnull)
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    monkeypatch.setenv("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
    monkeypatch.setenv("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")


VERIFY_PATH = ".github/workflows/release-native-verify.yml"


def run(run_id, title, *, conclusion="success", status="completed", event="workflow_run", head_branch=TAG, path=VERIFY_PATH):
    """A verify run as the workflow runs API reports it; a workflow_run from the tag push by default."""
    return {
        "id": run_id, "display_title": title, "status": status, "conclusion": conclusion,
        "event": event, "head_branch": head_branch, "path": path,
    }


def page(*runs, total=None):
    return {"total_count": len(runs) if total is None else total, "workflow_runs": list(runs)}


class FakeGitHub:
    """Serves canned endpoints and records each one. A run without canned jobs has every required job succeed."""

    def __init__(self, pages=None, *, release_published=True, jobs=None):
        self.pages = pages or {}
        self.release_published = release_published
        self.jobs = jobs or {}
        self.calls = []

    def default_branch(self):
        return DEFAULT_BRANCH

    def json(self, endpoint):
        self.calls.append(endpoint)
        if endpoint.startswith("releases/tags/"):
            if not self.release_published:
                raise common.GitHubError(endpoint, 404)
            return {"assets": []}
        if endpoint.startswith("actions/runs/"):
            conclusions = self.jobs.get(int(endpoint.split("/")[2]), PROVEN)
            return {"total_count": len(conclusions), "jobs": [{"name": name, "conclusion": value} for name, value in conclusions.items()]}
        prefix = f"actions/workflows/{gate.VERIFY_WORKFLOW_FILE}/runs?"
        assert endpoint.startswith(prefix), endpoint
        query = parse_qs(urlsplit(endpoint).query)
        assert query["status"] == ["success"]
        return self.pages.get(int(query["page"][0]), page())

    def manifest(self, tag):
        self.calls.append(f"manifest:{tag}")
        return tomllib.loads(MANIFEST.read_text(encoding="utf-8"))


def diff_of(*paths, manifest_build_changed=False):
    return gate.classify(list(paths), manifest_build_changed=manifest_build_changed)


# Run lookup


def test_newest_successful_run_with_matching_title_is_selected():
    client = FakeGitHub({1: page(
        run(9, TITLE, conclusion="failure"),
        run(8, common.verify_run_title("prerelease/v1.2.4", VERIFIED)),
        run(7, TITLE),
        run(5, TITLE),
    )})
    assert gate.find_verified_run(client, TAG, VERIFIED, TARGETS) == 7


def test_run_for_another_tag_is_ignored():
    client = FakeGitHub({1: page(run(12, common.verify_run_title("prerelease/v1.2.4", VERIFIED)))})
    with pytest.raises(gate.GateError, match="no successful Release Native Verify run exists for prerelease/v1.2.3 at aaaa"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_run_for_a_moved_tag_does_not_satisfy_the_gate():
    older_build = common.verify_run_title(TAG, "b" * 40)
    client = FakeGitHub({1: page(run(12, older_build))})
    with pytest.raises(gate.GateError, match="no successful Release Native Verify run exists"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_failed_or_unfinished_matching_runs_fail_closed():
    client = FakeGitHub({1: page(
        run(3, TITLE, conclusion="failure"),
        run(2, TITLE, status="in_progress", conclusion=None),
    )})
    with pytest.raises(gate.GateError, match="no successful"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_no_verify_runs_fails_closed():
    with pytest.raises(gate.GateError, match=f"dispatch Release Native Verify with tag {TAG} and commit {VERIFIED}"):
        gate.find_verified_run(FakeGitHub(), TAG, VERIFIED, TARGETS)


@pytest.mark.parametrize("override", [
    {"path": ".github/workflows/other.yml"},
    {"event": "pull_request"},
    {"event": "workflow_dispatch", "head_branch": "feature/x"},
])
def test_only_the_verify_workflow_on_a_trusted_event_can_prove_a_publish(override):
    client = FakeGitHub({1: page(run(7, TITLE, **override))})
    with pytest.raises(gate.GateError, match="no successful"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_a_dispatch_from_the_default_branch_proves_a_publish():
    client = FakeGitHub({1: page(run(7, TITLE, event="workflow_dispatch", head_branch=DEFAULT_BRANCH))})
    assert gate.find_verified_run(client, TAG, VERIFIED, TARGETS) == 7


def test_a_success_run_whose_native_jobs_were_skipped_proves_nothing():
    skipped = {name: "skipped" for name in PROVEN}
    client = FakeGitHub({1: page(run(7, TITLE))}, jobs={7: skipped})

    with pytest.raises(gate.GateError, match="rejected: run 7: jobs did not all succeed"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_a_failed_native_job_in_a_success_run_is_named():
    failed = dict(PROVEN, **{common.native_job(TARGETS[0]): "failure"})
    client = FakeGitHub({1: page(run(7, TITLE))}, jobs={7: failed})

    with pytest.raises(gate.GateError, match=rf"{common.native_job(TARGETS[0])} \(failure\)"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_a_run_without_the_report_job_is_not_proof():
    without_report = {name: value for name, value in PROVEN.items() if name != common.REPORT_JOB}
    client = FakeGitHub({1: page(run(7, TITLE))}, jobs={7: without_report})

    with pytest.raises(gate.GateError, match=rf"{common.REPORT_JOB} \(absent\)"):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)


def test_an_older_proven_run_is_used_when_the_newest_is_unproven():
    skipped = {name: "skipped" for name in PROVEN}
    client = FakeGitHub({1: page(run(9, TITLE), run(7, TITLE))}, jobs={9: skipped})

    assert gate.find_verified_run(client, TAG, VERIFIED, TARGETS) == 7


def test_pagination_finds_matching_run_on_second_page():
    other_runs = [run(1000 + index, common.verify_run_title("prerelease/v1.2.4", VERIFIED)) for index in range(common.PAGE_SIZE)]
    client = FakeGitHub({
        1: page(*other_runs, total=common.PAGE_SIZE + 1),
        2: page(run(7, TITLE), total=common.PAGE_SIZE + 1),
    })
    assert gate.find_verified_run(client, TAG, VERIFIED, TARGETS) == 7
    assert [parse_qs(urlsplit(call).query)["page"][0] for call in client.calls if "/runs?" in call] == ["1", "2"]


def test_search_stops_at_the_page_limit():
    full_pages = {number: page(*[run(number * 1000 + index, "other") for index in range(common.PAGE_SIZE)],
                               total=10_000)
                  for number in range(1, common.MAX_PAGES + 2)}
    client = FakeGitHub(full_pages)
    with pytest.raises(gate.GateError):
        gate.find_verified_run(client, TAG, VERIFIED, TARGETS)
    assert len([call for call in client.calls if "/runs?" in call]) == common.MAX_PAGES


def test_unpublished_prerelease_fails():
    with pytest.raises(gate.GateError, match=r"prerelease/v1\.2\.3 has no published prerelease \(HTTP 404\)"):
        gate.check_prerelease_release(FakeGitHub(release_published=False), TAG)


def test_published_prerelease_is_looked_up_with_an_encoded_slash():
    client = FakeGitHub()
    gate.check_prerelease_release(client, TAG)
    assert client.calls == ["releases/tags/prerelease%2Fv1.2.3"]


# Ancestry and artifact-input rule


@pytest.mark.parametrize("changed", [
    ["docs/requirements.md", "CHANGELOG.md", "site/index.html", "README.md"],
    ["site/reports/release-native-verify/index.html", "release/README.md"],
    [".github/workflows/pypi-publish.yml", ".github/workflows/release.yml", ".github/workflows/homebrew-publish.yml"],
    [".github/workflows/release-preflight.yml", ".github/workflows/release-native-verify.yml"],
    [".github/scripts/release_native_verify.py", ".github/scripts/release_native_gate.py",
     ".github/scripts/release_native_common.py", ".github/scripts/tests/test_release_native_gate.py"],
])
def test_changes_outside_artifact_inputs_pass_without_a_rerun(changed):
    gate.evaluate(TAG, VERIFIED, CANDIDATE, is_ancestor=True, diff=diff_of(*changed))


def test_identical_commits_pass_with_no_changes():
    gate.evaluate(TAG, VERIFIED, VERIFIED, is_ancestor=True, diff=diff_of())


@pytest.mark.parametrize("changed, offending", [
    (["docs/requirements.md", "crates/sc-lint/src/main.rs"], "crates/sc-lint/src/main.rs"),
    (["crates/sc-lint-boundary/Cargo.toml"], "crates/sc-lint-boundary/Cargo.toml"),
    (["bindings/sc-lint-py/python/sc_lint/__init__.py"], "bindings/sc-lint-py/python/sc_lint/__init__.py"),
    (["Cargo.lock"], "Cargo.lock"),
    (["rust-toolchain.toml"], "rust-toolchain.toml"),
    (["docs-bundle/cli-reference/index.md"], "docs-bundle/cli-reference/index.md"),
    ([".github/scripts/build_release_target.sh"], ".github/scripts/build_release_target.sh"),
    ([".github/scripts/release_stage.py"], ".github/scripts/release_stage.py"),
    ([".github/scripts/release_manifest.py"], ".github/scripts/release_manifest.py"),
    ([".github/scripts/release_platforms.py"], ".github/scripts/release_platforms.py"),
    ([".github/scripts/wheel_contents.py"], ".github/scripts/wheel_contents.py"),
    ([".github/actions/install-linux-native-deps/action.yml"], ".github/actions/install-linux-native-deps/action.yml"),
    ([".github/actions/setup-python-release-build/action.yml"], ".github/actions/setup-python-release-build/action.yml"),
    ([".cargo/config.toml"], ".cargo/config.toml"),
])
def test_artifact_input_change_fails_naming_the_path(changed, offending):
    with pytest.raises(gate.GateError) as error:
        gate.evaluate(TAG, VERIFIED, CANDIDATE, is_ancestor=True, diff=diff_of(*changed))
    assert offending in str(error.value)
    assert "cut a new prerelease and rerun Release Native Verify" in str(error.value)


def test_manifest_build_sections_are_artifact_inputs_only_when_they_changed():
    assert diff_of(common.MANIFEST_PATH, manifest_build_changed=True).artifact_inputs == (common.MANIFEST_PATH,)
    assert diff_of(common.MANIFEST_PATH, manifest_build_changed=False).other == (common.MANIFEST_PATH,)


def test_verified_commit_that_is_not_an_ancestor_fails():
    with pytest.raises(gate.GateError, match="not an ancestor of the release candidate"):
        gate.evaluate(TAG, VERIFIED, CANDIDATE, is_ancestor=False, diff=diff_of())


@pytest.mark.parametrize("verified, candidate", [("main", CANDIDATE), (VERIFIED, "v1.2.3")])
def test_commit_identities_must_be_full_shas(verified, candidate):
    with pytest.raises(gate.GateError, match="full 40-character SHAs"):
        gate.evaluate(TAG, verified, candidate, is_ancestor=True, diff=diff_of())


def test_classification_splits_artifact_inputs_from_the_rest():
    diff = diff_of("docs-bundle/index.md", "CHANGELOG.md", "crates/sc-lint/src/lib.rs", "site/index.html")
    assert diff.artifact_inputs == ("crates/sc-lint/src/lib.rs", "docs-bundle/index.md")
    assert diff.other == ("CHANGELOG.md", "site/index.html")


def test_artifact_inputs_cover_every_manifest_declared_build_path():
    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    declared = [crate["cargo_toml"] for crate in manifest["crates"]]
    declared += [package["manifest"] for package in manifest["python_packages"]]
    declared += [distribution["source"] for distribution in manifest["python_distributions"]]
    declared += [path["source"] for binary in manifest["release_binaries"] for path in binary.get("bundled_paths", [])]
    declared += ["Cargo.toml", "Cargo.lock", manifest["project"]["workspace_toml"], "rust-toolchain.toml"]
    uncovered = [path for path in declared if not gate.is_artifact_input(path)]
    assert uncovered == []


SCRIPT_REFERENCE = re.compile(r"\.github/(?:scripts|actions)/[A-Za-z0-9_./-]+")


def imported_modules(source: str) -> set[str]:
    """Top-level module names a Python source imports, including imports inside functions."""
    names: set[str] = set()
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Import):
            names |= {alias.name.split(".")[0] for alias in node.names}
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
            names.add(node.module.split(".")[0])
    return names


def build_script_closure() -> set[str]:
    """The files prerelease-archive.yml runs, and every local file they reach.

    Python sources contribute the local modules they import, lazy imports included. Shell
    scripts, actions and the workflow contribute the .github/scripts and .github/actions paths
    they name; an action's file is its action.yml. Only paths that exist are followed.
    """
    pending = {".github/workflows/prerelease-archive.yml"}
    covered: set[str] = set()
    while pending:
        path = pending.pop()
        if path in covered:
            continue
        covered.add(path)
        text = (ROOT / path).read_text(encoding="utf-8")
        if path.endswith(".py"):
            pending |= {f".github/scripts/{module}.py" for module in imported_modules(text)}
        for reference in SCRIPT_REFERENCE.findall(text):
            if reference.startswith(".github/actions/") and not reference.endswith((".yml", ".yaml")):
                reference = f"{reference.rstrip('/')}/action.yml"
            pending.add(reference)
        pending = {candidate for candidate in pending if (ROOT / candidate).is_file() or candidate in covered}
    return covered


def test_every_script_and_action_the_archive_build_runs_is_an_artifact_input():
    covered = build_script_closure()
    assert ".github/scripts/release_stage.py" in covered
    assert ".github/scripts/wheel_contents.py" in covered
    uncovered = [path for path in sorted(covered) if not gate.is_artifact_input(path)]
    assert uncovered == []


def test_root_cargo_files_are_artifact_inputs():
    for path in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        assert gate.is_artifact_input(path), path


def test_closure_follows_imports_inside_functions_and_from_imports():
    assert imported_modules("def build():\n    import release_registry\n") == {"release_registry"}
    assert imported_modules("from release_stage import run\nimport json.tool\n") == {"release_stage", "json"}


# Real git history: ancestry, renames and the manifest's build sections


def git(repo, *args):
    return subprocess.run(
        ["git", "-c", "user.name=test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false",
         "-c", f"core.hooksPath={os.devnull}", *args],
        cwd=repo, capture_output=True, text=True, check=True,
    ).stdout.strip()


def commit_file(repo, path, text, message):
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")
    git(repo, "add", path)
    git(repo, "commit", "-q", "-m", message)
    return git(repo, "rev-parse", "HEAD")


def new_repo(tmp_path, monkeypatch):
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    git(repo, "config", "diff.renames", "true")
    monkeypatch.chdir(repo)
    return repo


def test_git_helpers_report_ancestry_and_changed_paths(tmp_path, monkeypatch):
    repo = new_repo(tmp_path, monkeypatch)
    verified = commit_file(repo, "crates/sc-lint/src/lib.rs", "one\n", "verified")
    commit_file(repo, "docs/notes.md", "notes\n", "docs only")
    candidate = commit_file(repo, "CHANGELOG.md", "entry\n", "changelog")

    assert gate.git_is_ancestor(verified, candidate) is True
    assert gate.git_is_ancestor(candidate, verified) is False
    assert gate.git_changed_paths(verified, candidate) == ["CHANGELOG.md", "docs/notes.md"]


def test_git_changed_paths_names_a_space_in_the_path(tmp_path, monkeypatch):
    repo = new_repo(tmp_path, monkeypatch)
    base = commit_file(repo, "README.md", "x\n", "base")
    head = commit_file(repo, "crates/with space.rs", "y\n", "spaced path")

    assert gate.git_changed_paths(base, head) == ["crates/with space.rs"]


def test_renaming_an_artifact_input_out_of_its_directory_is_still_classified(tmp_path, monkeypatch):
    repo = new_repo(tmp_path, monkeypatch)
    verified = commit_file(repo, "crates/sc-lint/src/lib.rs", "one\n", "source")
    (repo / "docs").mkdir()
    git(repo, "mv", "crates/sc-lint/src/lib.rs", "docs/lib.rs")
    git(repo, "commit", "-q", "-m", "move source into docs")
    candidate = git(repo, "rev-parse", "HEAD")

    paths = gate.git_changed_paths(verified, candidate)
    assert paths == ["crates/sc-lint/src/lib.rs", "docs/lib.rs"]
    diff = gate.classify(paths, manifest_build_changed=False)
    assert diff.artifact_inputs == ("crates/sc-lint/src/lib.rs",)
    with pytest.raises(gate.GateError, match="crates/sc-lint/src/lib.rs"):
        gate.evaluate(TAG, verified, candidate, is_ancestor=True, diff=diff)


def test_renaming_a_docs_file_into_an_artifact_directory_is_classified(tmp_path, monkeypatch):
    repo = new_repo(tmp_path, monkeypatch)
    verified = commit_file(repo, "docs/lib.rs", "one\n", "notes")
    (repo / "crates").mkdir()
    git(repo, "mv", "docs/lib.rs", "crates/lib.rs")
    git(repo, "commit", "-q", "-m", "move notes into crates")
    candidate = git(repo, "rev-parse", "HEAD")

    paths = gate.git_changed_paths(verified, candidate)
    assert paths == ["crates/lib.rs", "docs/lib.rs"]
    assert gate.classify(paths, manifest_build_changed=False).artifact_inputs == ("crates/lib.rs",)


def edit_manifest(text, old, new):
    """Replace one line of the real manifest; the fixture must contain the line it edits."""
    edited = text.replace(old, new, 1)
    assert edited != text, f"manifest fixture has no {old!r} to edit"
    return edited


def test_manifest_edits_to_publish_only_sections_do_not_force_a_rerun(tmp_path, monkeypatch):
    repo = new_repo(tmp_path, monkeypatch)
    text = MANIFEST.read_text(encoding="utf-8")
    base = commit_file(repo, common.MANIFEST_PATH, text, "manifest")
    publish_only = commit_file(
        repo, common.MANIFEST_PATH, edit_manifest(text, 'workflow = "pypi-publish.yml"', 'workflow = "pypi-publish-2.yml"'),
        "channel",
    )
    build_change = commit_file(
        repo, common.MANIFEST_PATH, edit_manifest(text, 'os = "ubuntu-latest"', 'os = "ubuntu-24.04"'), "runner",
    )
    python_package = commit_file(
        repo, common.MANIFEST_PATH, edit_manifest(text, "[[python_packages]]", '[[python_packages]]\nfixture_marker = "edit"'),
        "python package",
    )

    assert gate.manifest_build_changed(base, publish_only) is False
    assert gate.manifest_build_changed(base, build_change) is True
    assert gate.manifest_build_changed(base, python_package) is True


# main(): output contract and fail-closed exits


def _main_args(verified=VERIFIED, candidate=CANDIDATE, output=None):
    args = ["--tag", TAG, "--prerelease-commit", verified, "--candidate-commit", candidate]
    if output is not None:
        args += ["--output", str(output)]
    return args


def _use_fixture(monkeypatch, client, *, is_ancestor=True, changed=(), manifest_changed=False):
    monkeypatch.setattr(gate, "GitHub", lambda repository, token: client)
    monkeypatch.setattr(gate, "git_is_ancestor", lambda ancestor, descendant: is_ancestor)
    monkeypatch.setattr(gate, "git_changed_paths", lambda base, head: list(changed))
    monkeypatch.setattr(gate, "manifest_build_changed", lambda base, head: manifest_changed)
    monkeypatch.setenv("GITHUB_REPOSITORY", "randlee/sc-lint")
    monkeypatch.setenv("GH_TOKEN", "test-token")


def test_main_writes_run_id_and_verified_commit_on_success(monkeypatch, tmp_path, capsys):
    client = FakeGitHub({1: page(run(7, TITLE))})
    _use_fixture(monkeypatch, client, changed=["docs/x.md", "CHANGELOG.md"])
    output = tmp_path / "github_output"
    output.write_text("existing=1\n", encoding="utf-8")

    status = gate.main(_main_args(output=output))

    assert status == 0
    assert output.read_text(encoding="utf-8") == f"existing=1\nnative_verify_run_id=7\nnative_verified_commit={VERIFIED}\n"
    assert f"manifest:{VERIFIED}" in client.calls
    out = capsys.readouterr().out
    assert "Release Native Verify run 7 on commit " + VERIFIED in out
    assert "Artifact inputs changed since the verified commit: none" in out
    assert "need no rerun: 2" in out


def test_main_fails_when_an_artifact_input_changed_and_writes_nothing(monkeypatch, tmp_path, capsys):
    client = FakeGitHub({1: page(run(7, TITLE))})
    _use_fixture(monkeypatch, client, changed=["crates/sc-lint/src/lib.rs"])
    output = tmp_path / "github_output"

    status = gate.main(_main_args(output=output))

    assert status == 1
    assert not output.exists()
    assert "release-native-gate: artifact inputs changed" in capsys.readouterr().err


def test_main_fails_when_the_manifest_build_sections_changed(monkeypatch, capsys):
    _use_fixture(monkeypatch, FakeGitHub({1: page(run(7, TITLE))}), changed=[common.MANIFEST_PATH], manifest_changed=True)

    assert gate.main(_main_args()) == 1
    assert common.MANIFEST_PATH in capsys.readouterr().err


def test_main_fails_closed_without_a_verify_run(monkeypatch, tmp_path, capsys):
    _use_fixture(monkeypatch, FakeGitHub())
    output = tmp_path / "github_output"

    status = gate.main(_main_args(output=output))

    assert status == 1
    assert not output.exists()
    assert "release-native-gate: no successful Release Native Verify run" in capsys.readouterr().err


def test_main_fails_when_the_only_run_is_for_another_build_of_the_tag(monkeypatch, capsys):
    _use_fixture(monkeypatch, FakeGitHub({1: page(run(7, common.verify_run_title(TAG, "b" * 40)))}))

    assert gate.main(_main_args()) == 1
    assert "no successful Release Native Verify run exists" in capsys.readouterr().err


def test_main_fails_when_the_verified_commit_is_not_an_ancestor(monkeypatch, capsys):
    _use_fixture(monkeypatch, FakeGitHub({1: page(run(7, TITLE))}), is_ancestor=False)

    assert gate.main(_main_args()) == 1
    assert "not an ancestor of the release candidate" in capsys.readouterr().err


def test_main_rejects_a_malformed_commit_before_calling_github(monkeypatch, capsys):
    client = FakeGitHub({1: page(run(7, TITLE))})
    _use_fixture(monkeypatch, client)

    assert gate.main(_main_args(verified="main")) == 1
    assert "full 40-character SHAs" in capsys.readouterr().err
    assert client.calls == []


def test_main_accepts_only_prerelease_tags(monkeypatch, capsys):
    _use_fixture(monkeypatch, FakeGitHub())
    args = _main_args()
    args[args.index(TAG)] = "v1.2.3"

    assert gate.main(args) == 1
    assert "is not a prerelease tag" in capsys.readouterr().err


def test_main_rejects_a_repository_that_is_not_owner_slash_name(monkeypatch, capsys):
    monkeypatch.setenv("GITHUB_REPOSITORY", "not a repository")
    monkeypatch.setenv("GH_TOKEN", "test-token")

    assert gate.main(_main_args()) == 1
    assert "GITHUB_REPOSITORY must be owner/repository" in capsys.readouterr().err


# Workflow wiring


def test_preflight_gates_publish_on_the_verified_native_matrix_run():
    text = (ROOT / ".github/workflows/release-preflight.yml").read_text(encoding="utf-8")
    assert "fetch-depth: 0" in text
    assert "id: native_matrix_gate" in text
    assert "if: ${{ always() && steps.meta.outcome == 'success' }}" in text
    assert "python3 .github/scripts/release_native_gate.py" in text
    assert "NATIVE_MATRIX_GATE: ${{ steps.native_matrix_gate.outcome }}" in text
    assert 'record native-matrix-gate "${NATIVE_MATRIX_GATE}"' in text
    assert "NATIVE_VERIFY_RUN_ID: ${{ steps.native_matrix_gate.outputs.native_verify_run_id }}" in text


def test_preflight_takes_the_prerelease_prefix_from_the_manifest():
    text = (ROOT / ".github/workflows/release-preflight.yml").read_text(encoding="utf-8")
    assert "release_native_verify.py prerelease-prefix --manifest release/publish-artifacts.toml" in text
    assert 'prerelease_tag="prerelease/' not in text
    gate_step = text.split("id: native_matrix_gate", 1)[1].split("\n      - ", 1)[0]
    assert "python3 -c" not in gate_step
    assert 'rev-parse --verify --quiet "refs/tags/${prerelease_tag}^{commit}"' in gate_step
    assert 'rev-parse --verify --quiet "refs/tags/${candidate_tag}^{commit}"' in gate_step


def test_verify_workflow_titles_each_run_with_its_tag_and_commit():
    text = (ROOT / ".github/workflows/release-native-verify.yml").read_text(encoding="utf-8")
    run_name = re.search(r"^run-name: (.+)$", text, re.MULTILINE).group(1)
    tag_expression, commit_expression = re.findall(r"\$\{\{[^}]*\}\}", run_name)

    assert tag_expression == "${{ inputs.tag || github.event.workflow_run.head_branch }}"
    assert commit_expression == "${{ inputs.commit || github.event.workflow_run.head_sha }}"
    assert run_name.replace(tag_expression, TAG).replace(commit_expression, VERIFIED) == common.verify_run_title(TAG, VERIFIED)


def test_verify_workflow_names_its_jobs_as_the_gate_expects():
    text = (ROOT / ".github/workflows/release-native-verify.yml").read_text(encoding="utf-8")
    assert f"name: {common.native_job('${{ matrix.target }}')}" in text
    assert f"name: {common.REPORT_JOB}" in text


@pytest.mark.parametrize("event, branch, trusted", [
    ("workflow_run", TAG, True),
    ("workflow_run", DEFAULT_BRANCH, False),
    ("workflow_run", "feature/x", False),
    ("workflow_dispatch", DEFAULT_BRANCH, True),
    ("workflow_dispatch", "feature/x", False),
])
def test_a_run_proves_a_prerelease_only_from_its_tag_or_default_branch(event, branch, trusted):
    candidate = {"path": VERIFY_PATH, "event": event, "head_branch": branch}
    assert gate.trusted_verify_run(candidate, TAG, DEFAULT_BRANCH) is trusted


def test_gate_takes_the_matrix_from_the_policy_module_not_the_verifier():
    tree = ast.parse(Path(gate.__file__).read_text(encoding="utf-8"))
    imported = {
        alias.name.split(".", 1)[0]
        for node in ast.walk(tree)
        if isinstance(node, ast.Import)
        for alias in node.names
    }
    imported.update(node.module for node in ast.walk(tree) if isinstance(node, ast.ImportFrom))
    assert "release_native_matrix" in imported
    assert "release_native_verify" not in imported
