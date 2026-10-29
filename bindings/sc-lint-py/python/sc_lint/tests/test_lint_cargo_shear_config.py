from __future__ import annotations

import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from sc_lint import lint_cargo_shear
from sc_lint.lint_common import ConfigError
from sc_lint.lint_cargo_shear import configured_unused_dependency_allowlist
from sc_lint.lint_cargo_shear import display_cargo_shear_config_path
from sc_lint.lint_cargo_shear import evaluate_policy
from sc_lint.lint_cargo_shear import load_policy_config
from sc_lint.lint_cargo_shear import parse_sections
from sc_lint.lint_cargo_shear import main


SC_LINT_TOML = "sc-lint.toml"
LINT_CONFIG = ".just/lint-config.toml"


def _write(root: Path, relative: str, content: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


class CargoShearConfigResolutionTests(unittest.TestCase):
    def test_sc_lint_toml_table_wins_over_lint_config(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(
                root,
                SC_LINT_TOML,
                '[cargo_shear.allowed_unused_dependencies]\n"demo.serde" = "primary reason"\n',
            )
            _write(
                root,
                LINT_CONFIG,
                '[cargo_shear.allowed_unused_dependencies]\n"demo.tokio" = "legacy reason"\n',
            )

            supplied, allowlist = configured_unused_dependency_allowlist(root)
            policy = load_policy_config(root)

            self.assertTrue(supplied)
            self.assertEqual(allowlist, {"demo.serde": "primary reason"})
            self.assertEqual(policy["allowed_unused_dependencies"], {"demo.serde": "primary reason"})

    def test_lint_config_table_alone_is_used(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(
                root,
                LINT_CONFIG,
                '[cargo_shear.allowed_unused_dependencies]\n"demo.tokio" = "legacy reason"\n',
            )

            supplied, allowlist = configured_unused_dependency_allowlist(root)
            policy = load_policy_config(root)

            self.assertTrue(supplied)
            self.assertEqual(allowlist, {"demo.tokio": "legacy reason"})
            self.assertEqual(policy["allowed_unused_dependencies"], {"demo.tokio": "legacy reason"})

    def test_policy_error_names_the_file_that_supplied_the_table(self) -> None:
        for config_path in (SC_LINT_TOML, LINT_CONFIG):
            with self.subTest(config_path=config_path), tempfile.TemporaryDirectory() as tempdir:
                root = Path(tempdir)
                _write(root, config_path, "[cargo_shear]\n")
                table, source = lint_cargo_shear.load_cargo_shear_config(root)
                self.assertEqual(table, {})
                self.assertEqual(source, root / config_path)
                displayed_source = display_cargo_shear_config_path(source, root)

                sections = parse_sections(
                    "shear/unlinked_files\n\n  ⚠ 1 unlinked file\n  │ src/orphan.rs\n"
                )
                policy = load_policy_config(root)
                findings, _ = evaluate_policy(
                    sections, policy, config_path=displayed_source
                )

                self.assertEqual(len(findings), 1)
                self.assertIn(config_path, findings[0].reason)

    def test_table_in_neither_file_fails_instead_of_defaulting(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, SC_LINT_TOML, '[tool.sc-lint]\nname = "demo"\n')

            with self.assertRaises(ConfigError) as raised:
                configured_unused_dependency_allowlist(root)
            with self.assertRaises(ConfigError):
                load_policy_config(root)

            self.assertEqual(raised.exception.table, "cargo_shear")
            self.assertIn(SC_LINT_TOML, raised.exception.searched)
            self.assertIn(LINT_CONFIG, raised.exception.searched)

    def test_main_reports_config_table_missing_and_skips_cargo_shear(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            stderr = io.StringIO()
            with (
                patch.object(lint_cargo_shear.sys, "stderr", stderr),
                patch("sc_lint.lint_cargo_shear.subprocess.run") as run,
            ):
                result = main(["lint_cargo_shear.py", "--root", str(root)])

            self.assertEqual(result, 3)
            self.assertIn("config.table_missing", stderr.getvalue())
            self.assertIn("configuration table [cargo_shear] not found", stderr.getvalue())
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
