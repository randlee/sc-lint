"""Gate a publish on a native matrix run that proved a verified prerelease.

A Release Native Verify run gates the publish only when it ran for the prerelease tag at
its commit V: its title names both, every native job and the report job concluded success.
V must be an ancestor of the release candidate commit C, and no artifact input may differ
between V and C. Changes outside the artifact inputs (docs, site, CHANGELOG, publish and
channel workflows, verifier scripts and tests, and the manifest's publish-only sections)
never require a rerun. Fail closed otherwise and name the offending paths.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from release_native_common import (
    MANIFEST_PATH, REPORT_JOB, GitHub, GitHubError, is_full_sha, native_job, native_matrix, quote_tag,
    verify_run_title,
)

VERIFY_WORKFLOW = "release-native-verify.yml"
VERIFY_WORKFLOW_PATH = f".github/workflows/{VERIFY_WORKFLOW}"
PAGE_SIZE = 100
MAX_PAGES = 10

# Paths whose content changes the released archives or wheels, or how they are built. The
# gate's tests derive the manifest's declared sources and the scripts and actions that
# prerelease-archive.yml runs (with their local imports) and fail on any path not covered here.
ARTIFACT_INPUT_DIRS = frozenset({"crates", "bindings", "docs-bundle"})
# .cargo/ holds build configuration (targets, linkers, rustflags) that changes the binaries.
ARTIFACT_INPUT_PREFIXES = (".cargo/", ".github/actions/")
ARTIFACT_INPUT_FILES = frozenset({
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".github/scripts/build_release_target.sh",
    ".github/scripts/release_artifacts.py",
    ".github/scripts/release_manifest.py",
    ".github/scripts/release_platforms.py",
    ".github/scripts/release_python.py",
    ".github/scripts/release_registry.py",
    ".github/scripts/npm_release.py",
    ".github/scripts/release_stage.py",
    ".github/scripts/verify_release_arch.py",
    ".github/scripts/worker_result.py",
    ".github/scripts/wheel_contents.py",
    ".github/workflows/prerelease-archive.yml",
})
# The manifest sections that describe the archives and wheels. Channel, prerelease and
# python-package settings only drive publication, so editing them never forces a rerun.
BUILD_MANIFEST_SECTIONS = (
    "schema_version", "project", "release_targets", "crates", "release_binaries", "python_packages",
    "python_distributions",
)


class GateError(ValueError):
    pass


@dataclass(frozen=True)
class Diff:
    """The V..C change set split by whether it can change released artifacts."""

    artifact_inputs: tuple[str, ...] = ()
    other: tuple[str, ...] = field(default=())


def is_artifact_input(path: str) -> bool:
    return (
        path in ARTIFACT_INPUT_FILES
        or path.startswith(ARTIFACT_INPUT_PREFIXES)
        or path.split("/", 1)[0] in ARTIFACT_INPUT_DIRS
    )


def classify(paths: list[str], *, manifest_build_changed: bool) -> Diff:
    def artifact(path: str) -> bool:
        if path == MANIFEST_PATH:
            return manifest_build_changed
        return is_artifact_input(path)

    inputs = tuple(sorted(path for path in paths if artifact(path)))
    other = tuple(sorted(path for path in paths if not artifact(path)))
    return Diff(artifact_inputs=inputs, other=other)


def manifest_build_view(text: str | None) -> dict:
    data = tomllib.loads(text) if text is not None else {}
    return {key: data.get(key) for key in BUILD_MANIFEST_SECTIONS}


def _verify_runs(client: GitHub):
    page = 1
    while page <= MAX_PAGES:
        data = client.json(f"actions/workflows/{VERIFY_WORKFLOW}/runs?status=success&per_page={PAGE_SIZE}&page={page}")
        yield from data.get("workflow_runs", [])
        if page * PAGE_SIZE >= data.get("total_count", 0):
            return
        page += 1


def _jobs_by_name(client: GitHub, run_id: int) -> dict[str, str | None]:
    jobs: dict[str, str | None] = {}
    page = 1
    while True:
        data = client.json(f"actions/runs/{run_id}/jobs?filter=latest&per_page={PAGE_SIZE}&page={page}")
        for job in data.get("jobs", []):
            if job["name"] in jobs:
                raise GateError(f"run {run_id}: duplicate job name {job['name']!r}")
            jobs[job["name"]] = job.get("conclusion")
        if page * PAGE_SIZE >= data.get("total_count", 0):
            return jobs
        page += 1


def unproven_reason(jobs: dict[str, str | None], targets: list[str]) -> str | None:
    """Name the required jobs that did not succeed; None when every native job and the report passed."""
    required = [native_job(target) for target in targets] + [REPORT_JOB]
    failing = [f"{name} ({jobs.get(name) or 'absent'})" for name in required if jobs.get(name) != "success"]
    return f"jobs did not all succeed: {', '.join(failing)}" if failing else None


def trusted_verify_run(run: dict, default_branch: str) -> bool:
    """Only the verify workflow file can prove a publish: run by a workflow_run, or dispatched from the default branch."""
    if run.get("path") != VERIFY_WORKFLOW_PATH:
        return False
    if run.get("event") == "workflow_run":
        return True
    return run.get("event") == "workflow_dispatch" and run.get("head_branch") == default_branch


def find_verified_run(client: GitHub, tag: str, commit: str, targets: list[str]) -> int:
    """Return the id of the newest successful verify run that proved the native matrix for tag at commit.

    The title binds the run to the tag and commit, so a run for an earlier build of a re-cut
    tag cannot match. A run concluding success still proves nothing when its native jobs were
    skipped, so the required jobs must each have succeeded.
    """
    title = verify_run_title(tag, commit)
    default_branch = client.default_branch()
    rejected = []
    for run in _verify_runs(client):
        if not trusted_verify_run(run, default_branch):
            continue
        if run.get("display_title") != title or run.get("status") != "completed" or run.get("conclusion") != "success":
            continue
        reason = unproven_reason(_jobs_by_name(client, int(run["id"])), targets)
        if reason is None:
            return int(run["id"])
        rejected.append(f"run {run['id']}: {reason}")
    detail = f" (rejected: {'; '.join(rejected)})" if rejected else ""
    raise GateError(
        f"no successful Release Native Verify run exists for {tag} at {commit}; "
        f"dispatch Release Native Verify with tag {tag} and commit {commit}{detail}"
    )


def check_prerelease_release(client: GitHub, tag: str) -> None:
    """The prerelease must be published, not only tagged."""
    try:
        client.json(f"releases/tags/{quote_tag(tag)}")
    except GitHubError as error:
        raise GateError(f"{tag} has no published prerelease (HTTP {error.status})") from error


def require_full_shas(tag: str, *commits: str) -> None:
    if not all(is_full_sha(commit) for commit in commits):
        raise GateError(f"{tag}: commit identities must be full 40-character SHAs")


def evaluate(tag: str, verified_commit: str, candidate_commit: str, *, is_ancestor: bool, diff: Diff) -> None:
    """Fail closed unless V is an ancestor of C and no artifact input changed between them."""
    require_full_shas(tag, verified_commit, candidate_commit)
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
    """Every path that differs, with renames split into a delete and an add so both sides are classified."""
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "-z", base, head],
        capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise GateError(f"git diff failed: {result.stderr.strip()}")
    return [path for path in result.stdout.split("\0") if path]


def git_show(commit: str, path: str) -> str | None:
    """The file's text at a commit, or None when it does not exist there."""
    result = subprocess.run(["git", "show", f"{commit}:{path}"], capture_output=True, text=True, check=False)
    return result.stdout if result.returncode == 0 else None


