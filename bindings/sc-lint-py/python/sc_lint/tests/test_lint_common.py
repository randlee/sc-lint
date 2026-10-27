from __future__ import annotations

import contextlib
from datetime import datetime, timezone
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest


from sc_lint.lint_common import build_report
from sc_lint.lint_common import classify_rust_test_scope
from sc_lint.lint_common import iter_string_literal_contents
from sc_lint.lint_common import iter_workspace_rust_files
from sc_lint.lint_common import LintDirectivePolicy
from sc_lint.lint_common import lint_slug
from sc_lint.lint_common import line_is_suppressed
from sc_lint.lint_common import load_lint_config
from sc_lint.lint_common import CONFIG_TABLE_MISSING
from sc_lint.lint_common import ConfigError
from sc_lint.lint_common import resolve_table
from sc_lint.lint_common import make_log_path
from sc_lint.lint_common import relative_log_path
from sc_lint.lint_common import render_workspace_crate_table
from sc_lint.lint_common import rust_file_test_scope
from sc_lint.lint_common import table_missing_error
from sc_lint.lint_common import workspace_crate_section_lines
from sc_lint.lint_common import workspace_crates
from sc_lint.lint_line_counts import main as line_counts_main
from sc_lint.python_adapter import AdapterError
from sc_lint.fixture_constants import TEAM_LEAD_IDENTITY


ROOT_MANIFEST = """\
[workspace]
members = ["crates/sc-lint-boundary", "crates/sc-lint-attributes"]
resolver = "2"

[workspace.package]
version = "0.1.0"
"""


