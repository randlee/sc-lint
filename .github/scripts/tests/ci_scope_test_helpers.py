"""Tracked-source read discovery helpers for the CI scope regression tests."""

from __future__ import annotations

import glob
from functools import lru_cache
import os
import re
import subprocess
import tomllib
from pathlib import Path, PurePosixPath

import yaml

REPO = Path(__file__).resolve().parents[3]
GIT_TIMEOUT_SECONDS = 60
EXEMPT_PREFIXES = ("docs", "site/reports", ".beads", ".atm-bd", ".triage")
STRING_LITERAL = re.compile(r'''"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*' ''', re.VERBOSE)
PYTEST_ROOT = r"(?:ROOT|REPO|REPO_ROOT|PACKAGE_ROOT|repo_root\(\)|Path\(__file__\)(?:\.resolve\(\))?(?:\.parents\[\d+\]|\.parent))"
PYTEST_READ = re.compile(
    rf"(?P<root>{PYTEST_ROOT})\s*(?P<parts>(?:/\s*[\"'][^\"']+[\"']\s*)+)"
)
PYTEST_JOINPATH = re.compile(rf"(?P<root>{PYTEST_ROOT})\.joinpath\((?P<parts>[^)]*)\)")


def _git_env() -> dict[str, str]:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(
        HOME=os.devnull,
        USERPROFILE=os.devnull,
        XDG_CONFIG_HOME=os.devnull,
        GIT_CONFIG_GLOBAL=os.devnull,
        GIT_CONFIG_SYSTEM=os.devnull,
        GIT_CONFIG_NOSYSTEM="1",
    )
    return env


@lru_cache(maxsize=None)
def tracked_paths(repo: Path = REPO) -> list[Path]:
    """Return Git-index paths only, preserving arbitrary filesystem name bytes."""
    result = subprocess.run(
        ["git", "ls-files", "-z"], cwd=repo, env=_git_env(), capture_output=True,
        check=True, timeout=GIT_TIMEOUT_SECONDS,
    )
    names = result.stdout.decode("utf-8", errors="surrogateescape").split("\0")
    return [repo.joinpath(*PurePosixPath(name).parts) for name in names if name]


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def _repo_relative(repo: Path, path: Path) -> str | None:
    try:
        return path.resolve().relative_to(repo.resolve()).as_posix()
    except ValueError:
        return None


def _matches_exempt_prefix(path: str) -> bool:
    return any(path == prefix or path.startswith(prefix + "/") for prefix in EXEMPT_PREFIXES)


def _crate_root(source: Path) -> Path:
    return next((parent for parent in source.parents if (parent / "Cargo.toml").is_file()), source.parent)


def _source_literals(source: Path) -> list[str]:
    literals = []
    for match in STRING_LITERAL.finditer(_read_text(source)):
        token = match.group(0)
        literals.append(token[1:-1].replace(r"\/", "/"))
    return literals


def _candidate_from_literal(repo: Path, source: Path, literal: str, *, base: Path | None = None) -> str | None:
    normalized = literal.replace("\\", "/")
    for index in range(len(normalized)):
        for prefix in EXEMPT_PREFIXES:
            if normalized.startswith(prefix + "/", index):
                before = normalized[:index].strip("/")
                path_text = "/".join(part for part in (before, normalized[index:]) if part)
                relative = _repo_relative(repo, (base or repo) / path_text)
                if relative and _matches_exempt_prefix(relative) and (repo / relative).is_file():
                    return relative
    return None


def prefix_literal_read_paths(repo: Path = REPO) -> set[str]:
    """Sweep tracked read expressions for literals into every exempt prefix.

    The sweep is limited to rooted test reads and Rust file-read expressions,
    avoiding arbitrary documentation examples and test assertion strings.
    """
    return _rust_read_expression_paths(repo) | _pytest_root_read_paths(repo)


