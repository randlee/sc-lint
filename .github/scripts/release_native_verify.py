"""Execute published archives and wheels on their manifest-selected native hosts."""
from __future__ import annotations

import argparse
import base64
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
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

from verify_release_arch import EXPECTED, machine, verify as verify_wheel_arch
from release_manifest import release_archive_name, release_archive_pattern
from release_python import wheel_asset_pattern

# This is host policy, not a release-target inventory. The manifest owns that inventory.
NATIVE_RUNNERS = {
    ("Linux", "x86_64"): "ubuntu-24.04",
    ("Linux", "aarch64"): "ubuntu-24.04-arm",
    ("Darwin", "x86_64"): "macos-15-intel",
    ("Darwin", "aarch64"): "macos-15",
    ("Windows", "x86_64"): "windows-latest",
    ("Windows", "aarch64"): "windows-11-arm",
}
MANIFEST_PATH = "release/publish-artifacts.toml"


def release_version(tag: str) -> str:
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError(f"invalid release tag {tag!r}; expected vMAJOR.MINOR.PATCH")
    return tag[1:]


def native_host(target: str) -> tuple[str, str]:
    if target not in EXPECTED:
        raise ValueError(f"{target}: no supported native GitHub-hosted runner/architecture verifier")
    kind = EXPECTED[target][0]
    return {"elf": "Linux", "macho": "Darwin", "pe": "Windows"}[kind], target.split("-", 1)[0]


def native_matrix(manifest: dict) -> dict:
    rows = []
    for entry in manifest.get("release_targets", []):
        target = entry["target"]
        host = native_host(target)
        runner = NATIVE_RUNNERS.get(host)
        if runner is None:
            raise ValueError(f"{target}: no native GitHub-hosted runner for {host}")
        if not wheel_specs(manifest, target):
            raise ValueError(f"{target}: manifest declares no wheel to verify")
        rows.append({"target": target, "runner": runner, "python_arch": "arm64" if host[1] == "aarch64" else "x64"})
    if not rows:
        raise ValueError("release manifest declares no release targets")
    if len({row["target"] for row in rows}) != len(rows):
        raise ValueError("release manifest contains duplicate release targets")
    return {"include": rows}


def wheel_specs(manifest: dict, target: str) -> list[tuple[str, str]]:
    return [
        (distribution["name"].replace("-", "_"), wheel["platform"])
        for distribution in manifest.get("python_distributions", [])
        for wheel in distribution.get("wheels", []) if wheel["target"] == target
    ]


