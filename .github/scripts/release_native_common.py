"""Contracts shared by the native verifier, its report renderer and the publish gate.

The verify workflow's run title and job names are part of the gate's contract: the gate
finds the run that proved a tag at a commit by its title, and requires the named jobs to
have succeeded. tests/test_release_native_gate.py ties the workflow text to these helpers.
"""
from __future__ import annotations

import base64
import json
import re
import tomllib
import urllib.error
import urllib.parse
import urllib.request

from verify_release_arch import EXPECTED

MANIFEST_PATH = "release/publish-artifacts.toml"
RESULT_SCHEMA_VERSION = 1
REPORT_JOB = "Render native verification report"
RELEASE_WORKFLOW = "Release"
PRERELEASE_WORKFLOW = "Prerelease Archive"
# The workflows whose release job publishes archives and wheels for a commit.
BUILD_WORKFLOW_FILES = ("prerelease-archive.yml", "release.yml")

# This is host policy, not a release-target inventory. The manifest owns that inventory.
NATIVE_RUNNERS = {
    ("Linux", "x86_64"): "ubuntu-24.04",
    ("Linux", "aarch64"): "ubuntu-24.04-arm",
    ("Darwin", "x86_64"): "macos-15-intel",
    ("Darwin", "aarch64"): "macos-15",
    ("Windows", "x86_64"): "windows-latest",
    ("Windows", "aarch64"): "windows-11-arm",
}
# Wheels are verified on Python 3.9 (the abi3-py39 floor); Windows ARM64 needs 3.11+.
WHEEL_PYTHON = "3.9"
WIN_ARM64_TARGET = "aarch64-pc-windows-msvc"
WIN_ARM64_PYTHON = "3.11"


def verify_run_title(tag: str, commit: str) -> str:
    """The run-name of release-native-verify.yml for one tag at one commit."""
    return f"Release Native Verify {tag} {commit}"


def native_job(target: str) -> str:
    """The job name of one target's native verification, as release-native-verify.yml names it."""
    return f"Native {target}"


def is_full_sha(value: str) -> bool:
    return re.fullmatch(r"[0-9a-f]{40}", value) is not None


def quote_tag(tag: str) -> str:
    """Encode a tag for a GitHub API path segment; a prerelease tag's slash becomes %2F."""
    return urllib.parse.quote(tag, safe="")


def release_version(tag: str, manifest: dict) -> str:
    """The version of a release tag (vX.Y.Z) or a prerelease tag (the manifest's tag_prefix, which ends in v)."""
    prefix = re.escape(manifest["prerelease"]["tag_prefix"])
    match = re.fullmatch(rf"(?:{prefix}|v)([0-9]+\.[0-9]+\.[0-9]+)", tag)
    if match is None:
        raise ValueError(f"invalid release tag {tag!r}; expected vMAJOR.MINOR.PATCH or a prerelease tag of the manifest's prefix")
    return match.group(1)


def native_host(target: str) -> tuple[str, str]:
    if target not in EXPECTED:
        raise ValueError(f"{target}: no supported native GitHub-hosted runner/architecture verifier")
    kind = EXPECTED[target][0]
    return {"elf": "Linux", "macho": "Darwin", "pe": "Windows"}[kind], target.split("-", 1)[0]


def wheel_specs(manifest: dict, target: str) -> list[tuple[str, str]]:
    return [
        (distribution["name"].replace("-", "_"), wheel["platform"])
        for distribution in manifest.get("python_distributions", [])
        for wheel in distribution.get("wheels", []) if wheel["target"] == target
    ]


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
        python_version = WIN_ARM64_PYTHON if target == WIN_ARM64_TARGET else WHEEL_PYTHON
        rows.append({
            "target": target, "runner": runner, "python_arch": "arm64" if host[1] == "aarch64" else "x64",
            "python_version": python_version,
        })
    if not rows:
        raise ValueError("release manifest declares no release targets")
    if len({row["target"] for row in rows}) != len(rows):
        raise ValueError("release manifest contains duplicate release targets")
    return {"include": rows}


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

    def default_branch(self) -> str:
        return self.json("")["default_branch"]

    def manifest(self, tag: str) -> dict:
        entry = self.json(f"contents/{MANIFEST_PATH}?ref={quote_tag(tag)}")
        return tomllib.loads(base64.b64decode(entry["content"]).decode("utf-8"))
