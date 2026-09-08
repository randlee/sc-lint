from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from sc_lint.lint_function_length import FAIL_THRESHOLD
from sc_lint.lint_function_length import classify
from sc_lint.lint_function_length import find_function_spans
from sc_lint.lint_function_length import is_test_only_path


class FunctionLengthTests(unittest.TestCase):
    def test_comments_and_whitespace_do_not_count(self) -> None:
        function = inspect_source(
            """\
pub fn example() {

    // a comment
    /* a block comment */
    let braces = \"{ not a brace }\";
    braces.len()
}
"""
        )
        self.assertEqual(function.code_lines, 4)

    def test_multiline_comments_and_literal_braces_do_not_change_the_span(self) -> None:
        function = inspect_source(
            """\
pub fn example() {
    /* opening comment {
       nested /* comment */ still comment }
    */
    let raw = r#"{ not a brace }"#;
    let character = '}';
}
"""
        )
        self.assertEqual(function.end_line, 7)
        self.assertEqual(function.code_lines, 4)

    def test_exactly_default_threshold_is_a_failure(self) -> None:
        body = "\n".join("    value += 1;" for _ in range(FAIL_THRESHOLD - 2))
        function = inspect_source(f"pub fn example() {{\n{body}\n}}\n")
        self.assertEqual(function.code_lines, FAIL_THRESHOLD)
        self.assertEqual(classify([function]).failures, (function,))

    def test_test_attribute_and_test_path_are_exempt(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            path = Path(tempdir) / "tests.rs"
            path.write_text("#[test]\nfn example() {}\n", encoding="utf-8")
            self.assertEqual(find_function_spans(path), [])
        self.assertTrue(is_test_only_path(Path("crates/demo/tests/example.rs")))
        self.assertTrue(is_test_only_path(Path("crates/demo/src/test_helper.rs")))

    def test_explicit_fail_at_override_is_used(self) -> None:
        body = "\n".join("    value += 1;" for _ in range(98))
        function = inspect_source(
            f"#[sc_lint(function_length.fail_at(120))]\npub fn example() {{\n{body}\n}}\n"
        )
        self.assertEqual(function.fail_threshold, 120)
        self.assertEqual(classify([function]).failures, ())

    def test_malformed_function_length_attribute_fails(self) -> None:
        with self.assertRaisesRegex(Exception, "function_length.fail_at"):
            inspect_source("#[sc_lint(function_length.fail_at(0))]\nfn example() {}\n")


def inspect_source(source: str):
    with tempfile.TemporaryDirectory() as tempdir:
        path = Path(tempdir) / "lib.rs"
        path.write_text(source, encoding="utf-8")
        functions = find_function_spans(path)
        assert len(functions) == 1
        return functions[0]


if __name__ == "__main__":
    unittest.main()
