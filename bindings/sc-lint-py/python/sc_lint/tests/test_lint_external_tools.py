from __future__ import annotations

import io
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
from sc_lint.lint_cargo_shear import annotate_sections
from sc_lint.lint_cargo_shear import build_command as build_cargo_shear_command
from sc_lint.lint_cargo_shear import evaluate_policy
from sc_lint.lint_cargo_shear import evaluate_unused_dependency_policy
from sc_lint.lint_cargo_shear import load_policy_config
from sc_lint.lint_cargo_shear import main as cargo_shear_main
from sc_lint.lint_cargo_shear import parse_sections
from sc_lint.lint_codespell import build_command as build_codespell_command
from sc_lint.lint_common import WorkspaceCrate
from sc_lint.lint_common import write_console_output

APP_CRATE = "fixture-app"
CORE_CRATE = "fixture-lib"
APP_PACKAGE = "synthetic-app"
CORE_PACKAGE = "synthetic-lib"


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
        with tempfile.TemporaryDirectory() as tempdir:
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

            runtime_path = build_runtime_config(repo_root)
            text = runtime_path.read_text(encoding="utf-8")

            self.assertNotIn('vulnerability = "deny"', text)
            self.assertNotIn('unlicensed = "deny"', text)
            self.assertIn('yanked = "deny"', text)
            self.assertIn('allow = ["MIT"]', text)

    def test_build_cargo_shear_command_targets_workspace_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.assertEqual(
                build_cargo_shear_command(repo_root),
                ["cargo-shear"],
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
        stdout = """\
shear/unused_dependency

  × unused dependency `serde`
    ╭─[crates/fixture-app/Cargo.toml:9:1]
  help: remove this dependency

shear/summary

  × 1 error
"""
        subprocess_run_mock.return_value = mock.Mock(returncode=1, stdout=stdout, stderr="")
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            just_dir = repo_root / ".just"
            just_dir.mkdir()
            (just_dir / "lint-config.toml").write_text(
                f"""\
[cargo_shear.allowed_unused_dependencies]
"{APP_PACKAGE}.serde" = "used by generated code"
""",
                encoding="utf-8",
            )
            workspace_crates_mock.return_value = [
                WorkspaceCrate(
                    crate_dir=f"crates/{APP_CRATE}",
                    package_name=APP_PACKAGE,
                    crate_path_name=APP_CRATE.replace("-", "_"),
                    manifest_path=str(repo_root / "crates" / APP_CRATE / "Cargo.toml"),
                )
            ]
            with mock.patch("sys.stdout", new_callable=io.StringIO) as stdout_mock:
                result = cargo_shear_main(["lint_cargo_shear.py", "--root", str(repo_root)])

        self.assertEqual(result, 0)
        self.assertIn(
            f"unused_dependency: downgraded {APP_PACKAGE}.serde (used by generated code)",
            stdout_mock.getvalue(),
        )

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

        with (
            patch("sc_lint.lint_codespell.discover_repo_root", return_value=Path(".")),
            patch("sc_lint.lint_codespell.workspace_crate_section_lines", return_value=[]),
            patch("sc_lint.lint_codespell.subprocess.run", return_value=completed),
            patch("sc_lint.lint_codespell.sys.stdout", stdout),
            patch("sc_lint.lint_codespell.sys.stderr", stderr),
        ):
            result = lint_codespell.main(["lint-codespell"])

        self.assertEqual(result, 7)
        self.assertEqual(stdout.getvalue(), "output: ?\n")
        self.assertEqual(stderr.getvalue(), "failure: ?\n")

    def test_cargo_deny_main_forwards_restrictive_console_output_and_exit_code(self) -> None:
        stdout = RestrictiveEncodingStream()
        stderr = RestrictiveEncodingStream()
        completed = subprocess.CompletedProcess(
            args=["cargo-deny"],
            returncode=5,
            stdout="deny output: \u03bb\n",
            stderr="deny failure: \u03bb\n",
        )

        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "deny.toml").write_text("[advisories]\n", encoding="utf-8")
            with (
                patch("sc_lint.lint_cargo_deny.discover_repo_root", return_value=repo_root),
                patch("sc_lint.lint_cargo_deny.shutil.which", return_value="cargo-deny"),
                patch("sc_lint.lint_cargo_deny.workspace_crate_section_lines", return_value=["workspace crates:"]),
                patch("sc_lint.lint_cargo_deny.detect_version", return_value=(0, 20)),
                patch("sc_lint.lint_cargo_deny.subprocess.run", return_value=completed),
                patch("sc_lint.lint_cargo_deny.sys.stdout", stdout),
                patch("sc_lint.lint_cargo_deny.sys.stderr", stderr),
            ):
                result = lint_cargo_deny.main(["lint-cargo-deny", "--root", str(repo_root)])

        self.assertEqual(result, 5)
        self.assertEqual(stdout.getvalue(), "workspace crates:\ndeny output: ?\n")
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
