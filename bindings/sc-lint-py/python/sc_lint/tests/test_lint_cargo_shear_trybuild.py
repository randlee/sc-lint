from __future__ import annotations

import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from sc_lint import lint_cargo_shear
from sc_lint.lint_cargo_shear import ShearSection
from sc_lint.lint_cargo_shear import evaluate_policy
from sc_lint.lint_cargo_shear import main
from sc_lint.lint_cargo_shear import parse_sections
from sc_lint.lint_cargo_shear import render_consumed_notes
from sc_lint.lint_cargo_shear import trybuild_consumed
from sc_lint.lint_cargo_shear import trybuild_harness_globs


POLICY = {
    "allowed_empty_files": {},
    "allowed_unlinked_files": {},
    "allowed_unused_dependencies": {},
}


def _write(root: Path, relative: str, content: str = "") -> Path:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def _unlinked_section(package: str, *paths: str) -> ShearSection:
    body_lines = (f"  ⚠ {len(paths)} unlinked file(s) in `{package}`",)
    return ShearSection("unlinked_files", "shear/unlinked_files", body_lines, tuple(paths))


def _unlinked_stdout(package: str, *paths: str) -> str:
    rows = "".join(f"  │ {path}\n" for path in paths)
    return f"shear/unlinked_files\n\n  ⚠ {len(paths)} unlinked file(s) in `{package}`\n{rows}"