def manifest_build_changed(base: str, head: str) -> bool:
    """Whether a build section of the manifest differs between base and head.

    Trade-off: each section is compared whole, so a field inside a build section that does not
    reach the archives still forces a rerun. The gate can over-trigger a rerun but never miss one.
    """
    return manifest_build_view(git_show(base, MANIFEST_PATH)) != manifest_build_view(git_show(head, MANIFEST_PATH))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True, help="prerelease tag, for example prerelease/v1.2.3")
    parser.add_argument("--prerelease-commit", required=True, help="commit V the prerelease tag points at")
    parser.add_argument("--candidate-commit", required=True, help="commit C of release-candidate-vX.Y.Z")
    parser.add_argument("--output", type=Path, help="GITHUB_OUTPUT file to receive the verified run and commit")
    args = parser.parse_args(argv)
    try:
        require_full_shas(args.tag, args.prerelease_commit, args.candidate_commit)
        client = GitHub(os.environ.get("GITHUB_REPOSITORY", ""), os.environ.get("GH_TOKEN", ""))
        check_prerelease_release(client, args.tag)
        targets = [row["target"] for row in native_matrix(client.manifest(args.tag))["include"]]
        run_id = find_verified_run(client, args.tag, args.prerelease_commit, targets)
        is_ancestor = git_is_ancestor(args.prerelease_commit, args.candidate_commit)
        paths = git_changed_paths(args.prerelease_commit, args.candidate_commit) if is_ancestor else []
        diff = classify(paths, manifest_build_changed=MANIFEST_PATH in paths and manifest_build_changed(
            args.prerelease_commit, args.candidate_commit,
        ))
        evaluate(args.tag, args.prerelease_commit, args.candidate_commit, is_ancestor=is_ancestor, diff=diff)
    except (GateError, GitHubError, ValueError) as error:
        print(f"release-native-gate: {error}", file=sys.stderr)
        return 1
    print(f"Native matrix verified for {args.tag}: Release Native Verify run {run_id} on commit {args.prerelease_commit}")
    print(f"Artifact inputs changed since the verified commit: {', '.join(diff.artifact_inputs) or 'none'}")
    print(f"Paths changed since the verified commit that need no rerun: {len(diff.other)}")
    if args.output:
        with args.output.open("a", encoding="utf-8") as handle:
            handle.write(f"native_verify_run_id={run_id}\n")
            handle.write(f"native_verified_commit={args.prerelease_commit}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
