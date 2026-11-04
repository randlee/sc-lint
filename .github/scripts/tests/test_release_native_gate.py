"""Deterministic fixture tests for the native-matrix publish gate."""
import subprocess
import sys
import tomllib
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_native_gate as gate
import release_native_verify as native

ROOT = Path(__file__).resolve().parents[3]
TAG = "prerelease/v1.2.3"
VERIFIED = "a" * 40
CANDIDATE = "c" * 40
TITLE = "Release Native Verify prerelease/v1.2.3"


def run(run_id, title, *, conclusion="success", status="completed"):
    return {"id": run_id, "display_title": title, "status": status, "conclusion": conclusion}


def page(*runs, total=None):
    return {"total_count": len(runs) if total is None else total, "workflow_runs": list(runs)}


class FakeGitHub:
    """Serves canned workflow-run pages and records every endpoint it was asked for."""

    def __init__(self, pages=None, *, release_published=True):
        self.pages = pages or {}
        self.release_published = release_published
        self.calls = []

    def json(self, endpoint):
        self.calls.append(endpoint)
        if endpoint.startswith("releases/tags/"):
            if not self.release_published:
                raise native.GitHubError(endpoint, 404)
            return {"assets": []}
        prefix = f"actions/workflows/{gate.VERIFY_WORKFLOW}/runs?"
        assert endpoint.startswith(prefix), endpoint
        query = parse_qs(urlsplit(endpoint).query)
        assert query["status"] == ["success"]
        return self.pages.get(int(query["page"][0]), page())


def diff_of(*paths):
    return gate.classify(list(paths))


# Run lookup


def test_newest_successful_run_with_matching_title_is_selected():
    client = FakeGitHub({1: page(
        run(9, TITLE, conclusion="failure"),
        run(8, "Release Native Verify prerelease/v1.2.4"),
        run(7, TITLE),
        run(5, TITLE),
    )})
    assert gate.find_verified_run(client, TAG) == 7


def test_run_for_another_tag_is_ignored():
    client = FakeGitHub({1: page(run(12, "Release Native Verify prerelease/v1.2.4"))})
    with pytest.raises(gate.GateError, match="no successful Release Native Verify run exists for prerelease/v1.2.3"):
        gate.find_verified_run(client, TAG)


def test_failed_or_unfinished_matching_runs_fail_closed():
    client = FakeGitHub({1: page(
        run(3, TITLE, conclusion="failure"),
        run(2, TITLE, status="in_progress", conclusion=None),
    )})
    with pytest.raises(gate.GateError, match="no successful"):
        gate.find_verified_run(client, TAG)


def test_no_verify_runs_fails_closed():
    with pytest.raises(gate.GateError, match="run the native matrix on that prerelease"):
        gate.find_verified_run(FakeGitHub(), TAG)


def test_pagination_finds_matching_run_on_second_page():
    other_runs = [run(1000 + index, "Release Native Verify prerelease/v1.2.4") for index in range(gate.PAGE_SIZE)]
    client = FakeGitHub({
        1: page(*other_runs, total=gate.PAGE_SIZE + 1),
        2: page(run(7, TITLE), total=gate.PAGE_SIZE + 1),
    })
    assert gate.find_verified_run(client, TAG) == 7
    assert [parse_qs(urlsplit(call).query)["page"][0] for call in client.calls] == ["1", "2"]


def test_search_stops_at_the_page_limit():
    full_pages = {number: page(*[run(number * 1000 + index, "other") for index in range(gate.PAGE_SIZE)],
                               total=10_000)
                  for number in range(1, gate.MAX_PAGES + 2)}
    client = FakeGitHub(full_pages)
    with pytest.raises(gate.GateError):
        gate.find_verified_run(client, TAG)
    assert len(client.calls) == gate.MAX_PAGES


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
    [".github/scripts/release_native_verify.py", ".github/scripts/release_native_gate.py",
     ".github/scripts/tests/test_release_native_gate.py"],
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
    (["release/publish-artifacts.toml"], "release/publish-artifacts.toml"),
    ([".github/scripts/build_release_target.sh"], ".github/scripts/build_release_target.sh"),
])
def test_artifact_input_change_fails_naming_the_path(changed, offending):
    with pytest.raises(gate.GateError) as error:
        gate.evaluate(TAG, VERIFIED, CANDIDATE, is_ancestor=True, diff=diff_of(*changed))
    assert offending in str(error.value)
    assert "cut a new prerelease and rerun Release Native Verify" in str(error.value)


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
    manifest = tomllib.loads((ROOT / "release/publish-artifacts.toml").read_text(encoding="utf-8"))
    declared = [crate["cargo_toml"] for crate in manifest["crates"]]
    declared += [manifest["python_packages"][0]["manifest"]]
    declared += [distribution["source"] for distribution in manifest["python_distributions"]]
    declared += [path["source"] for binary in manifest["release_binaries"] for path in binary.get("bundled_paths", [])]
    declared += ["Cargo.toml", "Cargo.lock", manifest["project"]["workspace_toml"], "rust-toolchain.toml"]
    uncovered = [path for path in declared if not gate.is_artifact_input(path)]
    assert uncovered == []


