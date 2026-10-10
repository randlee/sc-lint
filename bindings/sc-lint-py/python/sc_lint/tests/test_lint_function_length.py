from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from sc_lint.lint_function_length import BASELINE_PATH
from sc_lint.lint_function_length import FAIL_THRESHOLD
from sc_lint.lint_function_length import WARN_THRESHOLD
from sc_lint.lint_function_length import evaluate
from sc_lint.lint_function_length import find_functions
from sc_lint.lint_function_length import load_baseline
from sc_lint.lint_function_length import mask_rust_source
from sc_lint.lint_function_length import scan_repo
from sc_lint.lint_function_length import write_baseline
from sc_lint.python_adapter import AdapterError


REPO_ROOT = Path(__file__).resolve().parents[5]
DEMO_PATH = Path("crates/demo/src/lib.rs")
DEMO_KEY = ("crates/demo/src/lib.rs", "example")


def body_of(line_count: int) -> str:
    return "\n".join("    value += 1;" for _ in range(line_count))


def functions_in(source: str):
    return find_functions(source, DEMO_PATH)


def function_named(source: str, name: str):
    matches = [function for function in functions_in(source) if function.name == name]
    assert len(matches) == 1, matches
    return matches[0]


def sized(name: str, line_count: int):
    return function_named(f"pub fn {name}() {{\n{body_of(line_count - 2)}\n}}\n", name)


class CountingTests(unittest.TestCase):
    def test_comments_and_whitespace_do_not_count(self) -> None:
        function = function_named(
            'pub fn example() {\n\n    // a comment\n    /* block */\n    let b = "{ x }";\n    b.len()\n}\n',
            "example",
        )
        self.assertEqual(function.code_lines, 4)

    def test_nested_comments_and_literal_braces_keep_the_span(self) -> None:
        function = function_named(
            'pub fn example() {\n    /* { nested /* c */ } */\n    let r = r#"{ no }"#;\n'
            "    let c = '}';\n}\n",
            "example",
        )
        self.assertEqual((function.end_line, function.code_lines), (5, 4))

    def test_lines_holding_only_literals_are_code(self) -> None:
        function = function_named('pub fn example() {\n    "{";\n    "text\n\nmore";\n}\n', "example")
        self.assertEqual((function.end_line, function.code_lines), (6, 5))

    def test_mask_keeps_literal_text_for_counting_only(self) -> None:
        code, structure = mask_rust_source('let a = "{x}"; // trailing\n')
        self.assertIn('"{x}"', code)
        self.assertNotIn("trailing", code)
        self.assertNotIn("{", structure)


class ThresholdTests(unittest.TestCase):
    def test_bands_are_silent_advisory_and_violation(self) -> None:
        cases = {
            WARN_THRESHOLD - 1: (0, 0),
            WARN_THRESHOLD: (1, 0),
            FAIL_THRESHOLD - 1: (1, 0),
            FAIL_THRESHOLD: (0, 1),
            FAIL_THRESHOLD + 1: (0, 1),
        }
        for line_count, (advisories, violations) in cases.items():
            with self.subTest(line_count=line_count):
                evaluation = evaluate([sized("example", line_count)], {})
                self.assertEqual(len(evaluation.advisories), advisories)
                self.assertEqual(len(evaluation.violations), violations)
                if violations:
                    self.assertEqual(evaluation.violations[0].reason, "new")


