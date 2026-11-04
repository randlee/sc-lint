"""Execute published archives and wheels on their manifest-selected native hosts."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import platform
import re
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from pathlib import Path, PurePosixPath

from verify_release_arch import EXPECTED, machine, verify as verify_wheel_arch
from release_manifest import release_archive_name, release_archive_pattern
from release_native_common import (
    BUILD_WORKFLOWS, MANIFEST_PATH, PRERELEASE_KIND, PRERELEASE_WORKFLOW, RELEASE_WORKFLOW, RESULT_SCHEMA_VERSION,
    GitHub, GitHubError, build_workflow_for, is_full_sha, paged, prerelease_prefix, quote_tag, release_version,
    tag_commit, tag_kind,
)
from release_native_matrix import WIN_ARM64_TARGET, native_host, native_matrix, wheel_specs
from release_python import wheel_asset_pattern


def tag_from_run(client: GitHub, manifest: dict, run_id: str, *, prerelease: bool = False) -> str:
    if not run_id.isdecimal():
        raise ValueError("automatic verification requires the triggering Release run id")
    targets = manifest.get("release_targets", [])
    if not targets:
        raise ValueError("release manifest declares no archive artifact to identify the triggering tag")
    target = targets[0]
    matches = []
    page = 1
    while True:
        data = client.json(f"actions/runs/{run_id}/artifacts?per_page=100&page={page}")
        matches.extend(entry for entry in data["artifacts"] if entry["name"] == target["target"] and not entry.get("expired"))
        if page * 100 >= data["total_count"]:
            break
        page += 1
    if len(matches) != 1:
        raise ValueError(f"Release run {run_id}: expected one available {target['target']} archive artifact; found {len(matches)}")
    payload = client.get(f"actions/artifacts/{matches[0]['id']}/zip", binary=True)
    pattern = re.compile(release_archive_pattern(manifest["project"], target, r"([0-9]+\.[0-9]+\.[0-9]+)"))
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        versions = [match[1] for name in archive.namelist() if (match := pattern.fullmatch(PurePosixPath(name).name))]
    if len(versions) != 1:
        raise ValueError(f"Release run {run_id}: archive artifact must identify exactly one release version")
    prefix = prerelease_prefix(manifest) if prerelease else "v"
    return prefix + versions[0]


def run_release_conclusion(client: GitHub, run_id: str, workflow: str) -> str | None:
    """Return an established release job conclusion, or explain a non-published skip."""
    if not run_id.isdecimal():
        raise ValueError("automatic verification requires the triggering run id")
    jobs = []
    page = 1
    while True:
        data = client.json(f"actions/runs/{run_id}/jobs?filter=latest&per_page=100&page={page}")
        jobs.extend(job for job in data["jobs"] if job["name"] == "release")
        if page * 100 >= data["total_count"]:
            break
        page += 1
    if not jobs:
        print(f"Skipping native verification: {workflow} run {run_id} has no release job.", flush=True)
        return None
    if len(jobs) != 1:
        raise ValueError(f"{workflow} run {run_id}: cannot establish release status; expected one release job, found {len(jobs)}")
    conclusion = jobs[0].get("conclusion")
    if conclusion == "skipped":
        print(f"Skipping native verification: {workflow} run {run_id} did not run its release job (conclusion: skipped).", flush=True)
        return None
    if conclusion not in {"success", "failure", "cancelled"}:
        raise ValueError(f"{workflow} run {run_id}: cannot establish release status; release job conclusion is {conclusion!r}")
    return conclusion


def require_built_commit(client: GitHub, tag: str, commit: str, workflow: str) -> None:
    """Fail unless a successful run of the workflow that builds the tag's kind published a release at commit.

    A tag re-cut to a commit whose build failed, or was never run, names assets that no
    successful build at that commit produced, so a dispatched verification must not bind to it.
    """
    runs = paged(client, f"actions/workflows/{BUILD_WORKFLOWS[workflow]}/runs?head_sha={commit}&status=success", "workflow_runs")
    for run in runs:
        if workflow == PRERELEASE_WORKFLOW and run.get("head_branch") != tag:
            continue
        if run_release_conclusion(client, str(run["id"]), workflow) == "success":
            return
    raise ValueError(
        f"{commit} has no successful {workflow} run that published its release; "
        f"the tag must name a commit whose build succeeded"
    )


def require_source_branch(client: GitHub, run_id: str, tag: str) -> None:
    """A prerelease run is triggered by its tag push, so its branch must be that tag."""
    head_branch = client.json(f"actions/runs/{run_id}")["head_branch"]
    if head_branch != tag:
        raise ValueError(f"prerelease run {run_id} was triggered from {head_branch!r}, not its tag {tag}")


def plan_commit(client: GitHub, tag: str, supplied: str, *, dispatch: bool, manifest: dict) -> str:
    """Return the commit a verification is bound to.

    A prerelease and any dispatched verification bind to the commit the tag names, which must be
    the commit the run was given, and a dispatched one also needs a successful build of the
    tag's kind at that commit. A Release run keeps its tag's commit even when it differs from the
    run's commit: release.yml reuses an existing, verified tag when it retries a release from main.
    """
    commit = tag_commit(client, tag)
    if dispatch or tag_kind(tag, manifest) == PRERELEASE_KIND:
        if supplied != commit:
            raise ValueError(f"{tag} names commit {commit}, not the commit {supplied} the run was given")
    if dispatch:
        require_built_commit(client, tag, commit, build_workflow_for(tag, manifest))
    return commit


def select_release(client: GitHub, manifest: dict, tag: str, run_id: str, workflow: str) -> tuple[str, str] | None:
    if tag:
        return tag, "success"
    conclusion = run_release_conclusion(client, run_id, workflow)
    if conclusion is None:
        return None
    tag = tag_from_run(client, manifest, run_id, prerelease=workflow == PRERELEASE_WORKFLOW)
    if workflow == PRERELEASE_WORKFLOW:
        require_source_branch(client, run_id, tag)
    return tag, conclusion


def plan_release(client: GitHub, manifest: dict, tag: str, conclusion: str) -> dict | None:
    """Build the matrix from the manifest at the resolved tag commit."""
    release_version(tag, manifest)
    try:
        client.json(f"releases/tags/{quote_tag(tag)}")
    except GitHubError as error:
        if error.status != 404 or conclusion == "success":
            raise
        print(f"Skipping native verification: {tag} has no published release (HTTP 404).", flush=True)
        return None
    return native_matrix(manifest)


def wheel_python_version(run, python: str) -> tuple[int, int]:
    text = run([python, "-c", "import sys; print('%d.%d' % sys.version_info[:2])"])
    major, minor = text.split(".")
    return int(major), int(minor)


def assert_native(target: str) -> None:
    expected = native_host(target)
    aliases = {"amd64": "x86_64", "x86_64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}
    actual = platform.system(), aliases.get(platform.machine().lower(), platform.machine())
    print(f"Native host: {actual}; target: {target}; Python: {sys.version}", flush=True)
    if actual != expected:
        raise ValueError(f"{target}: native runner required; expected {expected}, got {actual}")


def assert_wheel_python(target: str, python_version: tuple[int, int]) -> None:
    if python_version < (3, 9):
        raise ValueError("wheel verification requires Python 3.9 or newer")
    if target == WIN_ARM64_TARGET and python_version < (3, 11):
        raise ValueError("wheel verification on win_arm64 requires Python 3.11 or newer")


def required_assets(manifest: dict, target: str, tag: str, assets: list[dict]) -> tuple[str, list[str]]:
    version = release_version(tag, manifest)
    entries = [entry for entry in manifest["release_targets"] if entry["target"] == target]
    if len(entries) != 1:
        raise ValueError(f"{target}: expected exactly one target in the tag's release manifest")
    archive = release_archive_name(manifest["project"], entries[0], version)
    names = [entry["name"] for entry in assets]
    for required in (archive, "checksums.txt"):
        if names.count(required) != 1:
            raise ValueError(f"{tag} {target}: required release asset {required!r} missing or duplicated")
    wheels = []
    specs = wheel_specs(manifest, target)
    if not specs:
        raise ValueError(f"{target}: manifest declares no wheel to verify")
    for package, wheel_platform in specs:
        pattern = wheel_asset_pattern(package, wheel_platform, version)
        matching = [name for name in names if re.fullmatch(pattern, name)]
        if len(matching) != 1:
            raise ValueError(f"{tag} {target}: expected one {package} wheel for {wheel_platform}; found {matching}")
        wheels.append(matching[0])
    return archive, wheels


def checksum_entries(payload: bytes) -> dict[str, str]:
    entries = {}
    for line in payload.decode("utf-8").splitlines():
        if not line.strip():
            continue
        digest, name = line.split(maxsplit=1)
        name = name.lstrip(" *")
        if not re.fullmatch(r"[0-9a-fA-F]{64}", digest) or name in entries:
            raise ValueError(f"invalid or duplicate checksum entry: {line!r}")
        entries[name] = digest.lower()
    return entries


def extract_archive(path: Path, destination: Path) -> None:
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            for entry in archive.infolist():
                name = PurePosixPath(entry.filename.replace("\\", "/"))
                if name.is_absolute() or ".." in name.parts or (entry.external_attr >> 16) & 0o170000 == 0o120000:
                    raise ValueError(f"unsafe archive entry: {entry.filename}")
            archive.extractall(destination)
    else:
        with tarfile.open(path, "r:gz") as archive:
            archive.extractall(destination, filter="data")


def run_command(argv: list[str]) -> str:
    child_env = {key: value for key, value in os.environ.items() if key.upper() not in {"GH_TOKEN", "GITHUB_TOKEN"}}
    result = subprocess.run(argv, capture_output=True, text=True, env=child_env)
    if result.returncode:
        raise ValueError(f"command failed ({result.returncode}): {argv!r}\n{result.stdout}\n{result.stderr}")
    if result.stdout.strip():
        print(result.stdout.strip(), flush=True)
    return result.stdout.strip()


def assert_version(run, argv: list[str], binary: str, version: str) -> None:
    actual = run([*argv, "--version"])
    expected = f"{binary} {version}"
    if actual != expected:
        raise ValueError(f"{binary}: version mismatch; expected {expected!r}, got {actual!r}")


def record(checks: list[dict] | None, name: str, detail: str) -> None:
    if checks is not None:
        checks.append({"name": name, "status": "pass", "detail": detail})


def write_result(path: Path, *, tag: str, target: str, checks: list[dict], error: str) -> None:
    """Write the per-target JSON result the native report renders; failures are recorded too."""
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {"schema_version": RESULT_SCHEMA_VERSION, "tag": tag, "target": target, "checks": checks, "error": error}
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def verify_release(
    client: GitHub, tag: str, commit: str, target: str, destination: Path, *, wheel_python: str = sys.executable, run=run_command,
    checks: list[dict] | None = None,
) -> None:
    manifest = client.manifest(commit)
    version = release_version(tag, manifest)
    assert_native(target)
    record(checks, "native host", f"{target} on {native_host(target)[0]}")
    assets = client.json(f"releases/tags/{quote_tag(tag)}")["assets"]
    archive_name, wheel_names = required_assets(manifest, target, tag, assets)
    by_name = {entry["name"]: entry["id"] for entry in assets}
    checksums = checksum_entries(client.get(f"releases/assets/{by_name['checksums.txt']}", binary=True))
    for name in [archive_name, *wheel_names]:
        payload = client.get(f"releases/assets/{by_name[name]}", binary=True)
        actual = hashlib.sha256(payload).hexdigest()
        if checksums.get(name) != actual:
            raise ValueError(f"{name}: SHA256 mismatch or missing checksum; expected {checksums.get(name)!r}, got {actual}")
        (destination / name).write_bytes(payload)
        print(f"SHA256 verified: {name}", flush=True)
        record(checks, f"sha256 {name}", "matches checksums.txt")
    extract_archive(destination / archive_name, destination)
    stage = archive_name.removesuffix(".tar.gz").removesuffix(".zip")
    suffix = ".exe" if native_host(target)[0] == "Windows" else ""
    binaries = manifest.get("release_binaries", [])
    if not binaries:
        raise ValueError("release manifest declares no bundled binaries")
    for binary in binaries:
        executable = destination / stage / "bin" / (binary["name"] + suffix)
        if machine(executable.read_bytes()) != EXPECTED[target]:
            raise ValueError(f"{binary['name']}: payload architecture does not match {target}")
        assert_version(run, [str(executable)], binary["name"], version)
        record(checks, f"binary {binary['name']}", f"architecture matches {target}; --version reports {version}")
    for index, name in enumerate(wheel_names):
        wheel = destination / name
        verify_wheel_arch(wheel, target)
        python_version = wheel_python_version(run, wheel_python)
        assert_wheel_python(target, python_version)
        environment = destination / f"wheel-env-{index}"
        run([wheel_python, "-m", "venv", str(environment)])
        scripts = environment / ("Scripts" if suffix else "bin")
        python = scripts / ("python.exe" if suffix else "python")
        run([str(python), "-m", "pip", "install", "--retries", "0", "--disable-pip-version-check", "--force-reinstall", str(wheel)])
        # The wheel exposes the native CLI through its Python binding, not a console script.
        code = "import sys, sc_lint; raise SystemExit(sc_lint.run(['sc-lint', *sys.argv[1:]]))"
        assert_version(run, [str(python), "-I", "-c", code], manifest["project"]["name"], version)
        record(checks, f"wheel {name}", f"installs and reports {version} under Python {python_version[0]}.{python_version[1]}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("plan", "verify", "prerelease-prefix"))
    parser.add_argument("--tag", default="")
    parser.add_argument("--run-id", default="")
    parser.add_argument("--workflow", default="", help=f"workflow that produced --run-id: {RELEASE_WORKFLOW!r} (default) or {PRERELEASE_WORKFLOW!r}")
    parser.add_argument("--commit", default="", help="full commit SHA the tag names; required by plan")
    parser.add_argument("--target")
    parser.add_argument("--wheel-python", help="interpreter the wheel is installed and run under (e.g. Python 3.9)")
    parser.add_argument("--manifest", type=Path, default=Path(MANIFEST_PATH))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--result", type=Path, help="verify: write the per-target JSON result for the native report")
    args = parser.parse_args(argv)
    try:
        if args.command == "prerelease-prefix":
            print(prerelease_prefix(tomllib.loads(args.manifest.read_text(encoding="utf-8"))), end="")
            return 0
        client = GitHub(os.environ.get("GITHUB_REPOSITORY", ""), os.environ.get("GH_TOKEN", ""))
        if args.command == "plan":
            local_manifest = tomllib.loads(args.manifest.read_text(encoding="utf-8"))
            workflow = args.workflow or RELEASE_WORKFLOW
            if workflow not in {RELEASE_WORKFLOW, PRERELEASE_WORKFLOW}:
                raise ValueError(f"unsupported verification workflow {workflow!r}")
            if not is_full_sha(args.commit):
                raise ValueError("plan requires --commit: the full 40-character commit SHA the tag names")
            selected = select_release(client, local_manifest, args.tag, args.run_id, workflow)
            if selected is None:
                tag, commit, matrix = "", "", {"include": []}
            else:
                tag, conclusion = selected
                commit = plan_commit(client, tag, args.commit, dispatch=bool(args.tag), manifest=local_manifest)
                manifest = client.manifest(commit)
                matrix = plan_release(client, manifest, tag, conclusion) or {"include": []}
                if not matrix["include"]:
                    tag, commit = "", ""
            enabled = "true" if tag else "false"
            targets = ",".join(row["target"] for row in matrix["include"])
            output = (
                f"verify={enabled}\ntag={tag}\ncommit={commit}\ntargets={targets}\n"
                f"matrix={json.dumps(matrix, separators=(',', ':'))}\n"
            )
            if args.output:
                with args.output.open("a", encoding="utf-8") as handle:
                    handle.write(output)
            print(output, end="")
        else:
            if not args.target:
                raise ValueError("verify requires --target")
            if not is_full_sha(args.commit):
                raise ValueError("verify requires --commit: the full 40-character tag commit SHA")
            if not args.wheel_python:
                raise ValueError("verify requires --wheel-python")
            checks: list[dict] = []
            error = ""
            try:
                with tempfile.TemporaryDirectory(prefix="release-native-verify-") as directory:
                    verify_release(client, args.tag, args.commit, args.target, Path(directory), wheel_python=args.wheel_python, checks=checks)
            except (ValueError, OSError, KeyError, tarfile.TarError, zipfile.BadZipFile) as caught:
                error = str(caught)
            if args.result:
                write_result(args.result, tag=args.tag, target=args.target, checks=checks, error=error)
            if error:
                print(f"release-native-verify: {error}", file=sys.stderr)
                return 1
        return 0
    except (ValueError, OSError, KeyError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release-native-verify: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
