"""Unit tests for the prerelease tag script; git is replaced by a fake."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import os
import tempfile
import unittest
from pathlib import Path
from typing import Sequence

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / ".just/prerelease_tag.py"
spec = importlib.util.spec_from_file_location("prerelease_tag", SCRIPT)
prerelease_tag = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prerelease_tag)

CARGO = '[workspace.package]\nversion = "0.8.1"\n'


class FakeGit:
    def __init__(self, local: str = "", remote: str = "") -> None:
        self.local, self.remote, self.calls = local, remote, []

    def __call__(self, args: Sequence[str]) -> str:
        self.calls.append(list(args))
        if args[:2] == ["tag", "--list"]:
            return self.local
        if args[0] == "ls-remote":
            return self.remote
        if args[0] == "rev-parse":
            return "abc123\n"
        return ""


def run_main(argv: list[str], git: FakeGit) -> tuple[int, str]:
    with tempfile.TemporaryDirectory() as directory:
        (Path(directory) / "Cargo.toml").write_text(CARGO, encoding="utf-8")
        previous = Path.cwd()
        os.chdir(directory)
        try:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = prerelease_tag.main(argv, git)
        finally:
            os.chdir(previous)
    return code, out.getvalue()


class PrereleaseTagTests(unittest.TestCase):
    def test_workspace_version_reads_workspace_package(self) -> None:
        self.assertEqual(prerelease_tag.workspace_version(CARGO), "0.8.1")

    def test_workspace_version_rejects_missing_and_non_stable(self) -> None:
        with self.assertRaises(SystemExit):
            prerelease_tag.workspace_version("[package]\nname = 'x'\n")
        with self.assertRaises(SystemExit):
            prerelease_tag.workspace_version('[workspace.package]\nversion = "0.8.1-rc.1"\n')

    def test_dry_run_prints_plan_and_changes_nothing(self) -> None:
        git = FakeGit()
        code, output = run_main(["--dry-run"], git)
        self.assertEqual((code, output), (0, "would create and push prerelease/v0.8.1\n"))
        self.assertFalse([c for c in git.calls if c[0] in {"tag", "push"} and c[:2] != ["tag", "--list"]])

    def test_publish_creates_and_pushes_tag_once(self) -> None:
        git = FakeGit()
        code, output = run_main([], git)
        self.assertEqual(code, 0)
        self.assertEqual(output, "created and pushed prerelease/v0.8.1 from abc123\n")
        self.assertIn(["tag", "prerelease/v0.8.1"], git.calls)
        self.assertIn(["push", "origin", "refs/tags/prerelease/v0.8.1"], git.calls)

    def test_output_matches_helper_parser(self) -> None:
        _, output = run_main([], FakeGit())
        self.assertEqual(output.split()[3], "prerelease/v0.8.1")

    def test_existing_local_tag_fails_closed(self) -> None:
        git = FakeGit(local="prerelease/v0.8.1\n")
        with self.assertRaises(SystemExit):
            run_main([], git)
        self.assertNotIn(["tag", "prerelease/v0.8.1"], git.calls)

    def test_existing_remote_tag_fails_closed(self) -> None:
        git = FakeGit(remote="deadbeef\trefs/tags/prerelease/v0.8.1\n")
        with self.assertRaises(SystemExit):
            run_main(["--dry-run"], git)
        self.assertFalse([c for c in git.calls if c[0] == "push"])


if __name__ == "__main__":
    unittest.main()
