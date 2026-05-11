from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from lint_common import build_report
from lint_common import classify_rust_test_scope
from lint_common import iter_string_literal_contents
from lint_common import iter_workspace_rust_files
from lint_common import LintDirectivePolicy
from lint_common import lint_slug
from lint_common import line_is_suppressed
from lint_common import load_lint_config
from lint_common import make_log_path
from lint_common import relative_log_path
from lint_common import render_workspace_crate_table
from lint_common import rust_file_test_scope
from lint_common import workspace_crate_section_lines
from lint_common import workspace_crates
from fixture_constants import TEAM_LEAD_IDENTITY


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

    def test_iter_string_literal_contents_supports_raw_and_escaped_literals(self) -> None:
        line = f'let a = "team\\nlead"; let b = r#"{TEAM_LEAD_IDENTITY}"#;'
        self.assertEqual(
            iter_string_literal_contents(line),
            ["team\nlead", TEAM_LEAD_IDENTITY],
        )

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
