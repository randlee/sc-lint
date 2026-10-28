from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from sc_lint.check_version_sync import main
from sc_lint.check_version_sync import success_message
from sc_lint.check_version_sync import validate_crate_versions
from sc_lint.check_version_sync import validate_lockfile
from sc_lint.check_version_sync import validate_winget_manifests


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
WORKSPACE_VERSION = "1.1.2"

ROOT_MANIFEST = f"""\
[workspace]
members = ["crates/{APP_CRATE}", "crates/{CORE_CRATE}", "crates/{DAEMON_CRATE}", "crates/{STORAGE_CRATE}"]
resolver = "2"

[workspace.package]
version = "{WORKSPACE_VERSION}"
"""


def crate_manifest(name: str, extra: str = "") -> str:
    return f"""\
[package]
name = "{name}"
version.workspace = true
edition = "2021"
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
        self.write_lockfile(repo_root)
        self.write_config(repo_root, "[version_sync]\n")

    def write_lockfile(self, repo_root: Path, storage_version: str = "1.1.2") -> None:
        packages = [(APP_PACKAGE, "1.1.2"), (CORE_PACKAGE, "1.1.2"), (DAEMON_PACKAGE, "1.1.2"), (STORAGE_PACKAGE, storage_version)]
        lock = "version = 3\n\n" + "\n\n".join(
            f'[[package]]\nname = "{name}"\nversion = "{version}"' for name, version in packages
        ) + "\n"
        (repo_root / "Cargo.lock").write_text(lock, encoding="utf-8")

    def write_config(self, repo_root: Path, config: str, *, primary: bool = False) -> Path:
        path = repo_root / ("sc-lint.toml" if primary else ".just/lint-config.toml")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(config, encoding="utf-8")
        return path

    def run_main(self, repo_root: Path, *args: str) -> tuple[int, str, str]:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            code = main(["sc_lint.check_version_sync", "--root", str(repo_root), *args])
        return code, stdout.getvalue(), stderr.getvalue()

    def json_result(self, repo_root: Path, *args: str) -> tuple[int, dict, str]:
        code, stdout, stderr = self.run_main(repo_root, "--json", *args)
        return code, json.loads(stdout), stderr

    @staticmethod
    def fixture_path(name: str) -> Path:
        return Path(__file__).parent / "fixtures" / "version_sync" / name

    def install_fixture(self, repo_root: Path, config_fixture: str, files: list[tuple[str, str]]) -> None:
        self.write_repo(repo_root)
        config_source = self.fixture_path(config_fixture)
        self.write_config(repo_root, config_source.read_text(encoding="utf-8"))
        for fixture_name, destination in files:
            path = repo_root / destination
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(self.fixture_path(fixture_name).read_text(encoding="utf-8"), encoding="utf-8")

    def test_validate_crate_versions_aggregates_invalid_versions_and_path_pins(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            storage_manifest = root / "crates" / STORAGE_CRATE / "Cargo.toml"
            storage_manifest.write_text(storage_manifest.read_text().replace("version.workspace = true\n", ""), encoding="utf-8")
            app_manifest = root / "crates" / APP_CRATE / "Cargo.toml"
            app_manifest.write_text(app_manifest.read_text().replace('version = "1.1.2"', 'version = "1.1.3"'), encoding="utf-8")

            findings = validate_crate_versions(root, WORKSPACE_VERSION)

            self.assertEqual(
                [finding.to_dict() for finding in findings],
                [
                    {
                        "check": "crate_version",
                        "file": f"crates/{STORAGE_CRATE}/Cargo.toml",
                        "expected": "non-empty string or version.workspace = true",
                        "actual": "absent",
                    },
                    {
                        "check": "internal_path_dep",
                        "file": f"crates/{APP_CRATE}/Cargo.toml",
                        "expected": "dependencies.fixture-lib version 1.1.2",
                        "actual": "1.1.3",
                    },
                ],
            )

    def test_validate_lockfile_reports_each_missing_or_mismatched_package(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_lockfile(root, storage_version="1.1.3")

            findings, skipped = validate_lockfile(root, WORKSPACE_VERSION)

            self.assertEqual(skipped, [])
            self.assertEqual(
                [finding.to_dict() for finding in findings],
                [{"check": "cargo_lock", "file": "Cargo.lock", "expected": "synthetic-storage 1.1.2", "actual": "synthetic-storage 1.1.3"}],
            )

    def test_validate_lockfile_reports_workspace_package_absent_from_lock(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            (root / "Cargo.lock").write_text(
                'version = 3\n\n'
                f'[[package]]\nname = "{APP_PACKAGE}"\nversion = "1.1.2"\n\n'
                f'[[package]]\nname = "{CORE_PACKAGE}"\nversion = "1.1.2"\n\n'
                f'[[package]]\nname = "{DAEMON_PACKAGE}"\nversion = "1.1.2"\n',
                encoding="utf-8",
            )

            findings, skipped = validate_lockfile(root, WORKSPACE_VERSION)

            self.assertEqual(skipped, [])
            self.assertEqual(
                [finding.to_dict() for finding in findings],
                [{"check": "cargo_lock", "file": "Cargo.lock", "expected": "synthetic-storage 1.1.2", "actual": "synthetic-storage absent"}],
            )

    def test_success_message_includes_workspace_version(self) -> None:
        self.assertEqual(
            success_message(WORKSPACE_VERSION, ["workspace member versions", "internal path deps", "Cargo.lock"]),
            "version sync check passed: workspace_version=1.1.2; workspace member versions, internal path deps, Cargo.lock are aligned.",
        )

    def test_version_sync_json_output_shape(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            manifest = root / "crates" / STORAGE_CRATE / "Cargo.toml"
            manifest.write_text(manifest.read_text().replace("version.workspace = true\n", ""), encoding="utf-8")

            code, payload, stderr = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertEqual(stderr, "")
            self.assertEqual(payload["adapter_schema"], "sc-lint-python-v1")
            self.assertTrue(payload["ok"])
            self.assertEqual(set(payload["data"]), {"ok", "findings", "skipped", "workspace_version"})
            self.assertFalse(payload["data"]["ok"])
            self.assertEqual(payload["data"]["workspace_version"], WORKSPACE_VERSION)
            self.assertEqual(set(payload["data"]["findings"][0]), {"check", "file", "expected", "actual"})
            self.assertEqual(payload["data"]["findings"][0]["check"], "crate_version")
            self.assertEqual(payload["data"]["findings"][0]["actual"], "absent")

    def test_version_sync_root_flag(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            cwd = Path.cwd()

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 0)
            self.assertEqual(payload["data"]["workspace_version"], WORKSPACE_VERSION)
            self.assertEqual(Path.cwd(), cwd)

    def test_version_sync_config_flag(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_config(root, "[version_sync]\n", primary=True)
            pyproject = root / "py" / "pyproject.toml"
            pyproject.parent.mkdir()
            pyproject.write_text('[project]\nversion = "1.1.3"\n', encoding="utf-8")
            explicit = root / "custom.toml"
            explicit.write_text('[version_sync.python]\npython_manifests = ["py/pyproject.toml"]\n', encoding="utf-8")

            default_code, default_payload, _ = self.json_result(root)
            explicit_code, explicit_payload, _ = self.json_result(root, "--config", str(explicit))

            self.assertEqual(default_code, 0)
            self.assertEqual(default_payload["data"]["findings"], [])
            self.assertEqual(explicit_code, 1)
            self.assertEqual(explicit_payload["data"]["findings"], [
                {"check": "python.version", "file": "py/pyproject.toml", "expected": WORKSPACE_VERSION, "actual": "1.1.3"}
            ])

    def test_version_sync_text_mode_output(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            code, stdout, stderr = self.run_main(root)
            self.assertEqual(code, 0)
            self.assertIn("version sync check passed:", stdout)
            self.assertEqual(stderr, "")

            manifest = root / "crates" / STORAGE_CRATE / "Cargo.toml"
            manifest.write_text(manifest.read_text().replace("version.workspace = true\n", ""), encoding="utf-8")
            code, stdout, stderr = self.run_main(root)
            self.assertEqual(code, 1)
            self.assertEqual(stdout, "")
            self.assertIn(
                f"crate_version crates/{STORAGE_CRATE}/Cargo.toml: expected non-empty string or version.workspace = true, actual absent",
                stderr,
            )

    def test_version_sync_malformed_inputs_are_config_errors(self) -> None:
        for case in ("cargo_toml", "missing_version", "cargo_lock"):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as tempdir:
                root = Path(tempdir)
                self.write_repo(root)
                if case == "cargo_toml":
                    (root / "Cargo.toml").write_text("invalid = [\n", encoding="utf-8")
                elif case == "missing_version":
                    (root / "Cargo.toml").write_text(ROOT_MANIFEST.replace('version = "1.1.2"\n', ""), encoding="utf-8")
                else:
                    (root / "Cargo.lock").write_text("invalid = [\n", encoding="utf-8")

                code, payload, _ = self.json_result(root)

                self.assertEqual(code, 3)
                self.assertFalse(payload["ok"])
                self.assertEqual(payload["error"]["kind"], "config")
                self.assertNotIn("code", payload["error"].get("details", {}))

    def test_version_sync_python_table(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            pyproject = root / "py" / "pyproject.toml"
            pyproject.parent.mkdir()
            pyproject.write_text('[project]\nversion = "1.1.2"\n', encoding="utf-8")
            self.write_config(root, '[version_sync.python]\npython_manifests = ["py/pyproject.toml"]\n')
            clean_code, clean_payload, _ = self.json_result(root)
            pyproject.write_text('[project]\nversion = "1.1.3"\n', encoding="utf-8")
            mismatch_code, mismatch_payload, _ = self.json_result(root)

            self.assertEqual(clean_code, 0)
            self.assertEqual(clean_payload["data"]["findings"], [])
            self.assertEqual(mismatch_code, 1)
            self.assertEqual(mismatch_payload["data"]["findings"], [
                {"check": "python.version", "file": "py/pyproject.toml", "expected": WORKSPACE_VERSION, "actual": "1.1.3"}
            ])

    def test_version_sync_python_dynamic_and_absent_version(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            pyproject = root / "py" / "pyproject.toml"
            pyproject.parent.mkdir()
            self.write_config(root, '[version_sync.python]\npython_manifests = ["py/pyproject.toml"]\n')
            cases = (
                ('[project]\ndynamic = ["version"]\n', "dynamic"),
                ('[project]\nname = "example"\n', "absent"),
            )
            for content, actual in cases:
                pyproject.write_text(content, encoding="utf-8")
                code, payload, _ = self.json_result(root)
                self.assertEqual(code, 1)
                self.assertIn(
                    {"check": "python.version", "file": "py/pyproject.toml", "expected": WORKSPACE_VERSION, "actual": actual},
                    payload["data"]["findings"],
                )

    def test_version_sync_python_manifest_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_config(root, '[version_sync.python]\npython_manifests = ["py/missing.toml"]\n')

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertEqual(payload["data"]["findings"], [
                {"check": "python.manifest_missing", "file": "py/missing.toml", "expected": "file exists", "actual": "absent"}
            ])

    def test_version_sync_python_non_string_version_is_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            pyproject = root / "py" / "pyproject.toml"
            pyproject.parent.mkdir()
            pyproject.write_text('[project]\nversion = 42\n', encoding="utf-8")
            self.write_config(root, '[version_sync.python]\npython_manifests = ["py/pyproject.toml"]\n')

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertEqual(payload["data"]["findings"], [
                {"check": "python.version", "file": "py/pyproject.toml", "expected": WORKSPACE_VERSION, "actual": "invalid"}
            ])

    def test_version_sync_python_table_config_errors(self) -> None:
        configurations = (
            '[version_sync.python]\npython_manifests = "x"\n',
            '[version_sync.python]\ncargo_lock = "yes"\n',
            '[version_sync.python]\npython_manifests = ["../x/pyproject.toml"]\n',
            '[version_sync.python]\npython_manifests = ["/tmp/x/pyproject.toml"]\n',
        )
        for config in configurations:
            with self.subTest(config=config), tempfile.TemporaryDirectory() as tempdir:
                root = Path(tempdir)
                self.write_repo(root)
                self.write_config(root, config)
                code, payload, _ = self.json_result(root)
                self.assertEqual(code, 3)
                self.assertEqual(payload["error"]["kind"], "config")
                self.assertNotIn("code", payload["error"].get("details", {}))

        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_config(root, '[version_sync.python]\npython_manifests = ["pyproject.toml"]\n')
            (root / "pyproject.toml").write_text("[project\n", encoding="utf-8")
            code, payload, _ = self.json_result(root)
            self.assertEqual(code, 3)
            self.assertEqual(payload["error"]["kind"], "config")
            self.assertNotIn("code", payload["error"].get("details", {}))

    def test_version_sync_without_workspace_members_is_config_error(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = []\n[workspace.package]\nversion = "1.1.2"\n',
                encoding="utf-8",
            )

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 3)
            self.assertEqual(payload["error"]["kind"], "config")
            self.assertEqual(payload["error"]["message"], "no workspace member manifests found")

    def test_version_sync_unexpected_error_uses_backend_protocol_envelope(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            with patch("sc_lint.check_version_sync._evaluate", side_effect=RuntimeError("unexpected")):
                code, payload, _ = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertFalse(payload["ok"])
            self.assertEqual(payload["error"]["kind"], "backend_protocol")
            self.assertIn("unexpected", payload["error"]["message"])

    def test_version_sync_without_cargo_lock(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            (root / "Cargo.lock").unlink()

            code, payload, _ = self.json_result(root)
            text_code, stdout, _ = self.run_main(root)

            self.assertEqual(code, 0)
            self.assertEqual(payload["data"]["skipped"], [{"check": "cargo_lock", "file": "Cargo.lock", "reason": "missing"}])
            self.assertEqual(text_code, 0)
            self.assertIn("skipped: Cargo.lock (missing)", stdout)

    def test_version_sync_cargo_lock_disabled(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_lockfile(root, storage_version="9.9.9")
            self.write_config(root, "[version_sync.python]\ncargo_lock = false\n")

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 0)
            self.assertEqual(payload["data"]["findings"], [])
            self.assertEqual(payload["data"]["skipped"], [])

    def test_version_sync_winget_subsection(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.install_fixture(root, "winget.lint-config.toml", [("winget-sample.yaml", ".winget/sample.yaml")])
            clean_code, clean_payload, _ = self.json_result(root)
            sample = root / ".winget/sample.yaml"

            original = sample.read_text(encoding="utf-8")
            sample.write_text(original.replace("PackageVersion: 1.1.2", "PackageVersion: 1.1.3"), encoding="utf-8")
            package_code, package_payload, _ = self.json_result(root)
            sample.write_text(original.replace("/download/v1.1.2/", "/download/v1.1.3/"), encoding="utf-8")
            url_code, url_payload, _ = self.json_result(root)
            sample.write_text(original.replace("ManifestVersion: 1.1.2\n", ""), encoding="utf-8")
            missing_code, missing_payload, _ = self.json_result(root)

            self.assertEqual(clean_code, 0)
            self.assertTrue(clean_payload["data"]["ok"])
            self.assertEqual(package_code, 1)
            self.assertIn({"check": "winget.package_version", "file": ".winget/sample.yaml", "expected": "PackageVersion 1.1.2", "actual": "PackageVersion 1.1.3"}, package_payload["data"]["findings"])
            self.assertEqual(url_code, 1)
            self.assertIn({"check": "winget.installer_url", "file": ".winget/sample.yaml", "expected": "InstallerUrl download version 1.1.2", "actual": "https://example.invalid/download/v1.1.3/app.zip"}, url_payload["data"]["findings"])
            self.assertEqual(missing_code, 1)
            self.assertIn({"check": "winget.missing_field", "file": ".winget/sample.yaml", "expected": "ManifestVersion", "actual": "absent"}, missing_payload["data"]["findings"])

    def test_version_sync_winget_manifest_glob_matches_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.install_fixture(root, "winget.lint-config.toml", [("winget-sample.yaml", ".winget/sample.yml")])

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertEqual(payload["data"]["findings"], [
                {"check": "winget.manifest_glob", "file": ".winget/*.yaml", "expected": "at least one manifest", "actual": "none"}
            ])

    def test_version_sync_release_wiring_subsection(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.install_fixture(root, "release_wiring.lint-config.toml", [("release-sample.yml", ".github/workflows/release.yml")])
            code, payload, _ = self.json_result(root)
            workflow = root / ".github/workflows/release.yml"
            workflow.write_text(workflow.read_text(encoding="utf-8").replace("release-target-matrix\n", ""), encoding="utf-8")
            missing_code, missing_payload, _ = self.json_result(root)

            self.assertEqual(code, 0)
            self.assertEqual(payload["data"]["findings"], [])
            self.assertEqual(missing_code, 1)
            self.assertIn({"check": "release_wiring.fragment", "file": ".github/workflows/release.yml", "expected": "release-target-matrix", "actual": "absent"}, missing_payload["data"]["findings"])

    def test_version_sync_release_wiring_file_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.install_fixture(root, "release_wiring.lint-config.toml", [])

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertEqual(payload["data"]["findings"], [
                {"check": "release_wiring.file_missing", "file": ".github/workflows/release.yml", "expected": "file exists", "actual": "absent"}
            ])

    def test_version_sync_reads_table_from_fallback_config(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_config(root, "[other]\nvalue = true\n", primary=True)
            self.write_config(root, '[version_sync.python]\npython_manifests = ["py/pyproject.toml"]\n')
            pyproject = root / "py/pyproject.toml"
            pyproject.parent.mkdir()
            pyproject.write_text('[project]\nversion = "1.1.3"\n', encoding="utf-8")

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 1)
            self.assertEqual(payload["data"]["findings"], [
                {"check": "python.version", "file": "py/pyproject.toml", "expected": WORKSPACE_VERSION, "actual": "1.1.3"}
            ])

    def test_version_sync_table_in_neither_file_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            self.write_repo(root)
            self.write_config(root, "[other]\nvalue = true\n", primary=True)
            self.write_config(root, "[other]\nvalue = true\n")

            code, payload, _ = self.json_result(root)

            self.assertEqual(code, 3)
            self.assertEqual(payload["error"]["kind"], "config")
            self.assertEqual(payload["error"]["details"]["code"], "config.table_missing")
            self.assertEqual(payload["error"]["details"]["table"], "version_sync")
            self.assertEqual(payload["error"]["details"]["searched"], ["sc-lint.toml", ".just/lint-config.toml"])

    def test_version_sync_non_table_subsections_are_config_errors(self) -> None:
        for table in ("python", "winget", "release_wiring"):
            with self.subTest(table=table), tempfile.TemporaryDirectory() as tempdir:
                root = Path(tempdir)
                self.write_repo(root)
                self.write_config(root, f"[version_sync]\n{table} = 'invalid'\n")
                code, payload, _ = self.json_result(root)
                self.assertEqual(code, 3)
                self.assertEqual(payload["error"]["kind"], "config")

    def test_version_sync_rejects_invalid_winget_and_release_paths(self) -> None:
        configurations = (
            '[version_sync.winget]\nenabled = true\nmanifest_glob = "../.winget/*.yaml"\n',
            '[version_sync.release_wiring]\nenabled = true\nfile = "../release.yml"\n',
            '[version_sync.release_wiring]\nenabled = true\nfile = "workflow.yml"\nrequired_fragments = "not-an-array"\n',
        )
        for config in configurations:
            with self.subTest(config=config), tempfile.TemporaryDirectory() as tempdir:
                root = Path(tempdir)
                self.write_repo(root)
                self.write_config(root, config)
                code, payload, _ = self.json_result(root)
                self.assertEqual(code, 3)
                self.assertEqual(payload["error"]["kind"], "config")
                self.assertNotIn("code", payload["error"].get("details", {}))

    def test_version_sync_existing_enabled_checks_return_finding_tuples(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / ".winget").mkdir(parents=True)
            (root / ".winget/app.yaml").write_text(
                "PackageVersion: 1.1.2\nInstallerUrl: https://example.invalid/download/v1.1.2/app.zip\nManifestVersion: 1.1.2\n",
                encoding="utf-8",
            )
            ran, findings = validate_winget_manifests(
                root,
                WORKSPACE_VERSION,
                {"winget": {"enabled": True, "manifest_glob": ".winget/*.yaml"}},
            )
            self.assertTrue(ran)
            self.assertEqual(findings, [])


if __name__ == "__main__":
    unittest.main()