class StripRedirectCredentials(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        redirected = super().redirect_request(req, fp, code, msg, headers, newurl)
        if redirected is not None and urllib.parse.urlsplit(req.full_url).netloc != urllib.parse.urlsplit(newurl).netloc:
            redirected.remove_header("Authorization")
        return redirected


class GitHubError(ValueError):
    def __init__(self, endpoint: str, status: int):
        self.status = status
        super().__init__(f"GitHub {endpoint}: HTTP {status}; required release asset or run artifact is unavailable")


class GitHub:
    def __init__(self, repository: str, token: str):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ValueError("GITHUB_REPOSITORY must be owner/repository")
        self.repository = repository
        self.token = token
        self.opener = urllib.request.build_opener(StripRedirectCredentials())

    def get(self, endpoint: str, *, binary: bool = False) -> bytes:
        request = urllib.request.Request(
            f"https://api.github.com/repos/{self.repository}/{endpoint}",
            headers={"Authorization": f"Bearer {self.token}", "User-Agent": "sc-lint-native-release-verifier",
                     # Actions artifact ZIP downloads require the API media type before redirecting.
                     "Accept": "application/octet-stream" if binary and endpoint.startswith("releases/assets/") else "application/vnd.github+json",
                     "X-GitHub-Api-Version": "2022-11-28"},
        )
        try:
            with self.opener.open(request, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            raise GitHubError(endpoint, error.code) from error

    def json(self, endpoint: str) -> dict:
        return json.loads(self.get(endpoint))

    def manifest(self, tag: str) -> dict:
        entry = self.json(f"contents/{MANIFEST_PATH}?ref={urllib.parse.quote(tag, safe='')}")
        return tomllib.loads(base64.b64decode(entry["content"]).decode("utf-8"))


def tag_from_run(client: GitHub, manifest: dict, run_id: str) -> str:
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
    return "v" + versions[0]


def production_release_run(client: GitHub, run_id: str) -> str | None:
    """Return an established production job conclusion, or explain a non-production skip."""
    if not run_id.isdecimal():
        raise ValueError("automatic verification requires the triggering Release run id")
    jobs = []
    page = 1
    while True:
        data = client.json(f"actions/runs/{run_id}/jobs?filter=latest&per_page=100&page={page}")
        jobs.extend(job for job in data["jobs"] if job["name"] == "release")
        if page * 100 >= data["total_count"]:
            break
        page += 1
    if not jobs:
        print(f"Skipping native verification: Release run {run_id} has no production release job.", flush=True)
        return None
    if len(jobs) != 1:
        raise ValueError(f"Release run {run_id}: cannot establish production status; expected one release job, found {len(jobs)}")
    conclusion = jobs[0].get("conclusion")
    if conclusion == "skipped":
        print(f"Skipping native verification: Release run {run_id} did not run its production release job (conclusion: skipped).", flush=True)
        return None
    if conclusion not in {"success", "failure", "cancelled"}:
        raise ValueError(f"Release run {run_id}: cannot establish production status; release job conclusion is {conclusion!r}")
    return conclusion


def plan_release(client: GitHub, manifest: dict, tag: str, run_id: str) -> tuple[str, dict] | None:
    conclusion = "success" if tag else production_release_run(client, run_id)
    if conclusion is None:
        return None
    tag = tag or tag_from_run(client, manifest, run_id)
    release_version(tag)
    try:
        client.json(f"releases/tags/{urllib.parse.quote(tag, safe='')}")
    except GitHubError as error:
        if error.status != 404 or conclusion == "success":
            raise
        print(f"Skipping native verification: Release run {run_id} release job concluded {conclusion} and {tag} has no published release (HTTP 404).", flush=True)
        return None
    return tag, native_matrix(client.manifest(tag))


def assert_native(target: str) -> None:
    expected = native_host(target)
    aliases = {"amd64": "x86_64", "x86_64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}
    actual = platform.system(), aliases.get(platform.machine().lower(), platform.machine())
    print(f"Native host: {actual}; target: {target}; Python: {sys.version}", flush=True)
    if actual != expected:
        raise ValueError(f"{target}: native runner required; expected {expected}, got {actual}")
    if sys.version_info < (3, 11):
        raise ValueError("wheel verification requires Python 3.11 or newer, including win_arm64")


def required_assets(manifest: dict, target: str, tag: str, assets: list[dict]) -> tuple[str, list[str]]:
    version = release_version(tag)
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


def verify_release(client: GitHub, tag: str, target: str, destination: Path, *, run=run_command) -> None:
    version = release_version(tag)
    assert_native(target)
    manifest = client.manifest(tag)
    assets = client.json(f"releases/tags/{urllib.parse.quote(tag, safe='')}")["assets"]
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
    for index, name in enumerate(wheel_names):
        wheel = destination / name
        verify_wheel_arch(wheel, target)
        environment = destination / f"wheel-env-{index}"
        run([sys.executable, "-m", "venv", str(environment)])
        scripts = environment / ("Scripts" if suffix else "bin")
        python = scripts / ("python.exe" if suffix else "python")
        run([str(python), "-m", "pip", "install", "--retries", "0", "--disable-pip-version-check", "--force-reinstall", str(wheel)])
        # The wheel exposes the native CLI through its Python binding, not a console script.
        code = "import sys, sc_lint; raise SystemExit(sc_lint.run(['sc-lint', *sys.argv[1:]]))"
        assert_version(run, [str(python), "-I", "-c", code], manifest["project"]["name"], version)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("plan", "verify"))
    parser.add_argument("--tag", default="")
    parser.add_argument("--run-id", default="")
    parser.add_argument("--target")
    parser.add_argument("--manifest", type=Path, default=Path(MANIFEST_PATH))
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        client = GitHub(os.environ.get("GITHUB_REPOSITORY", ""), os.environ.get("GH_TOKEN", ""))
        if args.command == "plan":
            local_manifest = tomllib.loads(args.manifest.read_text(encoding="utf-8"))
            plan = plan_release(client, local_manifest, args.tag, args.run_id)
            tag, matrix = plan if plan is not None else ("", {"include": []})
            enabled = "true" if plan is not None else "false"
            output = f"verify={enabled}\ntag={tag}\nmatrix={json.dumps(matrix, separators=(',', ':'))}\n"
            if args.output:
                with args.output.open("a", encoding="utf-8") as handle:
                    handle.write(output)
            print(output, end="")
        else:
            if not args.target:
                raise ValueError("verify requires --target")
            with tempfile.TemporaryDirectory(prefix="release-native-verify-") as directory:
                verify_release(client, args.tag, args.target, Path(directory))
        return 0
    except (ValueError, OSError, KeyError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release-native-verify: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
