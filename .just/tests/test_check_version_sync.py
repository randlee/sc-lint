from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from check_version_sync import validate_crate_versions
from check_version_sync import validate_lockfile
from check_version_sync import validate_winget_manifests
from check_version_sync import success_message


APP_CRATE = "fixture-app"
CORE_CRATE = "fixture-lib"
DAEMON_CRATE = "fixture-daemon"
STORAGE_CRATE = "fixture-storage"
APP_PACKAGE = "synthetic-app"
CORE_PACKAGE = "synthetic-lib"
DAEMON_PACKAGE = "synthetic-daemon"
STORAGE_PACKAGE = "synthetic-storage"
DEPENDENCY_KEY = "fixture-lib"
WINGET_PACKAGE_ID = "example.synthetic-app"


ROOT_MANIFEST = f"""\
[workspace]
members = ["crates/{APP_CRATE}", "crates/{CORE_CRATE}", "crates/{DAEMON_CRATE}", "crates/{STORAGE_CRATE}"]
resolver = "2"

[workspace.package]
version = "1.1.2"
"""


def crate_manifest(name: str, extra: str = "") -> str:
    return f"""\
[package]
name = "{name}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
{extra}
"""


class CheckVersionSyncTests(unittest.TestCase):
    def write_repo(self, repo_root: Path) -> None:
        (repo_root / "Cargo.toml").write_text(ROOT_MANIFEST, encoding="utf-8")
        crates_dir = repo_root / "crates"
        for crate_name, package_name in (
            (APP_CRATE, APP_PACKAGE),
            (CORE_CRATE, CORE_PACKAGE),
            (DAEMON_CRATE, DAEMON_PACKAGE),
            (STORAGE_CRATE, STORAGE_PACKAGE),
        ):
            crate_dir = crates_dir / crate_name
            crate_dir.mkdir(parents=True)
            extra = ""
            if crate_name == APP_CRATE:
                extra = (
                    f'\n[dependencies]\n{DEPENDENCY_KEY} = '
                    f'{{ package = "{CORE_PACKAGE}", path = "../{CORE_CRATE}", version = "1.1.2" }}\n'
                )
            (crate_dir / "Cargo.toml").write_text(crate_manifest(package_name, extra=extra), encoding="utf-8")

    def test_validate_crate_versions_checks_all_member_manifests(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_repo(repo_root)
            manifest = repo_root / "crates" / STORAGE_CRATE / "Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace("version.workspace = true\n", ""),
                encoding="utf-8",
            )

            with self.assertRaises(SystemExit) as error:
                validate_crate_versions(repo_root, "1.1.2")

            message = str(error.exception).replace("\\", "/")
            self.assertIn(
                f"crates/{STORAGE_CRATE}/Cargo.toml must define [package].version either as a non-empty string or version.workspace = true",
                message,
            )

    def test_validate_lockfile_checks_all_workspace_packages(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_repo(repo_root)
            (repo_root / "Cargo.lock").write_text(
                """\
version = 3

[[package]]
name = "synthetic-app"
version = "1.1.2"

[[package]]
name = "synthetic-lib"
version = "1.1.2"

[[package]]
name = "synthetic-daemon"
version = "1.1.2"
""",
                encoding="utf-8",
            )

            with self.assertRaises(SystemExit) as error:
                validate_lockfile(repo_root, "1.1.2")

            self.assertIn(f"{STORAGE_PACKAGE} missing from Cargo.lock", str(error.exception))

    def test_validate_crate_versions_requires_internal_path_dep_pin(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_repo(repo_root)
            manifest = repo_root / "crates" / APP_CRATE / "Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(', version = "1.1.2"', ""),
                encoding="utf-8",
            )

            with self.assertRaises(SystemExit) as error:
                validate_crate_versions(repo_root, "1.1.2")

            self.assertIn(
                f'crates/{APP_CRATE}/Cargo.toml [dependencies.{DEPENDENCY_KEY}]: '
                'internal path dependency version must match target crate version "1.1.2"',
                str(error.exception),
            )

    def test_validate_crate_versions_accepts_explicit_tool_crate_versions(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                f"""\
[workspace]
members = ["crates/{CORE_CRATE}", "crates/sc-lint-attributes"]
resolver = "2"

[workspace.package]
version = "1.1.2"
""",
                encoding="utf-8",
            )
            core_dir = repo_root / "crates" / CORE_CRATE
            core_dir.mkdir(parents=True)
            (core_dir / "Cargo.toml").write_text(
                crate_manifest(
                    CORE_PACKAGE,
                    extra='\n[dependencies]\nsc-lint-attributes = { path = "../sc-lint-attributes", version = "0.1.0" }\n',
                ),
                encoding="utf-8",
            )
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

            validate_crate_versions(repo_root, "1.1.2")

    def test_success_message_includes_workspace_version(self) -> None:
        self.assertEqual(
            success_message("1.1.2", ["workspace member versions", "internal path deps", "Cargo.lock"]),
            "version sync check passed: workspace_version=1.1.2; workspace member versions, internal path deps, Cargo.lock are aligned.",
        )

    def test_validate_winget_manifests_reads_installer_url_from_installers_array(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / ".winget").mkdir(parents=True)
            (repo_root / f".winget/{WINGET_PACKAGE_ID}.yaml").write_text(
                f"""\
PackageIdentifier: {WINGET_PACKAGE_ID}
PackageVersion: 1.1.2
Installers:
  - Architecture: x64
    InstallerType: zip
    InstallerUrl: https://example.invalid/downloads/synthetic-app_1.1.2_x86_64-pc-windows-msvc.zip
ManifestType: installer
ManifestVersion: 1.1.2
""",
                encoding="utf-8",
            )
            config = {
                "winget": {
                    "enabled": True,
                    "manifest_glob": ".winget/*.yaml",
                    "package_version_field": "PackageVersion",
                    "manifest_version_field": "ManifestVersion",
                    "installer_url_field": "InstallerUrl",
                }
            }

            self.assertTrue(validate_winget_manifests(repo_root, "1.1.2", config))


if __name__ == "__main__":
    unittest.main()
