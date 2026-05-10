from __future__ import annotations

from pathlib import Path
from pathlib import PureWindowsPath
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from lint_manifests import collect_manifest_violations
from lint_manifests import relative_manifest_display


CORE_CRATE = "fixture-lib"
APP_CRATE = "fixture-app"
CORE_PACKAGE = "synthetic-lib"
APP_PACKAGE = "synthetic-app"
DEPENDENCY_KEY = "fixture-lib"
WINDOWS_REPO_ROOT = PureWindowsPath(r"D:\a\synthetic-repo\synthetic-repo")


def root_manifest(*members: str) -> str:
    rendered_members = ", ".join(f'"crates/{member}"' for member in members)
    return f"""\
[workspace]
members = [{rendered_members}]
resolver = "2"

[workspace.package]
version = "1.1.2"
edition = "2024"
rust-version = "1.94.1"
authors = ["synthetic contributors"]
license = "MIT OR Apache-2.0"
repository = "https://example.invalid/repo"
homepage = "https://example.invalid/repo"
"""


GOOD_MEMBER = f"""\
[package]
name = "{CORE_PACKAGE}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
serde = "1"
"""


class LintManifestsTests(unittest.TestCase):
    def test_relative_manifest_display_uses_posix_separators(self) -> None:
        repo_root = WINDOWS_REPO_ROOT
        manifest_path = WINDOWS_REPO_ROOT / "crates" / CORE_CRATE / "Cargo.toml"

        self.assertEqual(
            relative_manifest_display(manifest_path, repo_root),
            f"crates/{CORE_CRATE}/Cargo.toml",
        )

    def write_repo(self, repo_root: Path) -> None:
        (repo_root / "Cargo.toml").write_text(
            root_manifest(CORE_CRATE, APP_CRATE),
            encoding="utf-8",
        )
        crate_dir = repo_root / "crates" / CORE_CRATE
        crate_dir.mkdir(parents=True)
        (crate_dir / "Cargo.toml").write_text(GOOD_MEMBER, encoding="utf-8")

    def test_collect_manifest_violations_accepts_workspace_inheritance(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_repo(repo_root)
            self.assertEqual(collect_manifest_violations(repo_root), [])

    def test_collect_manifest_violations_flags_missing_workspace_field(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_repo(repo_root)
            manifest_path = repo_root / "crates" / CORE_CRATE / "Cargo.toml"
            manifest_path.write_text(GOOD_MEMBER.replace("homepage.workspace = true\n", ""), encoding="utf-8")

            violations = collect_manifest_violations(repo_root)
            rendered = [violation.render() for violation in violations]
            self.assertIn(
                f"crates/{CORE_CRATE}/Cargo.toml: set [package].homepage.workspace = true",
                rendered,
            )

    def test_collect_manifest_violations_flags_mismatched_internal_path_version(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                root_manifest(APP_CRATE, CORE_CRATE),
                encoding="utf-8",
            )
            core_dir = repo_root / "crates" / CORE_CRATE
            core_dir.mkdir(parents=True)
            (core_dir / "Cargo.toml").write_text(GOOD_MEMBER, encoding="utf-8")
            app_dir = repo_root / "crates" / APP_CRATE
            app_dir.mkdir(parents=True)
            (app_dir / "Cargo.toml").write_text(
                f"""\
[package]
name = "{APP_PACKAGE}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
{DEPENDENCY_KEY} = {{ path = "../{CORE_CRATE}", version = "9.9.9" }}
""",
                encoding="utf-8",
            )

            violations = collect_manifest_violations(repo_root)
            rendered = [violation.render() for violation in violations]
            self.assertIn(
                f'crates/{APP_CRATE}/Cargo.toml [dependencies.{DEPENDENCY_KEY}]: '
                'path dependency version must match target crate version "1.1.2"',
                rendered,
            )

    def test_collect_manifest_violations_accepts_explicit_tool_crate_version(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                root_manifest(CORE_CRATE, APP_CRATE, "sc-lint-attributes"),
                encoding="utf-8",
            )
            self.write_repo(repo_root)
            tool_dir = repo_root / "crates/sc-lint-attributes"
            tool_dir.mkdir(parents=True)
            (tool_dir / "Cargo.toml").write_text(
                """\
[package]
name = "sc-lint-attributes"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
""",
                encoding="utf-8",
            )

            self.assertEqual(collect_manifest_violations(repo_root), [])


if __name__ == "__main__":
    unittest.main()
