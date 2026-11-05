from __future__ import annotations

import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest

from sc_lint.lint_fixed_sleep import collect_fixed_sleep_violations
from sc_lint.lint_fixed_sleep import load_allowed_paths
from sc_lint.lint_fixed_sleep import main


class FixedSleepTests(unittest.TestCase):
    def test_flags_supported_sleep_calls_in_test_scope_only(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "crates/demo/src/lib.rs").write_text(
                """pub fn production() { thread::sleep(duration); }
#[cfg(test)]
mod tests {
    fn first() { thread::sleep(duration); }
    fn second() { std::thread::sleep(duration); }
    fn third() { tokio::time::sleep(duration); }
    // thread::sleep(duration);
}
""",
                encoding="utf-8",
            )

            violations = collect_fixed_sleep_violations(root, allowed_paths=())

            self.assertEqual([item.line_number for item in violations], [4, 5, 6])

    def test_allowed_path_and_production_only_file_pass(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "crates/demo/tests/timing.rs").write_text(
                "fn test() { thread::sleep(duration); }\n", encoding="utf-8"
            )
            (root / "crates/demo/src/lib.rs").write_text(
                "pub fn production() { std::thread::sleep(duration); }\n", encoding="utf-8"
            )
            (root / ".just/lint-config.toml").write_text(
                '[fixed_sleep]\nallowed_paths = ["crates/demo/tests/timing.rs"]\n', encoding="utf-8"
            )

            self.assertEqual(load_allowed_paths(root), ("crates/demo/tests/timing.rs",))
            self.assertEqual(
                collect_fixed_sleep_violations(root, allowed_paths=load_allowed_paths(root)), []
            )

    def test_config_error_uses_structured_json_and_exit_status(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "sc-lint.toml").write_text('[fixed_sleep]\nallowed_paths = "bad"\n', encoding="utf-8")
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                result = main(["lint_fixed_sleep.py", "--root", str(root), "--json"])

            self.assertEqual(result, 3)
            payload = json.loads(stdout.getvalue())
            self.assertFalse(payload["ok"])
            self.assertEqual(payload["error"]["kind"], "config")
            self.assertIn("allowed_paths", payload["error"]["message"])

    def test_json_cli_reports_findings_and_nonzero_status(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "crates/demo/src/lib.rs").write_text(
                "#[cfg(test)]\nmod tests { fn waits() { thread::sleep(duration); } }\n",
                encoding="utf-8",
            )
            (root / "sc-lint.toml").write_text("[fixed_sleep]\nallowed_paths = []\n", encoding="utf-8")
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                result = main(["lint_fixed_sleep.py", "--root", str(root), "--json"])

            self.assertEqual(result, 1)
            payload = json.loads(stdout.getvalue())
            self.assertTrue(payload["ok"])
            self.assertEqual(payload["data"]["status"], "fail")
            self.assertEqual(len(payload["data"]["findings"]), 1)


def seed_workspace(root: Path) -> Path:
    (root / ".just").mkdir(parents=True, exist_ok=True)
    (root / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/demo"]\nresolver = "2"\n', encoding="utf-8"
    )
    crate = root / "crates/demo"
    (crate / "src").mkdir(parents=True)
    (crate / "tests").mkdir()
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "demo"\nversion = "0.1.0"\nedition = "2024"\n', encoding="utf-8"
    )
    (crate / "src/lib.rs").write_text("pub fn production() {}\n", encoding="utf-8")
    return root


if __name__ == "__main__":
    unittest.main()
