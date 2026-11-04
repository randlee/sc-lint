"""Gate a publish on a successful native matrix run for a verified prerelease.

A Release Native Verify run must have succeeded for the candidate's prerelease tag.
The verified prerelease commit V must be an ancestor of the release candidate commit C,
and no artifact input may differ between V and C. Changes outside the artifact inputs
(docs, site, CHANGELOG, publish and channel workflows, verifier scripts and tests) never
require a rerun. Fail closed otherwise and name the offending paths.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

from release_native_verify import GitHub, GitHubError

VERIFY_WORKFLOW = "release-native-verify.yml"
PAGE_SIZE = 100
MAX_PAGES = 10

# Paths whose content changes the released archives or wheels, or how they are built.
# Keep in step with release/publish-artifacts.toml; tests check manifest coverage.
ARTIFACT_INPUT_DIRS = frozenset({"crates", "bindings", "docs-bundle"})
ARTIFACT_INPUT_FILES = frozenset({
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "release/publish-artifacts.toml",
    ".github/scripts/build_release_target.sh",
    ".github/scripts/release_artifacts.py",
    ".github/scripts/release_python.py",
    ".github/workflows/prerelease-archive.yml",
})


class GateError(ValueError):
    pass


@dataclass(frozen=True)
class Diff:
    """The V..C change set split by whether it can change released artifacts."""

    artifact_inputs: tuple[str, ...] = ()
    other: tuple[str, ...] = field(default=())


def is_artifact_input(path: str) -> bool:
    return path in ARTIFACT_INPUT_FILES or path.split("/", 1)[0] in ARTIFACT_INPUT_DIRS


def classify(paths: list[str]) -> Diff:
    inputs = tuple(sorted(path for path in paths if is_artifact_input(path)))
    other = tuple(sorted(path for path in paths if not is_artifact_input(path)))
    return Diff(artifact_inputs=inputs, other=other)


def verify_run_title(tag: str) -> str:
    # release-native-verify.yml sets run-name to "Release Native Verify <tag>".
    return f"Release Native Verify {tag}"


def _verify_runs(client: GitHub):
    page = 1
    while page <= MAX_PAGES:
        data = client.json(f"actions/workflows/{VERIFY_WORKFLOW}/runs?status=success&per_page={PAGE_SIZE}&page={page}")
        yield from data.get("workflow_runs", [])
        if page * PAGE_SIZE >= data.get("total_count", 0):
            return
        page += 1


def find_verified_run(client: GitHub, tag: str) -> int:
    """Return the id of the newest successful verify run for the prerelease tag."""
    title = verify_run_title(tag)
    for run in _verify_runs(client):
        if run.get("display_title") == title and run.get("status") == "completed" and run.get("conclusion") == "success":
            return int(run["id"])
    raise GateError(f"no successful Release Native Verify run exists for {tag}; run the native matrix on that prerelease")


def check_prerelease_release(client: GitHub, tag: str) -> None:
    """The prerelease must be published, not only tagged."""
    try:
        client.json(f"releases/tags/{tag.replace('/', '%2F')}")
    except GitHubError as error:
        raise GateError(f"{tag} has no published prerelease (HTTP {error.status})") from error


def evaluate(tag: str, verified_commit: str, candidate_commit: str, *, is_ancestor: bool, diff: Diff) -> None:
    """Fail closed unless V is an ancestor of C and no artifact input changed between them."""
    if not re.fullmatch(r"[0-9a-f]{40}", verified_commit) or not re.fullmatch(r"[0-9a-f]{40}", candidate_commit):
        raise GateError(f"{tag}: commit identities must be full 40-character SHAs")
    if not is_ancestor:
        raise GateError(
            f"{tag} is built from {verified_commit}, which is not an ancestor of the release candidate "
            f"{candidate_commit}; cut a new prerelease from the candidate line"
        )
    if diff.artifact_inputs:
        raise GateError(
            f"artifact inputs changed between verified commit {verified_commit[:12]} and candidate "
            f"{candidate_commit[:12]}: {', '.join(diff.artifact_inputs)}; "
            f"cut a new prerelease and rerun Release Native Verify"
        )


def git_is_ancestor(ancestor: str, descendant: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        capture_output=True, text=True, check=False,
    )
    if result.returncode in (0, 1):
        return result.returncode == 0
    raise GateError(f"git merge-base failed: {result.stderr.strip()}")


def git_changed_paths(base: str, head: str) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "-z", base, head],
        capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise GateError(f"git diff failed: {result.stderr.strip()}")
    return [path for path in result.stdout.split("\0") if path]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True, help="prerelease tag, for example prerelease/v1.2.3")
    parser.add_argument("--prerelease-commit", required=True, help="commit V the prerelease tag points at")
    parser.add_argument("--candidate-commit", required=True, help="commit C of release-candidate-vX.Y.Z")
    parser.add_argument("--output", type=Path, help="GITHUB_OUTPUT file to receive the verified run and commit")
    args = parser.parse_args(argv)
    try:
        client = GitHub(os.environ.get("GITHUB_REPOSITORY", ""), os.environ.get("GH_TOKEN", ""))
        check_prerelease_release(client, args.tag)
        run_id = find_verified_run(client, args.tag)
        if not re.fullmatch(r"[0-9a-f]{40}", args.prerelease_commit) or not re.fullmatch(r"[0-9a-f]{40}", args.candidate_commit):
            raise GateError(f"{args.tag}: commit identities must be full 40-character SHAs")
        is_ancestor = git_is_ancestor(args.prerelease_commit, args.candidate_commit)
        diff = classify(git_changed_paths(args.prerelease_commit, args.candidate_commit)) if is_ancestor else Diff()
        evaluate(args.tag, args.prerelease_commit, args.candidate_commit, is_ancestor=is_ancestor, diff=diff)
    except (GateError, GitHubError, ValueError) as error:
        print(f"release-native-gate: {error}", file=sys.stderr)
        return 1
    print(f"Native matrix verified for {args.tag}: Release Native Verify run {run_id} on commit {args.prerelease_commit}")
    print(f"Changed since verified commit, artifact inputs: {', '.join(diff.artifact_inputs) or 'none'}")
    print(f"Changed since verified commit, no rerun needed: {len(diff.other)} paths")
    if args.output:
        with args.output.open("a", encoding="utf-8") as handle:
            handle.write(f"native_verify_run_id={run_id}\n")
            handle.write(f"native_verified_commit={args.prerelease_commit}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
