from __future__ import annotations

import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from unittest.mock import patch


from sc_lint import lint_cargo_deny
from sc_lint import lint_cargo_shear
from sc_lint import lint_codespell
from sc_lint.lint_common import WorkspaceCrate
from sc_lint.lint_cargo_deny import build_command as build_cargo_deny_command
from sc_lint.lint_cargo_deny import build_runtime_config
from sc_lint.lint_cargo_deny import run_cargo_deny
from sc_lint.lint_cargo_shear import annotate_sections
from sc_lint.lint_cargo_shear import build_command as build_cargo_shear_command
from sc_lint.lint_cargo_shear import CargoShearWorkspaceMetadata
from sc_lint.lint_cargo_shear import evaluate_policy
from sc_lint.lint_cargo_shear import evaluate_unused_dependency_policy
from sc_lint.lint_cargo_shear import load_cargo_shear_workspace_metadata
from sc_lint.lint_cargo_shear import load_policy_config
from sc_lint.lint_cargo_shear import manifest_to_package_name
from sc_lint.lint_cargo_shear import main as cargo_shear_main
from sc_lint.lint_cargo_shear import parse_sections
from sc_lint.lint_cargo_shear import PATH_KEY_MIGRATION_DIAGNOSTIC
from sc_lint.lint_cargo_shear import validate_unused_dependency_allowlist
from sc_lint.lint_codespell import build_command as build_codespell_command
from sc_lint.lint_common import write_console_output
from sc_lint.lint_common import workspace_crate_section_lines

APP_CRATE = "fixture-app"
CORE_CRATE = "fixture-lib"
APP_PACKAGE = "synthetic-app"
CORE_PACKAGE = "synthetic-lib"


def create_non_ascii_workspace(repo_root: Path) -> None:
    crate_dir = repo_root / "crates" / "λapp"
    crate_dir.mkdir(parents=True)
    (repo_root / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/λapp"]\nresolver = "2"\n',
        encoding="utf-8",
    )
    (crate_dir / "Cargo.toml").write_text(
        '[package]\nname = "synthetic-app"\nversion = "0.1.0"\n',
        encoding="utf-8",
    )


class RestrictiveEncodingStream(io.StringIO):
    encoding = "ascii"
    errors = "strict"

    def write(self, text: str) -> int:
        text.encode(self.encoding, errors=self.errors)
        return super().write(text)


class Cp1252RestrictiveEncodingStream(RestrictiveEncodingStream):
    encoding = "cp1252"


