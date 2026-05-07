from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from run_version import cargo_outdated_command
from run_version import workspace_version


ROOT_MANIFEST = """\
[workspace]
members = ["crates/sc-lint-boundary", "crates/sc-lint-attributes"]
resolver = "2"

[workspace.package]
version = "0.1.0"
"""


class RunVersionTests(unittest.TestCase):
    def test_workspace_version_reads_root_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(ROOT_MANIFEST, encoding="utf-8")
            self.assertEqual(workspace_version(repo_root), "0.1.0")

    def test_cargo_outdated_command_uses_manifest_path_and_root_deps_only(self) -> None:
        manifest_path = Path("/tmp/repo/crates/sc-lint-boundary/Cargo.toml")
        self.assertEqual(
            cargo_outdated_command(manifest_path),
            [
                "cargo",
                "outdated",
                "--format",
                "json",
                "--manifest-path",
                str(manifest_path),
                "--root-deps-only",
            ],
        )


if __name__ == "__main__":
    unittest.main()
