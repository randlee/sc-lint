"""Pass only when a native verify run proved prerelease commit V and no build input changed from V to candidate C."""
from __future__ import annotations

import json
import subprocess
import sys
import tomllib

# Paths whose change can alter the released archives or wheels, or how they are built.
BUILD_INPUTS = ("crates/", "bindings/", "docs-bundle/", ".cargo/", ".github/actions/", "Cargo.", "rust-toolchain",
                "release/publish-artifacts.toml", ".github/workflows/prerelease-archive.yml", ".github/workflows/release.yml",
                *(f".github/scripts/{name}" for name in (
                    "build_release_target.sh", "release_artifacts.py", "release_manifest.py", "release_platforms.py",
                    "release_python.py", "release_registry.py", "release_stage.py", "npm_release.py",
                    "verify_release_arch.py", "wheel_contents.py", "worker_result.py")))


def out(*argv: str) -> str:
    return subprocess.run(argv, capture_output=True, text=True, check=True).stdout.strip()


def changed_build_inputs(paths: list[str]) -> list[str]:
    return [path for path in paths if path.startswith(BUILD_INPUTS)]


def run_failure(run: dict, jobs: list[dict], tag: str, commit: str, targets: list[str]) -> str | None:
    """Why a verify run does not prove tag at commit; None when every manifest native job succeeded."""
    if run["displayTitle"] != f"Release Native Verify {tag} {commit}":
        return "title does not bind the tag and commit"
    conclusions = {job["name"]: job["conclusion"] for job in jobs}
    failed = [f"{name} ({conclusions.get(name) or 'missing'})" for name in (f"Native {t}" for t in targets)
              if conclusions.get(name) != "success"]
    return f"native jobs did not succeed: {', '.join(failed)}" if failed else None


def gate(tag: str, candidate_ref: str) -> str:
    """Return the passing verdict for tag against the candidate, or raise ValueError naming why it fails."""
    verified, candidate = (out("git", "rev-parse", "--verify", f"{ref}^{{commit}}") for ref in (f"refs/tags/{tag}", candidate_ref))
    targets = [entry["target"] for entry in tomllib.loads(out("git", "show", f"{verified}:release/publish-artifacts.toml"))["release_targets"]]
    runs = json.loads(out("gh", "run", "list", "--workflow", "release-native-verify.yml", "--status", "success",
                          "--limit", "200", "--json", "databaseId,displayTitle"))
    reasons = {run["databaseId"]: run_failure(run, json.loads(out("gh", "run", "view", str(run["databaseId"]), "--json", "jobs"))["jobs"],
                                              tag, verified, targets) for run in runs if tag in run["displayTitle"]}
    proof = next((run_id for run_id, reason in reasons.items() if reason is None), None)
    if proof is None:
        raise ValueError(f"no verify run proves {tag} at {verified}; dispatch Release Native Verify with that tag and commit: {reasons}")
    if changed := changed_build_inputs(out("git", "diff", "--name-only", "--no-renames", verified, candidate).splitlines()):
        raise ValueError(f"build inputs changed since {tag} ({verified}); cut a new prerelease: {', '.join(changed)}")
    return f"run {proof} verified {tag} at {verified}; candidate {candidate} has the same build inputs"


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: release_native_gate.py <prerelease-tag> <candidate-ref>")
    try:
        print(gate(sys.argv[1], sys.argv[2]))
    except (subprocess.CalledProcessError, ValueError, KeyError) as error:
        sys.exit(f"release-native-gate: {error} {getattr(error, 'stderr', '') or ''}".strip())