class ExternalLintToolTests(unittest.TestCase):
    def test_build_cargo_deny_command_uses_pre_020_option_position(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            config_path = Path(tempdir) / "deny.runtime.toml"
            self.assertEqual(
                build_cargo_deny_command(config_path, (0, 19)),
                [
                    "cargo-deny",
                    "check",
                    "--config",
                    str(config_path),
                    "advisories",
                    "bans",
                    "licenses",
                    "sources",
                ],
            )

    def test_build_cargo_deny_command_uses_020_global_option_position(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            config_path = Path(tempdir) / "deny.runtime.toml"
            self.assertEqual(
                build_cargo_deny_command(config_path, (0, 20)),
                [
                    "cargo-deny",
                    "--config",
                    str(config_path),
                    "check",
                    "advisories",
                    "bans",
                    "licenses",
                    "sources",
                ],
            )

    def test_build_runtime_config_strips_deprecated_keys(self) -> None:
        with (
            tempfile.TemporaryDirectory() as tempdir,
            tempfile.TemporaryDirectory(prefix="atm-lint-deny-") as runtime_dir,
        ):
            repo_root = Path(tempdir)
            (repo_root / "deny.toml").write_text(
                """\
[advisories]
vulnerability = "deny"
yanked = "deny"

[licenses]
unlicensed = "deny"
allow = ["MIT"]
""",
                encoding="utf-8",
            )

            with patch(
                "sc_lint.lint_cargo_deny.tempfile.mkdtemp",
                return_value=runtime_dir,
            ):
                runtime_path = build_runtime_config(repo_root)
            text = runtime_path.read_text(encoding="utf-8")

            self.assertNotIn('vulnerability = "deny"', text)
            self.assertNotIn('unlicensed = "deny"', text)
            self.assertIn('yanked = "deny"', text)
            self.assertIn('allow = ["MIT"]', text)

    def test_cargo_deny_retries_dns_resolution_failure(self) -> None:
        failure = subprocess.CompletedProcess(
            ["cargo-deny"], 1, stdout="Could NOT resolve hostname", stderr=""
        )
        success = subprocess.CompletedProcess(["cargo-deny"], 0, stdout="ok", stderr="")
        runner = mock.Mock(side_effect=[failure, success])
        sleeper = mock.Mock()
        with patch("sc_lint.lint_cargo_deny.sys.stderr", new_callable=io.StringIO):
            completed = run_cargo_deny(
                ["cargo-deny", "check"], Path("/repo"), runner=runner, sleeper=sleeper
            )

        self.assertEqual(completed.returncode, 0)
        self.assertEqual(runner.call_count, 2)
        sleeper.assert_called_once_with(2)

    def test_cargo_deny_does_not_retry_policy_failure(self) -> None:
        failure = subprocess.CompletedProcess(
            ["cargo-deny"], 1, stdout="", stderr="error: failed license policy"
        )
        runner = mock.Mock(return_value=failure)
        sleeper = mock.Mock()

        completed = run_cargo_deny(
            ["cargo-deny", "check"], Path("/repo"), runner=runner, sleeper=sleeper
        )

        self.assertEqual(completed.returncode, 1)
        runner.assert_called_once()
        sleeper.assert_not_called()

    def test_cargo_deny_reports_each_retry_and_preserves_final_output(self) -> None:
        dns_failures = [
            subprocess.CompletedProcess(
                ["cargo-deny"], 1, stdout="Could not resolve hostname", stderr=""
            ),
            subprocess.CompletedProcess(
                ["cargo-deny"], 1, stdout="", stderr="could not resolve HOSTNAME"
            ),
        ]
        success = subprocess.CompletedProcess(
            ["cargo-deny"], 0, stdout="final stdout\n", stderr="final stderr\n"
        )
        runner = mock.Mock(side_effect=[*dns_failures, success])
        sleeper = mock.Mock()
        stderr = io.StringIO()

        with patch("sc_lint.lint_cargo_deny.sys.stderr", stderr):
            completed = run_cargo_deny(
                ["cargo-deny", "check"], Path("/repo"), runner=runner, sleeper=sleeper
            )

        self.assertEqual(completed.returncode, 0)
        self.assertEqual(completed.stdout, "final stdout\n")
        self.assertEqual(completed.stderr, "final stderr\n")
        self.assertEqual(
            stderr.getvalue(),
            "cargo-deny DNS failure (attempt 1/3); retrying in 2s\n"
            "cargo-deny DNS failure (attempt 2/3); retrying in 2s\n",
        )
        self.assertEqual(runner.call_count, 3)
        self.assertEqual([call.args for call in sleeper.call_args_list], [(2,), (2,)])

    def test_cargo_deny_main_retries_dns_failure_and_forwards_only_final_result(self) -> None:
        dns_failure = subprocess.CompletedProcess(
            ["cargo-deny"], 1, "intermediate DNS output\ncould not resolve hostname\n", ""
        )
        final = subprocess.CompletedProcess(
            ["cargo-deny"], 7, "final stdout\n", "final stderr\n"
        )
        version = subprocess.CompletedProcess(
            ["cargo-deny", "--version"], 0, "cargo-deny 0.20.0\n", ""
        )
        stdout = io.StringIO()
        stderr = io.StringIO()

        with (
            patch("sc_lint.lint_cargo_deny.discover_repo_root", return_value=Path("/repo")),
            patch("sc_lint.lint_cargo_deny.shutil.which", return_value="cargo-deny"),
            patch("sc_lint.lint_cargo_deny.workspace_crate_section_lines", return_value=["workspace crates:"]),
            patch("sc_lint.lint_cargo_deny.build_runtime_config", return_value=Path("/repo/deny.toml")),
            patch("sc_lint.lint_cargo_deny.subprocess.run", side_effect=[version, dns_failure, final]) as run_mock,
            patch("sc_lint.lint_cargo_deny.time.sleep") as sleep_mock,
            patch("sc_lint.lint_cargo_deny.sys.stdout", stdout),
            patch("sc_lint.lint_cargo_deny.sys.stderr", stderr),
        ):
            result = lint_cargo_deny.main(["lint-cargo-deny", "--root", "/repo"])

        self.assertEqual(result, 7)
        self.assertEqual(stdout.getvalue(), "workspace crates:\nfinal stdout\n")
        self.assertEqual(
            stderr.getvalue(),
            "cargo-deny DNS failure (attempt 1/3); retrying in 2s\nfinal stderr\n",
        )
        self.assertEqual(run_mock.call_count, 3)
        sleep_mock.assert_called_once_with(2)

    def test_cargo_deny_stops_after_three_dns_failures(self) -> None:
        failure = subprocess.CompletedProcess(
            ["cargo-deny"], 9, "", "could not resolve hostname\n"
        )
        runner = mock.Mock(side_effect=[failure, failure, failure])
        sleeper = mock.Mock()
        stderr = io.StringIO()

        with patch("sc_lint.lint_cargo_deny.sys.stderr", stderr):
            completed = run_cargo_deny(
                ["cargo-deny", "check"], Path("/repo"), runner=runner, sleeper=sleeper
            )

        self.assertEqual(completed.returncode, 9)
        self.assertEqual(runner.call_count, 3)
        self.assertEqual(sleeper.call_count, 2)
        self.assertEqual(
            stderr.getvalue(),
            "cargo-deny DNS failure (attempt 1/3); retrying in 2s\n"
            "cargo-deny DNS failure (attempt 2/3); retrying in 2s\n",
        )

    def test_cargo_deny_mixed_dns_then_policy_stops(self) -> None:
        dns_failure = subprocess.CompletedProcess(
            ["cargo-deny"], 1, stdout="could not resolve hostname", stderr=""
        )
        policy_failure = subprocess.CompletedProcess(
            ["cargo-deny"], 7, stdout="final policy failure\n", stderr="denied\n"
        )
        runner = mock.Mock(side_effect=[dns_failure, policy_failure])
        sleeper = mock.Mock()
        stderr = io.StringIO()

        with patch("sc_lint.lint_cargo_deny.sys.stderr", stderr):
            completed = run_cargo_deny(
                ["cargo-deny", "check"], Path("/repo"), runner=runner, sleeper=sleeper
            )

        self.assertEqual(completed.returncode, 7)
        self.assertEqual(completed.stdout, "final policy failure\n")
        self.assertEqual(completed.stderr, "denied\n")
        self.assertEqual(runner.call_count, 2)
        sleeper.assert_called_once_with(2)
        self.assertEqual(
            stderr.getvalue(),
            "cargo-deny DNS failure (attempt 1/3); retrying in 2s\n",
        )

    def test_build_cargo_shear_command_targets_workspace_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.assertEqual(
                build_cargo_shear_command(repo_root),
                ["cargo-shear"],
            )

    def test_cargo_shear_metadata_loads_one_offline_no_deps_query_and_dependency_keys(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/app"]\n'
                '[workspace.dependencies]\nworkspace_alias = { package = "real-workspace", version = "1" }\n'
                'workspace_direct = "1"\n',
                encoding="utf-8",
            )
            app_manifest = repo_root / "crates" / "app" / "Cargo.toml"
            app_manifest.parent.mkdir(parents=True)
            app_manifest.write_text("[package]\nname = \"fixture-app\"\n", encoding="utf-8")
            metadata_json = {
                "packages": [
                    {
                        "name": "fixture-app",
                        "manifest_path": str(app_manifest),
                        "dependencies": [
                            {"name": "normal-dep", "rename": None, "kind": None},
                            {"name": "real-package", "rename": "dependency_alias", "kind": None},
                            {"name": "dev-dep", "rename": None, "kind": "dev"},
                            {"name": "build-dep", "rename": None, "kind": "build"},
                            {
                                "name": "target-dep",
                                "rename": "target_alias",
                                "kind": None,
                                "target": "cfg(windows)",
                            },
                        ],
                    }
                ]
            }
            metadata_result = subprocess.CompletedProcess(
                ["cargo", "metadata"], 0, json.dumps(metadata_json), ""
            )
            with patch(
                "sc_lint.lint_cargo_shear.subprocess.run", return_value=metadata_result
            ) as run_mock:
                metadata = load_cargo_shear_workspace_metadata(repo_root)

        run_mock.assert_called_once_with(
            ["cargo", "metadata", "--offline", "--format-version", "1", "--no-deps"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
        self.assertEqual(
            metadata.allowed_unused_dependency_keys,
            frozenset(
                {
                    "fixture-app.normal-dep",
                    "fixture-app.dependency_alias",
                    "fixture-app.dev-dep",
                    "fixture-app.build-dep",
                    "fixture-app.target_alias",
                    "workspace.workspace_alias",
                    "workspace.workspace_direct",
                }
            ),
        )
        self.assertEqual(
            metadata.manifest_packages["crates/app/Cargo.toml"], "fixture-app"
        )
        self.assertEqual(metadata.manifest_packages["Cargo.toml"], "workspace")

    def test_cargo_shear_package_named_workspace_does_not_get_virtual_root_keys(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                '[package]\nname = "workspace"\nversion = "0.1.0"\n'
                '[workspace]\nmembers = []\n'
                '[workspace.dependencies]\nvirtual_only = "1"\n',
                encoding="utf-8",
            )
            manifest = str(repo_root / "Cargo.toml")
            metadata_json = {
                "packages": [
                    {
                        "name": "workspace",
                        "manifest_path": manifest,
                        "dependencies": [{"name": "serde", "rename": None}],
                    }
                ]
            }
            completed = subprocess.CompletedProcess(
                ["cargo", "metadata"], 0, json.dumps(metadata_json), ""
            )
            with patch(
                "sc_lint.lint_cargo_shear.subprocess.run", return_value=completed
            ):
                metadata = load_cargo_shear_workspace_metadata(repo_root)

        self.assertIn("workspace.serde", metadata.allowed_unused_dependency_keys)
        self.assertNotIn("workspace.virtual_only", metadata.allowed_unused_dependency_keys)
        self.assertEqual(metadata.manifest_packages["Cargo.toml"], "workspace")

    def test_cargo_shear_metadata_collects_dependencies_for_out_of_root_members(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir, tempfile.TemporaryDirectory() as member_tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                "[workspace]\nmembers = []\nresolver = \"2\"\n",
                encoding="utf-8",
            )
            member_manifest = Path(member_tempdir) / "member" / "Cargo.toml"
            completed = subprocess.CompletedProcess(
                ["cargo", "metadata"],
                0,
                json.dumps(
                    {
                        "packages": [
                            {
                                "name": "external-member",
                                "manifest_path": str(member_manifest),
                                "dependencies": [
                                    {"name": "serde", "rename": None, "kind": None}
                                ],
                            }
                        ]
                    }
                ),
                "",
            )
            with patch(
                "sc_lint.lint_cargo_shear.subprocess.run", return_value=completed
            ):
                metadata = load_cargo_shear_workspace_metadata(repo_root)

        self.assertIn("external-member.serde", metadata.allowed_unused_dependency_keys)
        self.assertNotIn(member_manifest.as_posix(), metadata.manifest_packages)

    def test_cargo_shear_virtual_root_workspace_member_collision_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/workspace"]\n'
                '[workspace.dependencies]\nshared = "1"\nroot_only = "1"\n',
                encoding="utf-8",
            )
            member_manifest = repo_root / "crates" / "workspace" / "Cargo.toml"
            member_manifest.parent.mkdir(parents=True)
            member_manifest.write_text(
                '[package]\nname = "workspace"\nversion = "0.1.0"\n',
                encoding="utf-8",
            )
            metadata_json = {
                "packages": [
                    {
                        "name": "workspace",
                        "manifest_path": str(member_manifest),
                        "dependencies": [
                            {"name": "shared", "rename": None},
                            {"name": "member_only", "rename": None},
                        ],
                    }
                ]
            }
            completed = subprocess.CompletedProcess(
                ["cargo", "metadata"], 0, json.dumps(metadata_json), ""
            )
            with patch(
                "sc_lint.lint_cargo_shear.subprocess.run", return_value=completed
            ):
                metadata = load_cargo_shear_workspace_metadata(repo_root)

        self.assertEqual(
            metadata.ambiguous_unused_dependency_keys,
            frozenset({"workspace.shared"}),
        )
        self.assertIn("workspace.root_only", metadata.allowed_unused_dependency_keys)
        self.assertIn("workspace.member_only", metadata.allowed_unused_dependency_keys)
        diagnostics = validate_unused_dependency_allowlist(
            {"workspace.shared": "either package or virtual-root dependency"}, metadata
        )
        self.assertEqual(len(diagnostics), 1)
        self.assertIn("cause=ambiguous-dependency-key", diagnostics[0])
        self.assertIn("virtual workspace root", diagnostics[0])
        self.assertIn("package named workspace", diagnostics[0])

    def test_cargo_shear_validation_reports_exact_path_key_migration(self) -> None:
        metadata = CargoShearWorkspaceMetadata(
            {"crates/app/Cargo.toml": APP_PACKAGE},
            frozenset({f"{APP_PACKAGE}.serde"}),
        )
        key = "crates/app/Cargo.toml.serde"

        diagnostics = validate_unused_dependency_allowlist({key: "old path form"}, metadata)

        self.assertEqual(len(diagnostics), 2)
        self.assertIn("SC_LINT_CARGO_SHEAR_CONFIG_INVALID", diagnostics[0])
        self.assertIn("field=cargo_shear.allowed_unused_dependencies", diagnostics[0])
        self.assertIn("key=", diagnostics[0])
        self.assertIn("cause=path-like-key", diagnostics[0])
        self.assertNotIn("action=", diagnostics[0])
        self.assertEqual(diagnostics[1], PATH_KEY_MIGRATION_DIAGNOSTIC.format(key=key))

    def test_cargo_shear_validation_rejects_stale_keys_with_correction(self) -> None:
        metadata = CargoShearWorkspaceMetadata({}, frozenset({f"{APP_PACKAGE}.serde"}))

        diagnostics = validate_unused_dependency_allowlist(
            {f"{APP_PACKAGE}.missing": "stale"}, metadata
        )

        self.assertEqual(len(diagnostics), 1)
        self.assertIn("SC_LINT_CARGO_SHEAR_CONFIG_INVALID", diagnostics[0])
        self.assertIn("cause=dependency-key-not-found", diagnostics[0])
        self.assertIn("action=", diagnostics[0])

    def test_cargo_shear_validation_rejects_blank_and_non_string_reasons(self) -> None:
        metadata = CargoShearWorkspaceMetadata(
            {}, frozenset({f"{APP_PACKAGE}.serde", f"{APP_PACKAGE}.regex"})
        )

        diagnostics = validate_unused_dependency_allowlist(
            {f"{APP_PACKAGE}.serde": "   ", f"{APP_PACKAGE}.regex": 3}, metadata
        )

        self.assertEqual(len(diagnostics), 2)
        self.assertIn("key='synthetic-app.serde' cause=blank-reason", diagnostics[0])
        self.assertIn("key='synthetic-app.regex' cause=non-string-reason", diagnostics[1])
        self.assertTrue(
            all('action="supply a nonblank string reason"' in diagnostic for diagnostic in diagnostics)
        )

    def test_cargo_shear_validation_accepts_only_declared_canonical_keys(self) -> None:
        metadata = CargoShearWorkspaceMetadata(
            {}, frozenset({f"{APP_PACKAGE}.serde", "workspace.shared"})
        )

        self.assertEqual(
            validate_unused_dependency_allowlist(
                {f"{APP_PACKAGE}.serde": "normal", "workspace.shared": "root"}, metadata
            ),
            [],
        )

    def test_cargo_shear_absent_allowlist_preserves_child_status_without_metadata(self) -> None:
        child = subprocess.CompletedProcess(
            ["cargo-shear"], 7, "cargo-shear failed\n", "tool failure\n"
        )
        stdout = io.StringIO()
        stderr = io.StringIO()
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text(
                "[cargo_shear.allowed_empty_files]\n", encoding="utf-8"
            )
            with (
                patch("sc_lint.lint_cargo_shear.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[]),
                patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear"),
                patch("sc_lint.lint_cargo_shear.subprocess.run", return_value=child) as run_mock,
                patch(
                    "sc_lint.lint_cargo_shear.load_cargo_shear_workspace_metadata",
                    side_effect=AssertionError("absent optional table must not query metadata"),
                ) as metadata_mock,
                patch("sc_lint.lint_cargo_shear.sys.stdout", stdout),
                patch("sc_lint.lint_cargo_shear.sys.stderr", stderr),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 7)
        run_mock.assert_called_once()
        metadata_mock.assert_not_called()
        self.assertEqual(stdout.getvalue(), "cargo-shear failed\n")
        self.assertEqual(stderr.getvalue(), "tool failure\n")

    def test_cargo_shear_path_key_is_rejected_before_successful_tool_run(self) -> None:
        keys = (
            "crates/old/Cargo.toml.serde",
            r"crates\old\Cargo.toml.serde",
            "./crates/old/Cargo.toml.serde",
            "~/.cache/Cargo.toml.serde",
            r"C:\crates\old\Cargo.toml.serde",
        )
        for key in keys:
            with self.subTest(key=key):
                metadata = CargoShearWorkspaceMetadata(
                    {}, frozenset({f"{APP_PACKAGE}.serde"})
                )
                stderr = io.StringIO()
                with tempfile.TemporaryDirectory() as tempdir:
                    repo_root = Path(tempdir)
                    (repo_root / ".just").mkdir()
                    (repo_root / ".just" / "lint-config.toml").write_text(
                        f"[cargo_shear.allowed_unused_dependencies]\n'{key}' = \"old path\"\n",
                        encoding="utf-8",
                    )
                    with (
                        patch(
                            "sc_lint.lint_cargo_shear.discover_repo_root",
                            return_value=repo_root,
                        ),
                        patch(
                            "sc_lint.lint_cargo_shear.load_cargo_shear_workspace_metadata",
                            return_value=metadata,
                        ),
                        patch(
                            "sc_lint.lint_cargo_shear.workspace_crate_section_lines",
                            return_value=[],
                        ),
                        patch("sc_lint.lint_cargo_shear.shutil.which") as which_mock,
                        patch("sc_lint.lint_cargo_shear.subprocess.run") as child_mock,
                        patch("sc_lint.lint_cargo_shear.sys.stderr", stderr),
                    ):
                        result = cargo_shear_main(
                            ["lint_cargo_shear.py", "--root", str(repo_root)]
                        )

                diagnostics = stderr.getvalue().splitlines()
                self.assertEqual(result, 2)
                self.assertIn("SC_LINT_CARGO_SHEAR_CONFIG_INVALID", diagnostics[0])
                self.assertIn("cause=path-like-key", diagnostics[0])
                self.assertNotIn("action=", diagnostics[0])
                self.assertEqual(
                    diagnostics[1], PATH_KEY_MIGRATION_DIAGNOSTIC.format(key=key)
                )
                which_mock.assert_not_called()
                child_mock.assert_not_called()

    def test_cargo_shear_stale_key_is_rejected_before_successful_tool_run(self) -> None:
        stale_key = f"{APP_PACKAGE}.not_declared"
        metadata = CargoShearWorkspaceMetadata({}, frozenset({f"{APP_PACKAGE}.serde"}))
        stderr = io.StringIO()
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text(
                f'[cargo_shear.allowed_unused_dependencies]\n"{stale_key}" = "stale"\n',
                encoding="utf-8",
            )
            with (
                patch("sc_lint.lint_cargo_shear.discover_repo_root", return_value=repo_root),
                patch(
                    "sc_lint.lint_cargo_shear.load_cargo_shear_workspace_metadata",
                    return_value=metadata,
                ),
                patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[]),
                patch("sc_lint.lint_cargo_shear.shutil.which") as which_mock,
                patch("sc_lint.lint_cargo_shear.subprocess.run") as child_mock,
                patch("sc_lint.lint_cargo_shear.sys.stderr", stderr),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 2)
        self.assertIn("SC_LINT_CARGO_SHEAR_CONFIG_INVALID", stderr.getvalue())
        self.assertIn(stale_key, stderr.getvalue())
        self.assertIn("cause=dependency-key-not-found", stderr.getvalue())
        self.assertIn("action=", stderr.getvalue())
        which_mock.assert_not_called()
        child_mock.assert_not_called()

    def test_cargo_shear_metadata_failure_is_actionable_config_error(self) -> None:
        stderr = io.StringIO()
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text(
                f'[cargo_shear.allowed_unused_dependencies]\n"{APP_PACKAGE}.serde" = "needed"\n',
                encoding="utf-8",
            )
            failure = subprocess.CompletedProcess(
                ["cargo", "metadata"], 101, "", "offline metadata unavailable\n"
            )
            with (
                patch("sc_lint.lint_cargo_shear.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[]),
                patch("sc_lint.lint_cargo_shear.subprocess.run", return_value=failure) as run_mock,
                patch("sc_lint.lint_cargo_shear.shutil.which") as which_mock,
                patch("sc_lint.lint_cargo_shear.sys.stderr", stderr),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 2)
        self.assertIn("SC_LINT_CARGO_SHEAR_CONFIG_INVALID", stderr.getvalue())
        self.assertIn("offline metadata unavailable", stderr.getvalue())
        self.assertIn("cargo metadata --offline --format-version 1 --no-deps", stderr.getvalue())
        run_mock.assert_called_once_with(
            ["cargo", "metadata", "--offline", "--format-version", "1", "--no-deps"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
        which_mock.assert_not_called()

    def test_cargo_shear_reuses_metadata_mapping_for_renamed_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/app"]\nresolver = "2"\n',
                encoding="utf-8",
            )
            app_manifest = repo_root / "crates" / "app" / "Cargo.toml"
            app_manifest.parent.mkdir(parents=True)
            app_manifest.write_text(
                '[package]\nname = "fixture-app"\nversion = "0.1.0"\n'
                '[dependencies]\nserde_alias = { package = "serde", version = "1" }\n',
                encoding="utf-8",
            )
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text(
                '[cargo_shear.allowed_unused_dependencies]\n'
                '"fixture-app.serde_alias" = "used by generated code"\n',
                encoding="utf-8",
            )
            metadata = subprocess.CompletedProcess(
                ["cargo", "metadata"],
                0,
                json.dumps(
                    {
                        "packages": [
                            {
                                "name": "fixture-app",
                                "manifest_path": str(app_manifest),
                                "dependencies": [
                                    {"name": "serde", "rename": "serde_alias", "kind": None}
                                ],
                            }
                        ]
                    }
                ),
                "",
            )
            shear = subprocess.CompletedProcess(
                ["cargo-shear"],
                1,
                "shear/unused_dependency\n\n  × unused dependency `serde_alias`\n"
                "    ╭─[crates/app/Cargo.toml:5:1]\n\n"
                "shear/summary\n\n  × 1 error\n",
                "",
            )
            stdout = io.StringIO()
            with (
                patch("sc_lint.lint_cargo_shear.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[]),
                patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear"),
                patch(
                    "sc_lint.lint_cargo_shear.subprocess.run", side_effect=[metadata, shear]
                ) as run_mock,
                patch("sc_lint.lint_cargo_shear.sys.stdout", stdout),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 0)
        self.assertEqual(run_mock.call_count, 2)
        self.assertEqual(
            run_mock.call_args_list[0].args[0],
            ["cargo", "metadata", "--offline", "--format-version", "1", "--no-deps"],
        )
        self.assertEqual(run_mock.call_args_list[1].args[0], ["cargo-shear"])
        self.assertIn(
            "unused_dependency: downgraded fixture-app.serde_alias", stdout.getvalue()
        )

    def test_parse_sections_extracts_warning_files(self) -> None:
        stdout = f"""\
shear/unlinked_files

  ⚠ 1 unlinked file in `{APP_PACKAGE}`
  │ tests/support/mod.rs
  help: delete this file

shear/empty_files

  ⚠ 2 empty files in `{CORE_PACKAGE}`
  │ src/model_registry.rs
  │ src/schema/settings.rs
"""
        sections = parse_sections(stdout)
        self.assertEqual([section.name for section in sections], ["unlinked_files", "empty_files"])
        self.assertEqual(sections[0].file_paths, ("tests/support/mod.rs",))
        self.assertEqual(
            sections[1].file_paths,
            ("src/model_registry.rs", "src/schema/settings.rs"),
        )

    def test_parse_sections_extracts_crate_root_files_and_flags_unrecognized_lines(self) -> None:
        stdout = """\
shear/unlinked_files

  ⚠ 2 unlinked files in `synthetic-app`
  │ src/orphan.rs
  │ stray.rs

shear/empty_files

  ⚠ 2 empty files in `synthetic-app`
  │ empty.rs
  │ src/extra.rs
"""
        sections = parse_sections(stdout)

        self.assertEqual(
            [section.file_paths for section in sections],
            [("src/orphan.rs", "stray.rs"), ("empty.rs", "src/extra.rs")],
        )
        self.assertFalse(any(section.has_unrecognized_file_lines for section in sections))

        section_with_description = parse_sections(
            "shear/unlinked_files\n│ src/orphan.rs\n│ review this output manually\n"
        )[0]
        self.assertEqual(section_with_description.file_paths, ("src/orphan.rs",))
        self.assertTrue(section_with_description.has_unrecognized_file_lines)

    def test_evaluate_policy_promotes_unapproved_warning_files_to_errors(self) -> None:
        stdout = f"""\
shear/unlinked_files

  ⚠ 1 unlinked file in `{APP_PACKAGE}`
  │ tests/support/mod.rs
"""
        sections = parse_sections(stdout)
        findings, downgraded = evaluate_policy(
            sections,
            {"allowed_empty_files": {}, "allowed_unlinked_files": {}},
        )
        self.assertEqual(downgraded, [])
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].section_name, "unlinked_files")
        self.assertEqual(findings[0].file_path, "tests/support/mod.rs")

    def test_evaluate_policy_downgrades_allowlisted_files(self) -> None:
        stdout = f"""\
shear/empty_files

  ⚠ 1 empty file in `{CORE_PACKAGE}`
  │ src/model_registry.rs
"""
        sections = parse_sections(stdout)
        findings, downgraded = evaluate_policy(
            sections,
            {
                "allowed_empty_files": {"src/model_registry.rs": "planned stub"},
                "allowed_unlinked_files": {},
            },
        )
        self.assertEqual(findings, [])
        self.assertEqual(
            downgraded,
            ["empty_files: downgraded src/model_registry.rs (planned stub)"],
        )

    def test_evaluate_unused_dependency_policy_downgrades_exact_crate_dependency(self) -> None:
        stdout = """\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/fixture-app/Cargo.toml:9:1]
  help: remove this dependency

shear/summary

  × 1 error
"""
        findings, downgraded, only_unused_dependencies = evaluate_unused_dependency_policy(
            parse_sections(stdout),
            {
                "allowed_empty_files": {},
                "allowed_unlinked_files": {},
                "allowed_unused_dependencies": {
                    f"{APP_PACKAGE}.serde": "used by generated code",
                },
            },
            {f"crates/{APP_CRATE}/Cargo.toml": APP_PACKAGE},
        )
        self.assertEqual(findings, [])
        self.assertTrue(only_unused_dependencies)
        self.assertEqual(
            downgraded,
            [
                "unused_dependency: downgraded "
                f"{APP_PACKAGE}.serde (used by generated code)"
            ],
        )

    def test_evaluate_unused_dependency_policy_rejects_other_findings(self) -> None:
        stdout = """\
shear/misplaced_dependency

  × dependency `serde` is misplaced
"""
        findings, downgraded, only_unused_dependencies = evaluate_unused_dependency_policy(
            parse_sections(stdout),
            {
                "allowed_empty_files": {},
                "allowed_unlinked_files": {},
                "allowed_unused_dependencies": {
                    f"{APP_PACKAGE}.serde": "used by generated code",
                },
            },
            {f"crates/{APP_CRATE}/Cargo.toml": APP_PACKAGE},
        )
        self.assertEqual(findings, [])
        self.assertEqual(downgraded, [])
        self.assertFalse(only_unused_dependencies)

    def test_evaluate_unused_dependency_policy_rejects_unapproved_dependency(self) -> None:
        stdout = """\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/fixture-app/Cargo.toml:9:1]
  help: remove this dependency
"""
        findings, downgraded, only_unused_dependencies = evaluate_unused_dependency_policy(
            parse_sections(stdout),
            {
                "allowed_empty_files": {},
                "allowed_unlinked_files": {},
                "allowed_unused_dependencies": {},
            },
            {f"crates/{APP_CRATE}/Cargo.toml": APP_PACKAGE},
        )
        self.assertEqual(downgraded, [])
        self.assertTrue(only_unused_dependencies)
        self.assertEqual(findings[0].file_path, f"{APP_PACKAGE}.serde")

    @mock.patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[])
    @mock.patch("sc_lint.lint_cargo_shear.workspace_crates")
    @mock.patch("sc_lint.lint_cargo_shear.subprocess.run")
    @mock.patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear")
    def test_main_downgrades_only_allowlisted_unused_dependency_exit(
        self,
        _which_mock: mock.Mock,
        subprocess_run_mock: mock.Mock,
        workspace_crates_mock: mock.Mock,
        _section_lines_mock: mock.Mock,
    ) -> None:
        manifest_relative = "crates/fixture-λ/Cargo.toml"
        stdout = """\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/fixture-λ/Cargo.toml:9:1]
  help: remove this dependency

shear/summary

  × 1 error
"""
        stderr = "cargo-shear warning: λ\n"
        subprocess_run_mock.return_value = mock.Mock(returncode=1, stdout=stdout, stderr=stderr)
        restrictive_stdout = RestrictiveEncodingStream()
        restrictive_stderr = RestrictiveEncodingStream()
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            just_dir = repo_root / ".just"
            just_dir.mkdir()
            (just_dir / "lint-config.toml").write_text(
                f"""\
[cargo_shear.allowed_unused_dependencies]
"{APP_PACKAGE}.serde" = "generated by λ"
""",
                encoding="utf-8",
            )
            workspace_crates_mock.return_value = [
                WorkspaceCrate(
                    crate_dir="crates/fixture-λ",
                    package_name=APP_PACKAGE,
                    crate_path_name=APP_CRATE.replace("-", "_"),
                    manifest_path=str(repo_root / manifest_relative),
                )
            ]
            with (
                mock.patch("sc_lint.lint_cargo_shear.sys.stdout", restrictive_stdout),
                mock.patch("sc_lint.lint_cargo_shear.sys.stderr", restrictive_stderr),
                mock.patch(
                    "sc_lint.lint_cargo_shear.load_cargo_shear_workspace_metadata",
                    return_value=CargoShearWorkspaceMetadata(
                        {manifest_relative: APP_PACKAGE},
                        frozenset({f"{APP_PACKAGE}.serde"}),
                    ),
                ),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 0)
        self.assertEqual(
            restrictive_stdout.getvalue(),
            "unused_dependency: downgraded synthetic-app.serde (generated by ?)\n"
            + stdout.encode("ascii", errors="replace").decode("ascii"),
        )
        self.assertEqual(restrictive_stderr.getvalue(), "cargo-shear warning: ?\n")

    @mock.patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[])
    @mock.patch("sc_lint.lint_cargo_shear.workspace_crates")
    @mock.patch("sc_lint.lint_cargo_shear.subprocess.run")
    @mock.patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear")
    def test_main_safely_reports_unallowlisted_unused_dependency_exit(
        self,
        _which_mock: mock.Mock,
        subprocess_run_mock: mock.Mock,
        workspace_crates_mock: mock.Mock,
        _section_lines_mock: mock.Mock,
    ) -> None:
        manifest_relative = "crates/fixture-λ/Cargo.toml"
        stdout = """\
shear/unused_dependency

  × unused dependency `λserde`
    ╭─[crates/fixture-λ/Cargo.toml:9:1]
  help: remove this dependency

shear/summary

  × 1 error
"""
        stderr = "cargo-shear warning: λ\n"
        subprocess_run_mock.return_value = mock.Mock(returncode=1, stdout=stdout, stderr=stderr)
        restrictive_stdout = RestrictiveEncodingStream()
        restrictive_stderr = RestrictiveEncodingStream()
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            just_dir = repo_root / ".just"
            just_dir.mkdir()
            (just_dir / "lint-config.toml").write_text(
                "[cargo_shear.allowed_unused_dependencies]\n",
                encoding="utf-8",
            )
            workspace_crates_mock.return_value = [
                WorkspaceCrate(
                    crate_dir="crates/fixture-λ",
                    package_name=APP_PACKAGE,
                    crate_path_name=APP_CRATE.replace("-", "_"),
                    manifest_path=str(repo_root / manifest_relative),
                )
            ]
            with (
                mock.patch("sc_lint.lint_cargo_shear.sys.stdout", restrictive_stdout),
                mock.patch("sc_lint.lint_cargo_shear.sys.stderr", restrictive_stderr),
                mock.patch(
                    "sc_lint.lint_cargo_shear.load_cargo_shear_workspace_metadata",
                    return_value=CargoShearWorkspaceMetadata(
                        {manifest_relative: APP_PACKAGE}, frozenset()
                    ),
                ),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 1)
        self.assertEqual(
            restrictive_stdout.getvalue(),
            "shear policy error: synthetic-app.?serde [unused_dependency] "
            "unused dependency must be fixed or explicitly downgraded in .just/lint-config.toml\n"
            + stdout.encode("ascii", errors="replace").decode("ascii"),
        )
        self.assertEqual(restrictive_stderr.getvalue(), "cargo-shear warning: ?\n")

    def run_unused_dependency_main_from_foreign_cwd(
        self, allowlist: str, shear_stdout: str | None = None
    ) -> tuple[int, str]:
        """Run main() with --root from another cwd against a real workspace.

        workspace_crates returns repo-relative manifest paths, so this covers
        the path the mocked-absolute tests above do not.
        """
        stdout = shear_stdout or f"""\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/{APP_CRATE}/Cargo.toml:9:1]
  help: remove this dependency

shear/summary

  × 1 error
"""
        completed = mock.Mock(returncode=1, stdout=stdout, stderr="")
        with tempfile.TemporaryDirectory() as repo_dir:
            repo_root = Path(repo_dir)
            (repo_root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/*"]\n', encoding="utf-8"
            )
            crate_dir = repo_root / "crates" / APP_CRATE
            crate_dir.mkdir(parents=True)
            (crate_dir / "Cargo.toml").write_text(
                f'[package]\nname = "{APP_PACKAGE}"\nversion = "0.1.0"\n\n[dependencies]\nserde = "1"\n',
                encoding="utf-8",
            )
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text(allowlist, encoding="utf-8")
            self.assertNotEqual(repo_root.resolve(), Path.cwd().resolve())
            self.assertEqual(
                manifest_to_package_name(repo_root),
                {f"crates/{APP_CRATE}/Cargo.toml": APP_PACKAGE},
            )
            with (
                patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear"),
                patch("sc_lint.lint_cargo_shear.subprocess.run", return_value=completed),
                patch(
                    "sc_lint.lint_cargo_shear.load_cargo_shear_workspace_metadata",
                    return_value=CargoShearWorkspaceMetadata(
                        {f"crates/{APP_CRATE}/Cargo.toml": APP_PACKAGE},
                        frozenset(
                            {f"{APP_PACKAGE}.serde", f"{APP_PACKAGE}.regex"}
                        ),
                    ),
                ),
                patch("sys.stdout", new_callable=io.StringIO) as stdout_mock,
                patch("sys.stderr", new_callable=io.StringIO),
            ):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])
        return result, stdout_mock.getvalue()

    def test_main_from_foreign_cwd_downgrades_allowlisted_unused_dependency(self) -> None:
        result, output = self.run_unused_dependency_main_from_foreign_cwd(
            f'[cargo_shear.allowed_unused_dependencies]\n"{APP_PACKAGE}.serde" = "used by generated code"\n'
        )

        self.assertEqual(result, 0)
        self.assertIn(
            f"unused_dependency: downgraded {APP_PACKAGE}.serde (used by generated code)",
            output,
        )

    def test_main_from_foreign_cwd_rejects_unlisted_unused_dependency(self) -> None:
        result, output = self.run_unused_dependency_main_from_foreign_cwd("[cargo_shear]\n")

        self.assertEqual(result, 1)
        self.assertIn(
            f"shear policy error: {APP_PACKAGE}.serde [unused_dependency] unused dependency must be fixed",
            output,
        )

    # Section layout of a real cargo-shear run: exit 1 with two unused
    # dependencies (errors) and one unlinked file (warning).
    MIXED_SHEAR_STDOUT = f"""\
shear/unused_dependency

  × unused dependency `serde`
   ╭─[crates/{APP_CRATE}/Cargo.toml:6:1]
  help: remove this dependency

shear/unused_dependency

  × unused dependency `regex`
   ╭─[crates/{APP_CRATE}/Cargo.toml:7:1]
  help: remove this dependency

shear/unlinked_files

  ⚠ 1 unlinked file in `{APP_PACKAGE}`
  │ src/orphan.rs
  help: delete this file

shear/summary

  ✗ 2 errors
  ⚠ 1 warning
"""
    MIXED_DEPENDENCY_ALLOWLIST = f"""\
[cargo_shear.allowed_unused_dependencies]
"{APP_PACKAGE}.serde" = "used by generated code"
"{APP_PACKAGE}.regex" = "used by generated code"
"""

    def test_main_composes_allowlisted_unused_dependencies_and_unlinked_files(self) -> None:
        result, output = self.run_unused_dependency_main_from_foreign_cwd(
            self.MIXED_DEPENDENCY_ALLOWLIST
            + '\n[cargo_shear.allowed_unlinked_files]\n"src/orphan.rs" = "planned module"\n',
            self.MIXED_SHEAR_STDOUT,
        )

        self.assertEqual(result, 0)
        self.assertIn(f"unused_dependency: downgraded {APP_PACKAGE}.serde (used by generated code)", output)
        self.assertIn(f"unused_dependency: downgraded {APP_PACKAGE}.regex (used by generated code)", output)
        self.assertIn("unlinked_files: downgraded src/orphan.rs (planned module)", output)
        self.assertNotIn("shear policy error", output)

    def test_main_rejects_unlisted_unlinked_file_beside_allowlisted_dependencies(self) -> None:
        result, output = self.run_unused_dependency_main_from_foreign_cwd(
            self.MIXED_DEPENDENCY_ALLOWLIST, self.MIXED_SHEAR_STDOUT
        )

        self.assertEqual(result, 1)
        self.assertIn("shear policy error: src/orphan.rs [unlinked_files]", output)
        self.assertIn(f"unused_dependency: downgraded {APP_PACKAGE}.serde (used by generated code)", output)

    def test_main_evaluates_root_and_nested_files_in_both_error_sections(self) -> None:
        for section_name in ("unlinked_files", "empty_files"):
            with self.subTest(section_name=section_name):
                stdout = f"""\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/{APP_CRATE}/Cargo.toml:9:1]

shear/{section_name}

  ⚠ 2 files in `{APP_PACKAGE}`
  │ src/orphan.rs
  │ stray.rs

shear/summary

  × 1 error
"""
                allowlisted_nested_only = (
                    self.MIXED_DEPENDENCY_ALLOWLIST
                    + f'\n[cargo_shear.allowed_{section_name}]\n'
                    + '"src/orphan.rs" = "reviewed nested file"\n'
                )
                result, output = self.run_unused_dependency_main_from_foreign_cwd(
                    allowlisted_nested_only, stdout
                )

                self.assertEqual(result, 1)
                self.assertIn(f"shear policy error: stray.rs [{section_name}]", output)
                self.assertNotIn(f"{section_name}: downgraded stray.rs", output)

                fully_allowlisted = (
                    allowlisted_nested_only
                    + '"stray.rs" = "reviewed root file"\n'
                )
                result, output = self.run_unused_dependency_main_from_foreign_cwd(
                    fully_allowlisted, stdout
                )

                self.assertEqual(result, 0)
                self.assertIn(
                    f"{section_name}: downgraded src/orphan.rs (reviewed nested file)",
                    output,
                )
                self.assertIn(
                    f"{section_name}: downgraded stray.rs (reviewed root file)",
                    output,
                )

    def test_main_does_not_treat_descriptive_file_lines_as_reviewed(self) -> None:
        for section_name in ("unlinked_files", "empty_files"):
            with self.subTest(section_name=section_name):
                stdout = f"""\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/{APP_CRATE}/Cargo.toml:9:1]

shear/{section_name}

  ⚠ file report for `{APP_PACKAGE}`
  │ src/orphan.rs
  │ review this output manually

shear/summary

  × 1 error
"""
                allowlisted_file = (
                    self.MIXED_DEPENDENCY_ALLOWLIST
                    + f'\n[cargo_shear.allowed_{section_name}]\n'
                    + '"src/orphan.rs" = "reviewed nested file"\n'
                )
                result, output = self.run_unused_dependency_main_from_foreign_cwd(
                    allowlisted_file, stdout
                )

                self.assertEqual(result, 1)
                self.assertNotIn(
                    f"{section_name}: downgraded src/orphan.rs", output
                )
                self.assertIn(f"shear/{section_name}", output)

    def test_main_rejects_unparsed_error_sections_beside_allowlisted_dependencies(self) -> None:
        for section_name in ("unlinked_files", "empty_files"):
            with self.subTest(section_name=section_name):
                stdout = f"""\\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/{APP_CRATE}/Cargo.toml:9:1]

shear/{section_name}

  help: cargo-shear output did not include a file path

shear/summary

  × 1 error
"""
                result, output = self.run_unused_dependency_main_from_foreign_cwd(
                    self.MIXED_DEPENDENCY_ALLOWLIST,
                    stdout,
                )

                self.assertEqual(result, 1)
                self.assertNotIn("unused_dependency: downgraded", output)
                self.assertIn(f"shear/{section_name}", output)

    @mock.patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=[])
    @mock.patch("sc_lint.lint_cargo_shear.subprocess.run")
    @mock.patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear")
    def test_main_preserves_fatal_cargo_shear_exit(
        self,
        _which_mock: mock.Mock,
        subprocess_run_mock: mock.Mock,
        _section_lines_mock: mock.Mock,
    ) -> None:
        subprocess_run_mock.return_value = mock.Mock(
            returncode=2,
            stdout="error: cargo metadata failed\n",
            stderr="",
        )
        with tempfile.TemporaryDirectory() as tempdir:
            (Path(tempdir) / ".just").mkdir()
            (Path(tempdir) / ".just" / "lint-config.toml").write_text("[cargo_shear]\n", encoding="utf-8")
            with mock.patch("sys.stdout", new_callable=io.StringIO):
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", tempdir])

        self.assertEqual(result, 2)

    def test_load_policy_config_normalizes_windows_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            just_dir = repo_root / ".just"
            just_dir.mkdir()
            (just_dir / "lint-config.toml").write_text(
                """\
[cargo_shear.allowed_empty_files]
"src\\\\model_registry.rs" = "planned stub"

[cargo_shear.allowed_unlinked_files]
"tests\\\\support\\\\mod.rs" = "legacy pending"

[cargo_shear.allowed_unused_dependencies]
"synthetic-app.serde" = "used by generated code"
"synthetic-lib.empty" = "   "
""",
                encoding="utf-8",
            )

            policy = load_policy_config(repo_root)
            self.assertEqual(
                policy["allowed_empty_files"]["src/model_registry.rs"],
                "planned stub",
            )
            self.assertEqual(
                policy["allowed_unlinked_files"]["tests/support/mod.rs"],
                "legacy pending",
            )
            self.assertEqual(
                policy["allowed_unused_dependencies"]["synthetic-app.serde"],
                "used by generated code",
            )
            self.assertNotIn("synthetic-lib.empty", policy["allowed_unused_dependencies"])

    def test_load_policy_config_defaults_unused_dependency_allowlist(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            (Path(tempdir) / ".just").mkdir()
            (Path(tempdir) / ".just" / "lint-config.toml").write_text("[cargo_shear]\n", encoding="utf-8")
            self.assertEqual(
                load_policy_config(Path(tempdir))["allowed_unused_dependencies"],
                {},
            )

    def test_annotate_sections_uses_crate_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                f"""\
[workspace]
members = ["crates/{APP_CRATE}", "crates/{CORE_CRATE}"]
resolver = "2"
""",
                encoding="utf-8",
            )
            for crate_name, package_name in (
                (APP_CRATE, APP_PACKAGE),
                (CORE_CRATE, CORE_PACKAGE),
            ):
                crate_dir = repo_root / "crates" / crate_name
                crate_dir.mkdir(parents=True)
                (crate_dir / "Cargo.toml").write_text(
                    f"""\
[package]
name = "{package_name}"
version = "1.1.2"
""",
                    encoding="utf-8",
                )

            sections = parse_sections(
                f"""\
shear/unlinked_files

  ⚠ 1 unlinked file in `{APP_PACKAGE}`
  │ tests/support/mod.rs
"""
            )
            self.assertEqual(
                annotate_sections(sections, repo_root),
                [f"shear note: crates/{APP_CRATE}/tests/support/mod.rs [unlinked_files]"],
            )

    def test_build_codespell_command_uses_repo_config(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            command = build_codespell_command(repo_root)
            self.assertEqual(command[:2], [sys.executable, "-c"])
            self.assertIn("codespell_lib", command[2])
            self.assertEqual(len(command), 3)

    def test_write_console_output_preserves_representable_unicode(self) -> None:
        stream = io.StringIO()

        write_console_output(stream, "symbol \u03bb\n")

        self.assertEqual(stream.getvalue(), "symbol \u03bb\n")

    def test_codespell_forwards_restrictive_console_output_and_exit_code(self) -> None:
        stdout = RestrictiveEncodingStream()
        stderr = RestrictiveEncodingStream()
        completed = subprocess.CompletedProcess(
            args=["codespell"],
            returncode=7,
            stdout="output: \u03bb\n",
            stderr="failure: \u03bb\n",
        )

        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            create_non_ascii_workspace(repo_root)
            section = "".join(
                f"{line}\n" for line in workspace_crate_section_lines(repo_root)
            )
            with (
                patch("sc_lint.lint_codespell.subprocess.run", return_value=completed),
                patch("sc_lint.lint_codespell.sys.stdout", stdout),
                patch("sc_lint.lint_codespell.sys.stderr", stderr),
            ):
                result = lint_codespell.main(
                    ["lint-codespell", "--root", str(repo_root)]
                )

        self.assertEqual(result, 7)
        self.assertEqual(
            stdout.getvalue(),
            section.encode("ascii", errors="replace").decode("ascii")
            + "output: ?\n",
        )
        self.assertEqual(stderr.getvalue(), "failure: ?\n")

    def test_codespell_main_preserves_non_ascii_workspace_on_utf8_console(self) -> None:
        stdout = io.StringIO()
        completed = subprocess.CompletedProcess(
            args=["codespell"], returncode=0, stdout="", stderr=""
        )

        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            create_non_ascii_workspace(repo_root)
            with (
                patch("sc_lint.lint_codespell.subprocess.run", return_value=completed),
                patch("sc_lint.lint_codespell.sys.stdout", stdout),
            ):
                result = lint_codespell.main(
                    ["lint-codespell", "--root", str(repo_root)]
                )

        self.assertEqual(result, 0)
        self.assertIn("λapp", stdout.getvalue())
        self.assertNotIn("?app", stdout.getvalue())

    def test_cargo_deny_main_forwards_restrictive_console_output_and_exit_code(self) -> None:
        stdout = RestrictiveEncodingStream()
        stderr = RestrictiveEncodingStream()
        completed = subprocess.CompletedProcess(
            args=["cargo-deny"],
            returncode=5,
            stdout="deny output: \u03bb\n",
            stderr="deny failure: \u03bb\n",
        )

        with (
            tempfile.TemporaryDirectory() as tempdir,
            tempfile.TemporaryDirectory(prefix="atm-lint-deny-") as runtime_dir,
        ):
            repo_root = Path(tempdir)
            create_non_ascii_workspace(repo_root)
            (repo_root / "deny.toml").write_text("[advisories]\n", encoding="utf-8")
            section = "".join(
                f"{line}\n" for line in workspace_crate_section_lines(repo_root)
            )
            with (
                patch("sc_lint.lint_cargo_deny.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_deny.shutil.which", return_value="cargo-deny"),
                patch("sc_lint.lint_cargo_deny.detect_version", return_value=(0, 20)),
                patch("sc_lint.lint_cargo_deny.subprocess.run", return_value=completed),
                patch("sc_lint.lint_cargo_deny.sys.stdout", stdout),
                patch("sc_lint.lint_cargo_deny.sys.stderr", stderr),
                patch(
                    "sc_lint.lint_cargo_deny.tempfile.mkdtemp",
                    return_value=runtime_dir,
                ),
            ):
                result = lint_cargo_deny.main(["lint-cargo-deny", "--root", str(repo_root)])

        self.assertEqual(result, 5)
        self.assertEqual(
            stdout.getvalue(),
            section.encode("ascii", errors="replace").decode("ascii")
            + "deny output: ?\n",
        )
        self.assertEqual(stderr.getvalue(), "deny failure: ?\n")

    def test_cargo_shear_main_safely_writes_policy_and_downgraded_lines(self) -> None:
        stdout = Cp1252RestrictiveEncodingStream()
        stderr = Cp1252RestrictiveEncodingStream()
        child_output = """\
shear/unlinked_files

  ⚠ 1 unlinked file in `synthetic-app`
  │ src/λambda.rs
  help: delete this file

shear/empty_files

  ⚠ 1 empty file in `synthetic-app`
  │ src/βeta.rs
"""
        completed = subprocess.CompletedProcess(
            args=["cargo-shear"], returncode=0, stdout=child_output, stderr=""
        )

        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text("[cargo_shear]\n", encoding="utf-8")
            crate = WorkspaceCrate(
                crate_dir="app",
                package_name=APP_PACKAGE,
                crate_path_name="app",
                manifest_path="crates/app/Cargo.toml",
            )
            policy = {
                "allowed_empty_files": {"src/βeta.rs": "reviewed λ"},
                "allowed_unlinked_files": {},
            }
            with (
                patch("sc_lint.lint_cargo_shear.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear"),
                patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=["workspace crates:"]),
                patch("sc_lint.lint_cargo_shear.workspace_crates", return_value=[crate]),
                patch("sc_lint.lint_cargo_shear.load_policy_config", return_value=policy),
                patch("sc_lint.lint_cargo_shear.subprocess.run", return_value=completed),
                patch("sc_lint.lint_cargo_shear.sys.stdout", stdout),
                patch("sc_lint.lint_cargo_shear.sys.stderr", stderr),
            ):
                result = lint_cargo_shear.main(["lint-cargo-shear", "--root", str(repo_root)])

        self.assertEqual(result, 1)
        self.assertEqual(
            stdout.getvalue(),
            "workspace crates:\n"
            "shear policy error: src/?ambda.rs [unlinked_files] "
            "unlinked_files must be fixed or explicitly downgraded in .just/lint-config.toml\n"
            "shear note: crates/app/src/?ambda.rs [unlinked_files]\n"
            "empty_files: downgraded src/?eta.rs (reviewed ?)\n"
            + child_output.encode("cp1252", errors="replace").decode("cp1252"),
        )
        self.assertEqual(stderr.getvalue(), "")

    def test_cargo_shear_main_safely_writes_downgraded_only_lines(self) -> None:
        stdout = Cp1252RestrictiveEncodingStream()
        stderr = Cp1252RestrictiveEncodingStream()
        child_output = """\
shear/empty_files

  ⚠ 1 empty file in `synthetic-app`
  │ src/βeta.rs
"""
        completed = subprocess.CompletedProcess(
            args=["cargo-shear"], returncode=0, stdout=child_output, stderr=""
        )

        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / ".just").mkdir()
            (repo_root / ".just" / "lint-config.toml").write_text("[cargo_shear]\n", encoding="utf-8")
            policy = {
                "allowed_empty_files": {"src/βeta.rs": "reviewed λ"},
                "allowed_unlinked_files": {},
            }
            with (
                patch("sc_lint.lint_cargo_shear.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear"),
                patch("sc_lint.lint_cargo_shear.workspace_crate_section_lines", return_value=["workspace crates:"]),
                patch("sc_lint.lint_cargo_shear.load_policy_config", return_value=policy),
                patch("sc_lint.lint_cargo_shear.subprocess.run", return_value=completed),
                patch("sc_lint.lint_cargo_shear.sys.stdout", stdout),
                patch("sc_lint.lint_cargo_shear.sys.stderr", stderr),
            ):
                result = lint_cargo_shear.main(["lint-cargo-shear", "--root", str(repo_root)])

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue(),
            "workspace crates:\nempty_files: downgraded src/?eta.rs (reviewed ?)\n"
            + child_output.encode("cp1252", errors="replace").decode("cp1252"),
        )
        self.assertEqual(stderr.getvalue(), "")

    def test_write_console_output_preserves_unrelated_io_errors(self) -> None:
        class BrokenStream:
            encoding = "ascii"
            errors = "strict"

            def write(self, text: str) -> None:
                raise OSError("console disconnected")

        with self.assertRaisesRegex(OSError, "console disconnected"):
            write_console_output(BrokenStream(), "symbol \u03bb\n")


if __name__ == "__main__":
    unittest.main()