class LintCommonTests(unittest.TestCase):
    def test_lint_slug_normalizes_names(self) -> None:
        self.assertEqual(lint_slug("Rule 8 / identities"), "rule-8-identities")

    def test_build_report_writes_log(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            started_at = datetime(2026, 5, 4, 3, 15, 0, tzinfo=timezone.utc)
            report = build_report(
                lint_name="manifests",
                repo_root=repo_root,
                passed=True,
                summary="manifest policy satisfied",
                findings=[],
                transcript_lines=["no manifest violations found"],
                started_at=started_at,
                duration_seconds=0.42,
            )

            self.assertTrue(report.log_path.is_file())
            self.assertIn("summary: manifest policy satisfied", report.log_path.read_text(encoding="utf-8"))
            self.assertEqual(relative_log_path(repo_root, report.log_path), ".just/logs/20260504031500-manifests.log")

    def test_make_log_path_uses_timestamp_and_slug(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            started_at = datetime(2026, 5, 4, 3, 16, 0, tzinfo=timezone.utc)
            path = make_log_path(repo_root, "Boundary Check", started_at)
            self.assertEqual(path, repo_root / ".just/logs/20260504031600-boundary-check.log")

    def test_load_lint_config_reads_repo_policy(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            just_dir = repo_root / ".just"
            just_dir.mkdir()
            (just_dir / "lint-config.toml").write_text(
                "[portability]\nconfig_home_env = \"SC_LINT_CONFIG_HOME\"\n",
                encoding="utf-8",
            )

            config = load_lint_config(repo_root)

            self.assertEqual(config["portability"]["config_home_env"], "SC_LINT_CONFIG_HOME")

    def test_resolve_table_prefers_explicit_then_sc_lint_toml_then_lint_config(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / ".just").mkdir()
            (root / "sc-lint.toml").write_text("[line_counts]\nmax_total_lines = 300\n", encoding="utf-8")
            (root / ".just/lint-config.toml").write_text("[line_counts]\nmax_total_lines = 500\n", encoding="utf-8")
            explicit = root / "explicit.toml"
            explicit.write_text("[line_counts]\nmax_total_lines = 200\n", encoding="utf-8")
            table, path = resolve_table("line_counts", explicit, root)
            self.assertEqual((table["max_total_lines"], path), (200, explicit))
            table, path = resolve_table("line_counts", None, root)
            self.assertEqual((table["max_total_lines"], path), (300, root / "sc-lint.toml"))
            (root / "sc-lint.toml").write_text("[logging]\nconsole = false\n", encoding="utf-8")
            table, path = resolve_table("line_counts", None, root)
            self.assertEqual((table["max_total_lines"], path), (500, root / ".just/lint-config.toml"))

    def test_resolve_table_falls_back_per_table(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / ".just").mkdir()
            (root / "sc-lint.toml").write_text("[identities]\nforbidden_literals = []\n", encoding="utf-8")
            (root / ".just/lint-config.toml").write_text("[line_counts]\nmax_total_lines = 500\n", encoding="utf-8")
            self.assertEqual(
                resolve_table("identities", None, root),
                ({"forbidden_literals": []}, root / "sc-lint.toml"),
            )
            self.assertEqual(
                resolve_table("line_counts", None, root),
                ({"max_total_lines": 500}, root / ".just/lint-config.toml"),
            )

    def test_resolve_table_dedupes_explicit_equal_to_default(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / "sc-lint.toml").write_text("[logging]\nconsole = false\n", encoding="utf-8")
            with self.assertRaises(ConfigError) as caught:
                resolve_table("line_counts", Path("sc-lint.toml"), root)
            self.assertEqual(caught.exception.searched, ["sc-lint.toml", ".just/lint-config.toml"])

    def test_resolve_table_explicit_missing_file_is_hard_error(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / "sc-lint.toml").write_text("[line_counts]\nmax_total_lines = 300\n", encoding="utf-8")
            with self.assertRaises(AdapterError) as caught:
                resolve_table("line_counts", Path("missing.toml"), root)
            self.assertEqual(caught.exception.kind, "config")
            self.assertEqual(caught.exception.message, "explicit config file not found: missing.toml")

    def test_resolve_table_explicit_without_table_falls_through(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / "sc-lint.toml").write_text("[line_counts]\nmax_total_lines = 300\n", encoding="utf-8")
            explicit = root / "explicit.toml"
            explicit.write_text("[identities]\n", encoding="utf-8")
            self.assertEqual(
                resolve_table("line_counts", explicit, root),
                ({"max_total_lines": 300}, root / "sc-lint.toml"),
            )

    def test_resolve_table_rejects_non_table_value(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / "sc-lint.toml").write_text("line_counts = 5\n", encoding="utf-8")
            with self.assertRaises(AdapterError) as caught:
                resolve_table("line_counts", None, root)
            self.assertEqual(caught.exception.kind, "config")
            self.assertEqual(caught.exception.message, "[line_counts] must be a TOML table")

    def test_resolve_table_bad_toml_is_adapter_error(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / ".just").mkdir()
            (root / "sc-lint.toml").write_text("[line_counts\n", encoding="utf-8")
            (root / ".just/lint-config.toml").write_text("[line_counts]\n", encoding="utf-8")
            with self.assertRaises(AdapterError) as caught:
                resolve_table("line_counts", None, root)
            self.assertEqual(caught.exception.kind, "config")
            self.assertTrue(
                caught.exception.message.startswith("failed to parse sc-lint.toml: "),
                caught.exception.message,
            )

    def test_resolve_table_missing_lists_searched_files(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir, tempfile.TemporaryDirectory() as outside:
            root = Path(tempdir)
            (root / "sc-lint.toml").write_text("[logging]\nconsole = false\n", encoding="utf-8")
            with self.assertRaises(ConfigError) as caught:
                resolve_table("line_counts", None, root)
            self.assertEqual(caught.exception.code, CONFIG_TABLE_MISSING)
            self.assertEqual(caught.exception.table, "line_counts")
            self.assertEqual(caught.exception.searched, ["sc-lint.toml", ".just/lint-config.toml"])
            self.assertEqual(
                caught.exception.message,
                "configuration table [line_counts] not found in any of: "
                "sc-lint.toml, .just/lint-config.toml",
            )
            explicit = Path(outside) / "explicit.toml"
            explicit.write_text("[identities]\n", encoding="utf-8")
            with self.assertRaises(ConfigError) as caught:
                resolve_table("line_counts", explicit, root)
            self.assertEqual(
                caught.exception.searched,
                [explicit.resolve().as_posix(), "sc-lint.toml", ".just/lint-config.toml"],
            )

    def test_config_table_missing_constant_is_wire_string(self) -> None:
        self.assertEqual(CONFIG_TABLE_MISSING, "config.table_missing")

    def test_config_errors_use_standard_envelope(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / "sc-lint.toml").write_text("[logging]\nconsole = false\n", encoding="utf-8")
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                result = line_counts_main(["lint_line_counts.py", "--root", str(root), "--json"])
            self.assertEqual(result, 3)
            self.assertEqual(
                json.loads(stdout.getvalue())["error"],
                {
                    "kind": "config",
                    "message": (
                        "configuration table [line_counts] not found in any of: "
                        "sc-lint.toml, .just/lint-config.toml"
                    ),
                    "details": {
                        "code": "config.table_missing",
                        "table": "line_counts",
                        "searched": ["sc-lint.toml", ".just/lint-config.toml"],
                    },
                    "suggested_action": "add the table to `sc-lint.toml` or `.just/lint-config.toml`",
                },
            )

    def test_table_missing_error_uses_the_documented_recovery_literal(self) -> None:
        error = table_missing_error(
            ConfigError(
                code=CONFIG_TABLE_MISSING,
                table="line_counts",
                searched=["sc-lint.toml", ".just/lint-config.toml"],
            )
        )

        self.assertEqual(
            error.suggested_action,
            "add the table to `sc-lint.toml` or `.just/lint-config.toml`",
        )

    def test_workspace_crates_reads_package_and_crate_path(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(ROOT_MANIFEST, encoding="utf-8")
            crates_dir = repo_root / "crates"
            boundary_dir = crates_dir / "sc-lint-boundary"
            boundary_dir.mkdir(parents=True)
            (boundary_dir / "Cargo.toml").write_text(
                """\
[package]
name = "sc-lint-boundary"
version = "0.1.0"

[lib]
name = "sc_lint_boundary"
""",
                encoding="utf-8",
            )
            attr_dir = crates_dir / "sc-lint-attributes"
            attr_dir.mkdir(parents=True)
            (attr_dir / "Cargo.toml").write_text(
                """\
[package]
name = "sc-lint-attributes"
version = "0.1.0"
""",
                encoding="utf-8",
            )

            crates = workspace_crates(repo_root)

            self.assertEqual(
                [(crate.crate_dir, crate.package_name, crate.crate_path_name) for crate in crates],
                [
                    ("sc-lint-attributes", "sc-lint-attributes", "sc_lint_attributes"),
                    ("sc-lint-boundary", "sc-lint-boundary", "sc_lint_boundary"),
                ],
            )

    def test_render_workspace_crate_table_supports_extra_columns(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                ROOT_MANIFEST.replace('"crates/sc-lint-attributes", ', ""),
                encoding="utf-8",
            )
            crate_dir = repo_root / "crates" / "sc-lint-boundary"
            crate_dir.mkdir(parents=True)
            (crate_dir / "Cargo.toml").write_text(
                """\
[package]
name = "sc-lint-boundary"
version = "0.1.0"

[lib]
name = "sc_lint_boundary"
""",
                encoding="utf-8",
            )

            lines = render_workspace_crate_table(
                repo_root,
                extra_columns=[("lint_mode", "lint_mode", lambda _crate: "check")],
            )

            self.assertIn("crate", lines[0])
            self.assertIn("lint_mode", lines[0])
            self.assertTrue(any("sc-lint-boundary" in line and "check" in line for line in lines))

    def test_workspace_crate_section_lines_wraps_table(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                ROOT_MANIFEST.replace('"crates/sc-lint-attributes", ', ""),
                encoding="utf-8",
            )
            crate_dir = repo_root / "crates" / "sc-lint-boundary"
            crate_dir.mkdir(parents=True)
            (crate_dir / "Cargo.toml").write_text(
                """\
[package]
name = "sc-lint-boundary"
version = "0.1.0"
""",
                encoding="utf-8",
            )

            lines = workspace_crate_section_lines(repo_root, title="inventory:")

            self.assertEqual(lines[0], "inventory:")
            self.assertIn("crate", lines[1])
            self.assertIn("manifest", lines[1])
            self.assertEqual(lines[-1], "")

    def test_line_is_suppressed_supports_tool_key_and_rule_aliases(self) -> None:
        lines = [
            "// lint-portability: allow-next-line",
            "let _ = \"/Users/example/.config\";",
            "// rule-003: allow-start",
            "let _ = std::env::set_var(\"HOME\", \"/tmp\");",
            "// lint-portability: allow-end",
            "let _ = std::env::set_var(\"HOME\", \"/var/tmp\");",
        ]
        policy = LintDirectivePolicy(tool_key="portability", aliases=("rule-003",))

        self.assertTrue(line_is_suppressed(2, lines, policy))
        self.assertTrue(line_is_suppressed(4, lines, policy))
        self.assertFalse(line_is_suppressed(6, lines, policy))

    def test_line_is_suppressed_supports_multiline_comment_directives(self) -> None:
        lines = [
            "/* lint-portability: allow-start */",
            "let _ = \"/Users/example/.config\";",
            "* lint-portability: allow-end",
            "let _ = std::env::set_var(\"HOME\", \"/tmp\");",
        ]
        policy = LintDirectivePolicy(tool_key="portability", aliases=("rule-003",))

        self.assertTrue(line_is_suppressed(2, lines, policy))
        self.assertFalse(line_is_suppressed(4, lines, policy))

    def test_classify_rust_test_scope_marks_cfg_test_block_only(self) -> None:
        lines = [
            "pub fn production() {}",
            "#[cfg(test)]",
            "mod tests {",
            "    #[test]",
            "    fn example() {}",
            "}",
        ]

        scope = classify_rust_test_scope(lines)

        self.assertEqual(scope, [False, True, True, True, True, True])

    def test_iter_string_literal_contents_supports_rust_unicode_and_char_escapes(self) -> None:
        line = f'let a = "team\\nlead"; let b = r#"{TEAM_LEAD_IDENTITY}"#; let c = "\\u{{1F600}}"; let d = \'\\u{{7}}\';'
        self.assertEqual(
            iter_string_literal_contents(line),
            ["team\nlead", TEAM_LEAD_IDENTITY, "😀", "\a"],
        )

    def test_iter_string_literal_contents_preserves_unknown_rust_escapes(self) -> None:
        self.assertEqual(iter_string_literal_contents(r'let value = "\\q";'), [r"\q"])

    def test_iter_string_literal_contents_does_not_treat_string_apostrophes_as_chars(self) -> None:
        self.assertEqual(iter_string_literal_contents('let value = "don\'t";'), ["don't"])

    def test_iter_workspace_rust_files_includes_src_and_tests(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(
                """\
[workspace]
members = ["crates/demo"]
resolver = "2"
""",
                encoding="utf-8",
            )
            crate_root = repo_root / "crates/demo"
            (crate_root / "src").mkdir(parents=True)
            (crate_root / "tests").mkdir(parents=True)
            (crate_root / "Cargo.toml").write_text(
                """\
[package]
name = "demo"
version = "0.1.0"
""",
                encoding="utf-8",
            )
            (crate_root / "src/lib.rs").write_text("pub fn run() {}\n", encoding="utf-8")
            (crate_root / "tests/basic.rs").write_text("#[test]\nfn smoke() {}\n", encoding="utf-8")

            files = [path.relative_to(repo_root).as_posix() for path in iter_workspace_rust_files(repo_root)]

            self.assertCountEqual(files, ["crates/demo/src/lib.rs", "crates/demo/tests/basic.rs"])

    def test_rust_file_test_scope_marks_tests_tree_as_test(self) -> None:
        lines = ["#[test]", "fn smoke() {}"]
        scope = rust_file_test_scope(Path("crates/demo/tests/basic.rs"), lines)
        self.assertEqual(scope, [True, True])


if __name__ == "__main__":
    unittest.main()