class AttributeTests(unittest.TestCase):
    def test_test_attributes_exempt_only_exact_forms(self) -> None:
        body = body_of(FAIL_THRESHOLD - 2)
        exempt = functions_in("#[test]\nfn a() {}\n#[tokio::test]\nasync fn b() {}\n")
        self.assertEqual(exempt, [])
        kept = functions_in(
            f'#[cfg(not(test))]\npub fn c() {{\n{body}\n}}\n'
            f'#[expect(clippy::too_many_lines, reason = "Tests cover this")]\npub fn d() {{\n{body}\n}}\n'
        )
        self.assertEqual([function.name for function in kept], ["c", "d"])

    def test_cfg_test_module_exempts_its_functions(self) -> None:
        body = body_of(FAIL_THRESHOLD)
        functions = functions_in(
            f"#[cfg(test)]\nmod tests {{\n    fn helper() {{\n{body}\n    }}\n}}\n\npub fn production() {{}}\n"
        )
        self.assertEqual([function.name for function in functions], ["production"])

    def test_attributes_do_not_leak_across_code_or_items(self) -> None:
        body = body_of(FAIL_THRESHOLD - 2)
        functions = functions_in(
            f"#[test]\nfn only_test() {{}}\n#[test]\nconst MARK: u32 = 1;\nfn production() {{\n{body}\n}}\n"
        )
        self.assertEqual([function.name for function in functions], ["production"])

    def test_multiline_attribute_and_bodiless_declarations(self) -> None:
        body = body_of(FAIL_THRESHOLD - 2)
        functions = functions_in(
            f"#[sc_lint(\n    boundary.internal_only,\n)]\npub fn example() {{\n{body}\n}}\n"
            "trait Shape {\n    fn area(&self) -> u32;\n}\n"
            "pub fn sized(buffer: [u8; 4]) {\n    let _ = buffer;\n}\n"
        )
        self.assertEqual([function.name for function in functions], ["example", "sized"])


class BaselineTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        (self.root / ".just").mkdir()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def test_unchanged_or_shrunk_over_cap_function_passes(self) -> None:
        function = sized("example", FAIL_THRESHOLD + 3)
        self.assertEqual(evaluate([function], {DEMO_KEY: function.code_lines}).violations, ())
        self.assertEqual(evaluate([function], {DEMO_KEY: function.code_lines + 10}).violations, ())

    def test_growth_past_the_recorded_length_fails(self) -> None:
        function = sized("example", FAIL_THRESHOLD + 3)
        evaluation = evaluate([function], {DEMO_KEY: function.code_lines - 1})
        self.assertEqual(
            [(v.reason, v.recorded_lines) for v in evaluation.violations],
            [("grown", FAIL_THRESHOLD + 2)],
        )

    def test_entry_no_longer_over_cap_is_stale_not_failing(self) -> None:
        evaluation = evaluate([sized("example", WARN_THRESHOLD)], {DEMO_KEY: FAIL_THRESHOLD + 1})
        self.assertEqual(evaluation.violations, ())
        self.assertEqual(evaluation.stale, (DEMO_KEY,))

    def test_write_then_load_round_trips_over_cap_functions(self) -> None:
        function = sized("example", FAIL_THRESHOLD + 1)
        self.assertEqual(write_baseline(self.root, [function]), 1)
        self.assertEqual(load_baseline(self.root), {DEMO_KEY: FAIL_THRESHOLD + 1})

    def test_missing_malformed_and_duplicate_baselines_are_config_errors(self) -> None:
        path = self.root / BASELINE_PATH
        with self.assertRaisesRegex(AdapterError, "--write-baseline"):
            load_baseline(self.root)
        for text, pattern in (
            ("crates/demo/src/lib.rs\texample\n", "path<TAB>name<TAB>code_lines"),
            ("crates/demo/src/lib.rs\texample\t80\n" * 2, "unique"),
        ):
            with self.subTest(pattern=pattern):
                path.write_text(text, encoding="utf-8")
                with self.assertRaisesRegex(AdapterError, pattern):
                    load_baseline(self.root)


class RepositoryTests(unittest.TestCase):
    @unittest.skipUnless((REPO_ROOT / "Cargo.toml").is_file(), "needs the source checkout")
    def test_current_tree_passes_the_recorded_baseline(self) -> None:
        violations = evaluate(scan_repo(REPO_ROOT), load_baseline(REPO_ROOT)).violations
        self.assertEqual([(v.function.path, v.function.name) for v in violations], [])


if __name__ == "__main__":
    unittest.main()