# Real git history: ancestry and the changed-path set


def git(repo, *args):
    return subprocess.run(
        ["git", "-c", "user.name=test", "-c", "user.email=test@example.com", *args],
        cwd=repo, capture_output=True, text=True, check=True,
    ).stdout.strip()


def commit_file(repo, path, text, message):
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")
    git(repo, "add", path)
    git(repo, "commit", "-q", "-m", message)
    return git(repo, "rev-parse", "HEAD")


def test_git_helpers_report_ancestry_and_changed_paths(tmp_path, monkeypatch):
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    verified = commit_file(repo, "crates/sc-lint/src/lib.rs", "one\n", "verified")
    commit_file(repo, "docs/notes.md", "notes\n", "docs only")
    candidate = commit_file(repo, "CHANGELOG.md", "entry\n", "changelog")
    monkeypatch.chdir(repo)

    assert gate.git_is_ancestor(verified, candidate) is True
    assert gate.git_is_ancestor(candidate, verified) is False
    assert gate.git_changed_paths(verified, candidate) == ["CHANGELOG.md", "docs/notes.md"]


def test_git_changed_paths_names_a_space_in_the_path(tmp_path, monkeypatch):
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    base = commit_file(repo, "README.md", "x\n", "base")
    head = commit_file(repo, "crates/with space.rs", "y\n", "spaced path")
    monkeypatch.chdir(repo)

    assert gate.git_changed_paths(base, head) == ["crates/with space.rs"]


# main(): output contract and fail-closed exits


def _main_args(verified=VERIFIED, candidate=CANDIDATE, output=None):
    args = ["--tag", TAG, "--prerelease-commit", verified, "--candidate-commit", candidate]
    if output is not None:
        args += ["--output", str(output)]
    return args


def _use_fixture(monkeypatch, client, *, is_ancestor=True, changed=()):
    monkeypatch.setattr(gate, "GitHub", lambda repository, token: client)
    monkeypatch.setattr(gate, "git_is_ancestor", lambda ancestor, descendant: is_ancestor)
    monkeypatch.setattr(gate, "git_changed_paths", lambda base, head: list(changed))
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
    out = capsys.readouterr().out
    assert "Release Native Verify run 7" in out
    assert "artifact inputs: none" in out
    assert "no rerun needed: 2 paths" in out


def test_main_fails_when_an_artifact_input_changed_and_writes_nothing(monkeypatch, tmp_path, capsys):
    client = FakeGitHub({1: page(run(7, TITLE))})
    _use_fixture(monkeypatch, client, changed=["crates/sc-lint/src/lib.rs"])
    output = tmp_path / "github_output"

    status = gate.main(_main_args(output=output))

    assert status == 1
    assert not output.exists()
    assert "release-native-gate: artifact inputs changed" in capsys.readouterr().err


def test_main_fails_closed_without_a_verify_run(monkeypatch, tmp_path, capsys):
    _use_fixture(monkeypatch, FakeGitHub())
    output = tmp_path / "github_output"

    status = gate.main(_main_args(output=output))

    assert status == 1
    assert not output.exists()
    assert "release-native-gate: no successful Release Native Verify run" in capsys.readouterr().err


def test_main_fails_when_the_verified_commit_is_not_an_ancestor(monkeypatch, capsys):
    _use_fixture(monkeypatch, FakeGitHub({1: page(run(7, TITLE))}), is_ancestor=False)

    assert gate.main(_main_args()) == 1
    assert "not an ancestor of the release candidate" in capsys.readouterr().err


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


def test_verify_workflow_titles_each_run_with_its_tag():
    text = (ROOT / ".github/workflows/release-native-verify.yml").read_text(encoding="utf-8")
    assert "run-name: Release Native Verify ${{ inputs.tag || github.event.workflow_run.head_branch }}" in text
    assert gate.verify_run_title(TAG) == TITLE