def _rust_read_expression_paths(repo: Path = REPO) -> set[str]:
    """Collect include macros and common fs::read paths, including manifest joins."""
    result = set()
    read_start = re.compile(
        r"\b(?:include_(?:str|bytes)|(?:(?:[A-Za-z_]\w*::)*)fs::read(?:_to_string)?|"
        r"(?:(?:[A-Za-z_]\w*::)*)File::open|read_to_string|read_text|read_bytes|open)\s*!?\s*\("
    )
    for source in tracked_paths(repo):
        if source.suffix != ".rs" or not source.is_file():
            continue
        text = _read_text(source)
        crate_root = _crate_root(source)
        for call in read_start.finditer(text):
            expression = text[call.end():text.find(";", call.end()) if ";" in text[call.end():] else len(text)]
            literals = _source_literals_from_text(expression)
            if not literals:
                continue
            if "CARGO_MANIFEST_DIR" in expression and "concat!" in expression:
                joined = "".join(value for value in literals if value != "CARGO_MANIFEST_DIR")
                candidate = _candidate_from_literal(repo, source, joined, base=crate_root)
                if candidate:
                    result.add(candidate)
            elif "CARGO_MANIFEST_DIR" in expression:
                target = crate_root
                for value in literals:
                    if value in {"CARGO_MANIFEST_DIR"}:
                        continue
                    target = target / value
                relative = _repo_relative(repo, target)
                if relative and _matches_exempt_prefix(relative) and target.is_file():
                    result.add(relative)
            else:
                for base in (source.parent, crate_root, repo):
                    candidate = next(
                        (found for literal in literals if (found := _candidate_from_literal(repo, source, literal, base=base))),
                        None,
                    )
                    if candidate:
                        result.add(candidate)
    return result


def _source_literals_from_text(text: str) -> list[str]:
    return [token[1:-1].replace(r"\/", "/") for token in STRING_LITERAL.findall(text)]


def _pytest_root_read_paths(repo: Path = REPO) -> set[str]:
    result = set()
    for source in tracked_paths(repo):
        if source.suffix != ".py" or not source.is_file():
            continue
        text = _read_text(source)
        matches = list(PYTEST_READ.finditer(text)) + list(PYTEST_JOINPATH.finditer(text))
        for match in matches:
            root = match.group("root")
            if root.startswith("Path(__file__)"):
                parent_match = re.search(r"\.parents\[(\d+)\]", root)
                base = source.parents[int(parent_match.group(1))] if parent_match else source.parent
            else:
                base = repo
            segments = re.findall(r"[\"']([^\"']+)[\"']", match.group("parts"))
            target = base.joinpath(*segments)
            relative = _repo_relative(repo, target)
            if relative and _matches_exempt_prefix(relative) and target.is_file():
                result.add(relative)
            elif target.is_dir():
                for child in tracked_paths(repo):
                    child_relative = _repo_relative(repo, child)
                    if child_relative and child.is_file() and child.is_relative_to(target.resolve()):
                        result.add(child_relative)
    return result


def lint_glob_read_paths(repo: Path = REPO) -> set[str]:
    """Files selected by lint config; each configured glob must stay runtime."""
    def file_globs(node):
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "files" and isinstance(value, list):
                    yield from value
                else:
                    yield from file_globs(value)
        elif isinstance(node, list):
            for item in node:
                yield from file_globs(item)

    config = tomllib.loads(_read_text(repo / ".just/lint-config.toml"))
    result = set()
    tracked = {_repo_relative(repo, path) for path in tracked_paths(repo)}
    for pattern in file_globs(config):
        for match in glob.glob(str(repo / pattern), recursive=True):
            path = Path(match)
            relative = _repo_relative(repo, path)
            if relative and relative in tracked and path.is_file():
                result.add(relative)
    return result


@lru_cache(maxsize=None)
def read_paths(repo: Path = REPO) -> dict[str, set[str]]:
    """All statically discovered files read by tests, Rust includes and lints."""
    return {
        "tracked-source prefix literals": prefix_literal_read_paths(repo),
        "Rust include/fs reads": _rust_read_expression_paths(repo),
        "pytest repository-root reads": _pytest_root_read_paths(repo),
        "lint files globs": lint_glob_read_paths(repo),
    }


def parse_workflow(path: Path = REPO / ".github/workflows/ci.yml"):
    return yaml.safe_load(_read_text(path))
