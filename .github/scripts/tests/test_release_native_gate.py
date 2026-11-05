"""Deterministic release native gate tests: fixed paths and fixed verify-run JSON, no GitHub client."""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_native_gate as gate

TAG = "prerelease/v1.2.3"
COMMIT = "a" * 40
TARGETS = ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"]


def verify_run(*, title=f"Release Native Verify {TAG} {COMMIT}", head_sha=COMMIT):
    return {"databaseId": 7, "displayTitle": title, "headSha": head_sha}


def jobs(**conclusions):
    named = {f"Native {target}": "success" for target in TARGETS}
    named.update({f"Native {target}": conclusion for target, conclusion in conclusions.items()})
    return [{"name": "plan", "conclusion": "success"}] + [
        {"name": name, "conclusion": conclusion} for name, conclusion in named.items() if conclusion is not None
    ]


@pytest.mark.parametrize("path", [
    "crates/sc-lint/src/main.rs",
    "docs-bundle/using-sc-lint.md",
    ".github/scripts/build_release_target.sh",
    "release/publish-artifacts.toml",
    "bindings/sc-lint-py/src/lib.rs",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/config.toml",
    ".github/actions/setup-python-release-build/action.yml",
    ".github/workflows/prerelease-archive.yml",
    ".github/workflows/release-native-verify.yml",
    ".github/scripts/release_native_verify.py",
    ".github/scripts/release_native_gate.py",
])
def test_build_input_change_fails(path):
    assert gate.changed_build_inputs(["docs/requirements.md", path]) == [path]


@pytest.mark.parametrize("path", ["docs/requirements.md", "CHANGELOG.md", "release/README.md", ".github/workflows/ci.yml"])
def test_non_build_change_passes(path):
    assert gate.changed_build_inputs([path]) == []


def test_run_with_every_native_job_successful_proves_the_tag_and_commit():
    assert gate.run_failure(verify_run(), jobs(), TAG, COMMIT, TARGETS) is None


def test_run_with_a_different_head_sha_fails():
    assert gate.run_failure(verify_run(head_sha="b" * 40), jobs(), TAG, COMMIT, TARGETS) == "run headSha does not match the verified commit"


@pytest.mark.parametrize("title", [
    f"Release Native Verify {TAG} {'b' * 40}",
    f"Release Native Verify {TAG}",
    f"Release Native Verify prerelease/v1.2.4 {COMMIT}",
])
def test_run_for_another_commit_or_tag_fails(title):
    assert gate.run_failure(verify_run(title=title), jobs(), TAG, COMMIT, TARGETS) == "title does not bind the tag and commit"


@pytest.mark.parametrize("conclusion, shown", [("skipped", "skipped"), ("failure", "failure"), (None, "missing")])
def test_unsuccessful_or_missing_native_job_fails(conclusion, shown):
    reason = gate.run_failure(verify_run(), jobs(**{"aarch64-apple-darwin": conclusion}), TAG, COMMIT, TARGETS)
    assert reason == f"native jobs did not succeed: Native aarch64-apple-darwin ({shown})"


def fixed_commands(monkeypatch, *, changed, runs, candidate="c" * 40):
    """Answer the gate's git and gh commands from fixed text."""
    import json
    manifest = "".join(f'[[release_targets]]\ntarget = "{target}"\n' for target in TARGETS)
    answers = {
        ("git", "rev-parse", "--verify", f"refs/tags/{TAG}^{{commit}}"): COMMIT,
        ("git", "rev-parse", "--verify", "candidate^{commit}"): candidate,
        ("git", "show", f"{COMMIT}:release/publish-artifacts.toml"): manifest,
        ("gh", "run", "list", "--workflow", "release-native-verify.yml", "--status", "success", "--limit", "200",
         "--json", "databaseId,displayTitle,headSha"): json.dumps(runs),
        ("gh", "run", "view", "7", "--json", "jobs"): json.dumps({"jobs": jobs()}),
        ("git", "diff", "--name-only", "--no-renames", COMMIT, candidate): "\n".join(changed),
    }
    monkeypatch.setattr(gate, "out", lambda *argv: answers[argv])


def test_gate_passes_a_verified_prerelease_with_only_docs_changes(monkeypatch):
    fixed_commands(monkeypatch, changed=["docs/requirements.md"], runs=[verify_run()])
    assert gate.gate(TAG, "candidate").startswith(f"run 7 verified {TAG} at {COMMIT}")


def test_gate_passes_when_the_candidate_is_the_verified_commit(monkeypatch):
    fixed_commands(monkeypatch, changed=[], runs=[verify_run()], candidate=COMMIT)
    assert gate.gate(TAG, "candidate").endswith(f"candidate {COMMIT} has the same build inputs")


def test_gate_fails_when_a_build_input_changed_after_the_verified_prerelease(monkeypatch):
    fixed_commands(monkeypatch, changed=["docs/requirements.md", "crates/sc-lint/src/main.rs"], runs=[verify_run()])
    with pytest.raises(ValueError, match="build inputs changed since .*: crates/sc-lint/src/main.rs$"):
        gate.gate(TAG, "candidate")


def test_gate_fails_without_a_run_proving_the_tag_at_its_commit(monkeypatch):
    fixed_commands(monkeypatch, changed=[], runs=[verify_run(title=f"Release Native Verify {TAG} {'b' * 40}")])
    with pytest.raises(ValueError, match=f"no verify run proves {TAG} at {COMMIT}"):
        gate.gate(TAG, "candidate")
