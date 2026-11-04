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

MANIFEST_PATH = "release/publish-artifacts.toml"
RESULT_SCHEMA_VERSION = 1
REPORT_JOB = "Render native verification report"


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
        entry = self.json(f"contents/{MANIFEST_PATH}?ref={quote_tag(tag)}")
        return tomllib.loads(base64.b64decode(entry["content"]).decode("utf-8"))
