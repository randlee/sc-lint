"""Contracts shared by the native verifier, its report renderer and the publish gate.

The verify workflow's run title and job names are part of the gate's contract: the gate
finds the run that proved a tag at a commit by its title, and requires the named jobs to
have succeeded. tests/test_release_native_gate.py ties the workflow text to these helpers.
The native host and matrix policy lives in release_native_matrix.py.
"""
from __future__ import annotations

import base64
import json
import re
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from collections.abc import Iterator

MANIFEST_PATH = "release/publish-artifacts.toml"
RESULT_SCHEMA_VERSION = 1
REPORT_JOB = "Render native verification report"
PAGE_SIZE = 100
MAX_PAGES = 10
VERIFY_WORKFLOW_NAME = "Release Native Verify"
VERIFY_WORKFLOW_FILE = "release-native-verify.yml"
VERIFY_WORKFLOW_PATH = f".github/workflows/{VERIFY_WORKFLOW_FILE}"
RELEASE_WORKFLOW = "Release"
PRERELEASE_WORKFLOW = "Prerelease Archive"
# The one identifier per build workflow: its display name, and the file that runs it. Every
# query, trust rule and message that names a build workflow reads from this table.
BUILD_WORKFLOWS = {RELEASE_WORKFLOW: "release.yml", PRERELEASE_WORKFLOW: "prerelease-archive.yml"}
PRERELEASE_KIND = "prerelease"
RELEASE_KIND = "release"


def verify_run_title(tag: str, commit: str) -> str:
    """The run-name of release-native-verify.yml for one tag at one commit."""
    return f"{VERIFY_WORKFLOW_NAME} {tag} {commit}"


def native_job(target: str) -> str:
    """The job name of one target's native verification, as release-native-verify.yml names it."""
    return f"Native {target}"


def is_full_sha(value: str) -> bool:
    return re.fullmatch(r"[0-9a-f]{40}", value) is not None


def quote_tag(tag: str) -> str:
    """Encode a tag for a GitHub API path segment; a prerelease tag's slash becomes %2F."""
    return urllib.parse.quote(tag, safe="")


def quote_ref(ref: str) -> str:
    """Encode a ref for a GitHub git-refs path, where the slashes of a prerelease tag are path separators."""
    return urllib.parse.quote(ref, safe="/")


def prerelease_prefix(manifest: dict) -> str:
    """The manifest's prerelease tag prefix, which ends in v (prerelease/v).

    It is validated because the prefix is spliced into tag names, refs and regular expressions.
    """
    prefix = manifest.get("prerelease", {}).get("tag_prefix", "")
    if not re.fullmatch(r"[A-Za-z0-9._/-]+v", prefix):
        raise ValueError(f"manifest prerelease.tag_prefix must match [A-Za-z0-9._/-]+v, got {prefix!r}")
    return prefix


def release_version(tag: str, manifest: dict) -> str:
    """The version of a release tag (vX.Y.Z), or of a prerelease tag when the manifest declares a prefix."""
    prefixes = ["v"]
    if manifest.get("prerelease", {}).get("tag_prefix"):
        prefixes.insert(0, re.escape(prerelease_prefix(manifest)))
    match = re.fullmatch(rf"(?:{'|'.join(prefixes)})([0-9]+\.[0-9]+\.[0-9]+)", tag)
    if match is None:
        raise ValueError(f"invalid release tag {tag!r}; expected vMAJOR.MINOR.PATCH or the manifest's prerelease tag")
    return match.group(1)


def tag_kind(tag: str, manifest: dict) -> str:
    """Classify a valid tag as a prerelease or a release; the build workflow and trust rules follow from it."""
    release_version(tag, manifest)
    prefix = manifest.get("prerelease", {}).get("tag_prefix", "")
    return PRERELEASE_KIND if prefix and tag.startswith(prefix) else RELEASE_KIND


def build_workflow_for(tag: str, manifest: dict) -> str:
    """The build workflow that publishes a tag's archives: Prerelease Archive for a prerelease, Release otherwise."""
    return PRERELEASE_WORKFLOW if tag_kind(tag, manifest) == PRERELEASE_KIND else RELEASE_WORKFLOW


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
        # An empty endpoint names the repository itself; a trailing slash is not a valid API path.
        url = f"https://api.github.com/repos/{self.repository}" + (f"/{endpoint}" if endpoint else "")
        request = urllib.request.Request(
            url,
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

    def manifest(self, ref: str) -> dict:
        """The release manifest as it reads at a ref: a tag or, for the gate, the verified commit."""
        entry = self.json(f"contents/{MANIFEST_PATH}?ref={quote_tag(ref)}")
        return tomllib.loads(base64.b64decode(entry["content"]).decode("utf-8"))


def paged(client: GitHub, path: str, key: str, *, max_pages: int = MAX_PAGES) -> Iterator[dict]:
    """Yield every item of a paginated GitHub list, reading at most max_pages pages.

    The path carries its own query string; the page size and page number are appended to it.
    """
    separator = "&" if "?" in path else "?"
    for page in range(1, max_pages + 1):
        data = client.json(f"{path}{separator}per_page={PAGE_SIZE}&page={page}")
        yield from data.get(key, [])
        if page * PAGE_SIZE >= data.get("total_count", 0):
            return


def tag_commit(client: GitHub, tag: str) -> str:
    """The commit a tag names, read from refs/tags so a branch of the same name cannot answer.

    An annotated tag points at a tag object, which is peeled until it reaches a commit.
    """
    target = client.json(f"git/ref/tags/{quote_ref(tag)}")["object"]
    while target["type"] == "tag":
        target = client.json(f"git/tags/{target['sha']}")["object"]
    if target["type"] != "commit":
        raise ValueError(f"{tag} does not name a commit; it names a {target['type']}")
    return target["sha"]