class TrybuildDetectionTests(unittest.TestCase):
    def test_trybuild_globs_found_for_pass_and_compile_fail(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(
                root,
                "tests/ui.rs",
                'fn tests() {\n    t.compile_fail("tests/ui/*.rs");\n'
                '    t.pass("tests/pass/ok.rs");\n}\n',
            )

            self.assertEqual(
                trybuild_harness_globs(root, frozenset()),
                [
                    ("tests/ui/*.rs", "tests/ui.rs", 2),
                    ("tests/pass/ok.rs", "tests/ui.rs", 3),
                ],
            )

    def test_trybuild_raw_string_glob_found(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 't.compile_fail(r"tests/ui/*.rs");\nt.pass(r#"tests/pass/*.rs"#);\n')

            self.assertEqual(
                trybuild_harness_globs(root, frozenset()),
                [
                    ("tests/ui/*.rs", "tests/ui.rs", 1),
                    ("tests/pass/*.rs", "tests/ui.rs", 2),
                ],
            )

    def test_trybuild_non_literal_glob_ignored(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", "t.compile_fail(PATTERN);\nt.compile_fail(&format!(\"{}\", PATTERN));\n")

            self.assertEqual(trybuild_harness_globs(root, frozenset()), [])

    def test_consumed_fixture_resolved_against_package_dir(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            package = root / "crates/demo"
            _write(package, "tests/ui.rs", 't.compile_fail("tests/ui/*.rs");\n')
            _write(package, "tests/ui/a.rs", "compile_error!(\"a\");")
            _write(package, "tests/ui/b.rs", "compile_error!(\"b\");")
            sections = [_unlinked_section("demo", "tests/ui/a.rs", "tests/ui/b.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "crates/demo"}):
                consumed = trybuild_consumed(sections, root)

            self.assertEqual(
                consumed,
                {
                    (0, "tests/ui/a.rs"): "tests/ui.rs:1",
                    (0, "tests/ui/b.rs"): "tests/ui.rs:1",
                },
            )

    def test_unmatched_unlinked_file_not_consumed(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            package = root / "crates/demo"
            _write(package, "tests/ui.rs", 't.compile_fail("tests/ui/*.rs");\n')
            _write(package, "tests/ui/a.rs")
            _write(package, "src/stray.rs")
            sections = [_unlinked_section("demo", "tests/ui/a.rs", "src/stray.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "crates/demo"}):
                consumed = trybuild_consumed(sections, root)

            self.assertEqual(consumed, {(0, "tests/ui/a.rs"): "tests/ui.rs:1"})

    def test_absolute_and_parent_dir_globs_not_interpreted(self) -> None:
        globs = (
            "/tmp/tests/*.rs",
            "C:/tests/*.rs",
            "tests/../src/*.rs",
            r"tests\ui\*.rs",
        )
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            calls = "\n".join(f't.compile_fail(r"{glob}");' for glob in globs)
            _write(root, "tests/ui.rs", calls)
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("demo", "tests/ui/a.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}):
                consumed = trybuild_consumed(sections, root)

            self.assertEqual(consumed, {})

    def test_invalid_glob_is_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 't.pass("");\nt.pass("bad[");\n')
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("demo", "tests/ui/a.rs")]
            original_glob = Path.glob

            def glob_or_fail(path: Path, pattern: str, **kwargs):
                if pattern == "bad[":
                    raise ValueError("invalid glob")
                return original_glob(path, pattern, **kwargs)

            with (
                patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}),
                patch.object(Path, "glob", glob_or_fail),
            ):
                self.assertEqual(trybuild_consumed(sections, root), {})

    def test_unrelated_pass_call_matches_nothing_and_does_not_crash(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 'custom.pass("tests/ui/missing.rs");\n')
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("demo", "tests/ui/a.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}):
                self.assertEqual(trybuild_consumed(sections, root), {})

    def test_unlinked_fixture_cannot_consume_itself(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 't.pass("tests/ui.rs");\n')
            sections = [_unlinked_section("demo", "tests/ui.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}):
                self.assertEqual(trybuild_consumed(sections, root), {})

    def test_target_directory_is_not_scanned(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "target/generated/tests.rs", 't.pass("tests/ui/a.rs");\n')
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("demo", "tests/ui/a.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}):
                self.assertEqual(trybuild_consumed(sections, root), {})

    def test_first_harness_wins_by_sorted_path_then_line(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/z.rs", 't.pass("tests/ui/*.rs");\n')
            _write(root, "tests/a.rs", 't.pass("tests/ui/*.rs");\nt.pass("tests/ui/*.rs");\n')
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("demo", "tests/ui/a.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}):
                consumed = trybuild_consumed(sections, root)

            self.assertEqual(consumed, {(0, "tests/ui/a.rs"): "tests/a.rs:1"})

    def test_directory_match_is_not_consumed(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 't.pass("tests/ui/*");\n')
            (root / "tests/ui/directory.rs").mkdir(parents=True)
            sections = [_unlinked_section("demo", "tests/ui/directory.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"demo": "."}):
                self.assertEqual(trybuild_consumed(sections, root), {})

    def test_unknown_package_leaves_section_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 't.pass("tests/ui/a.rs");\n')
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("unknown", "tests/ui/a.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={}):
                self.assertEqual(trybuild_consumed(sections, root), {})

    def test_root_package_note_has_no_dot_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            _write(root, "tests/ui.rs", 't.pass("tests/ui/a.rs");\n')
            _write(root, "tests/ui/a.rs")
            sections = [_unlinked_section("workspace", "tests/ui/a.rs")]
            with patch.object(lint_cargo_shear, "package_to_crate_dir", return_value={"workspace": "."}):
                consumed = trybuild_consumed(sections, root)
                notes = render_consumed_notes(sections, consumed, root)

            self.assertEqual(
                notes,
                ["shear note: tests/ui/a.rs consumed by trybuild harness tests/ui.rs:1 [unlinked_files]"],
            )


class TrybuildPolicyTests(unittest.TestCase):
    def test_policy_drops_consumed_and_keeps_other_unlinked(self) -> None:
        sections = [_unlinked_section("demo", "tests/ui/a.rs", "src/stray.rs")]
        findings, downgraded = evaluate_policy(sections, POLICY, {(0, "tests/ui/a.rs"): "tests/ui.rs:1"})

        self.assertEqual([(finding.section_name, finding.file_path) for finding in findings], [("unlinked_files", "src/stray.rs")])
        self.assertEqual(downgraded, [])

    def test_consumed_wins_over_allowlist_without_downgrade_line(self) -> None:
        sections = [_unlinked_section("demo", "tests/ui/a.rs")]
        policy = {**POLICY, "allowed_unlinked_files": {"tests/ui/a.rs": "legacy reason"}}

        findings, downgraded = evaluate_policy(sections, policy, {(0, "tests/ui/a.rs"): "tests/ui.rs:1"})

        self.assertEqual(findings, [])
        self.assertEqual(downgraded, [])

    def test_default_consumed_argument_changes_nothing(self) -> None:
        sections = [_unlinked_section("demo", "src/stray.rs")]

        without_argument = evaluate_policy(sections, POLICY)
        with_empty_argument = evaluate_policy(sections, POLICY, {})

        self.assertEqual(without_argument, with_empty_argument)
        self.assertEqual(len(without_argument[0]), 1)


class TrybuildMainTests(unittest.TestCase):
    def _run_main(self, root: Path, shear_stdout: str, returncode: int) -> tuple[int, str]:
        output = io.StringIO()
        completed = subprocess.CompletedProcess(["cargo-shear"], returncode, shear_stdout, "")
        package_map = {"demo": "crates/demo", "workspace": "."}
        with (
            patch.object(lint_cargo_shear, "discover_repo_root", return_value=root),
            patch.object(lint_cargo_shear, "configured_unused_dependency_allowlist", return_value=(False, {})),
            patch.object(lint_cargo_shear, "load_policy_config", return_value=POLICY),
            patch.object(lint_cargo_shear, "package_to_crate_dir", return_value=package_map),
            patch.object(lint_cargo_shear, "workspace_crate_section_lines", return_value=[]),
            patch("sc_lint.lint_cargo_shear.shutil.which", return_value="cargo-shear"),
            patch("sc_lint.lint_cargo_shear.subprocess.run", return_value=completed),
            patch.object(lint_cargo_shear.sys, "stdout", output),
            patch.object(lint_cargo_shear.sys, "stderr", io.StringIO()),
            patch.object(lint_cargo_shear, "manifest_to_package_name", return_value={"crates/demo/Cargo.toml": "demo"}),
        ):
            result = main(["lint_cargo_shear.py", "--root", str(root)])
        return result, output.getvalue()

    def test_main_prints_consumed_note_and_passes(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            package = root / "crates/demo"
            _write(package, "tests/ui.rs", 't.compile_fail("tests/ui/*.rs");\n')
            _write(package, "tests/ui/a.rs")

            result, output = self._run_main(root, _unlinked_stdout("demo", "tests/ui/a.rs"), 0)

            self.assertEqual(result, 0)
            self.assertIn("shear note: crates/demo/tests/ui/a.rs consumed by trybuild harness tests/ui.rs:1", output)

    def test_main_still_fails_on_ordinary_unlinked_file(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)

            result, output = self._run_main(root, _unlinked_stdout("demo", "src/stray.rs"), 0)

            self.assertEqual(result, 1)
            self.assertIn("shear policy error: src/stray.rs [unlinked_files]", output)

    def test_main_prints_consumed_note_on_unused_dependency_exit_1_path(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            package = root / "crates/demo"
            _write(package, "tests/ui.rs", 't.pass("tests/ui/a.rs");\n')
            _write(package, "tests/ui/a.rs")
            shear_stdout = (
                "shear/unused_dependency\n\n"
                "  × unused dependency `serde`\n"
                "    ╭─[crates/demo/Cargo.toml:5:1]\n\n"
                + _unlinked_stdout("demo", "tests/ui/a.rs")
            )

            result, output = self._run_main(root, shear_stdout, 1)

            self.assertEqual(result, 1)
            self.assertIn("shear note: crates/demo/tests/ui/a.rs consumed by trybuild harness tests/ui.rs:1", output)
            self.assertIn("shear policy error: demo.serde [unused_dependency]", output)
            self.assertEqual(output.count("shear policy error:"), 1)


if __name__ == "__main__":
    unittest.main()
