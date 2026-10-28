"""Regression guard: installer-substituted templates must not carry sc-compose front matter."""
from __future__ import annotations

from pathlib import Path

import pytest
from install_helpers import ROOT
from install_helpers import run_subprocess


def has_front_matter(data: bytes) -> bool:
    lines = data.splitlines()
    return lines[:1] == [b"---"] and b"---" in lines[1:]


def tracked_template_files() -> list[Path] | None:
    try:
        top_level = run_subprocess(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
    except FileNotFoundError:
        return None
    if top_level.returncode != 0 or Path(top_level.stdout.strip()).resolve() != ROOT:
        return None
    listed = run_subprocess(
        ["git", "ls-files", "-z", "--", "packages/sc-lint-adoption"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if listed.returncode != 0:
        raise RuntimeError(f"git ls-files failed: {listed.stderr}")
    return [ROOT / name for name in listed.stdout.split("\0") if name.endswith(".tmpl")]


def test_no_tracked_tmpl_file_has_front_matter() -> None:
    templates = tracked_template_files()
    if templates is None:
        pytest.skip("not a git work tree")
    offenders = [str(path.relative_to(ROOT)) for path in templates if has_front_matter(path.read_bytes())]
    assert not offenders, f"tracked .tmpl files carry front matter: {offenders}"


@pytest.mark.parametrize(
    ("data", "expected"),
    [
        (b"---\na: 1\n---\nbody", True),
        (b"---\r\na: 1\r\n---\r\nbody", True),
        (b"body\n---\na\n---\n", False),
        (b"---\nunclosed\n", False),
        (b"", False),
    ],
)
def test_has_front_matter_classifies_controlled_bytes(data: bytes, expected: bool) -> None:
    assert has_front_matter(data) is expected
