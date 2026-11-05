from __future__ import annotations

import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest

from sc_lint.lint_token_boundary import collect_violations
from sc_lint.lint_token_boundary import load_config
from sc_lint.lint_token_boundary import main


class TokenBoundaryTests(unittest.TestCase):
    def test_flags_production_token_use_but_allows_paths_and_test_scope(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "crates/demo/src/lib.rs").write_text(
                """pub fn allowed() { OBSERVE_RUNTIME(); }
#[cfg(test)]
mod tests {
    fn test_use() { OBSERVE_RUNTIME(); }
}
""",
                encoding="utf-8",
            )
            (root / "crates/demo/src/rogue.rs").write_text(
                "pub fn bad() { OBSERVE_RUNTIME(); }\n", encoding="utf-8"
            )
            (root / "crates/demo/tests/integration.rs").write_text(
                "fn integration_use() { OBSERVE_RUNTIME(); }\n", encoding="utf-8"
            )
            write_config(root, allowed_paths=["crates/demo/src/lib.rs"])
            config = load_config(root)

            self.assertEqual(
                [(item.path, item.line_number) for item in collect_violations(root, config)],
                [("crates/demo/src/rogue.rs", 1)],
            )

            write_config(root, allowed_paths=["crates/demo/src/lib.rs", "crates/demo/src/rogue.rs"])
            self.assertEqual(collect_violations(root, load_config(root)), [])

    def test_required_positive_missing_file_or_symbol_fails_then_restores(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            write_config(root, required_positive=[("crates/demo/src/lib.rs", "runtime_observer")])

            findings = collect_violations(root, load_config(root))
            self.assertEqual([(item.kind, item.line_number) for item in findings], [("required_positive_missing", 0)])

            (root / "crates/demo/src/lib.rs").write_text(
                "pub fn runtime_observer() {}\n", encoding="utf-8"
            )
            self.assertEqual(collect_violations(root, load_config(root)), [])

    def test_invalid_table_returns_structured_config_error(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "sc-lint.toml").write_text(
                '[token_boundary]\ntokens = "OBSERVE_RUNTIME"\nallowed_paths = []\nrequired_positive = []\n',
                encoding="utf-8",
            )
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                result = main(["lint_token_boundary.py", "--root", str(root), "--json"])

            self.assertEqual(result, 3)
            payload = json.loads(stdout.getvalue())
            self.assertFalse(payload["ok"])
            self.assertEqual(payload["error"]["kind"], "config")
            self.assertIn("tokens", payload["error"]["message"])

    def test_missing_table_is_structured_config_error(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                result = main(["lint_token_boundary.py", "--root", str(root), "--json"])

            self.assertEqual(result, 3)
            error = json.loads(stdout.getvalue())["error"]
            self.assertEqual(error["details"]["code"], "config.table_missing")
            self.assertEqual(error["details"]["table"], "token_boundary")

    def test_json_cli_reports_findings_and_nonzero_status(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = seed_workspace(Path(tempdir))
            (root / "crates/demo/src/lib.rs").write_text(
                "pub fn observe() { OBSERVE_RUNTIME(); }\n", encoding="utf-8"
            )
            write_config(root)
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                result = main(["lint_token_boundary.py", "--root", str(root), "--json"])

            self.assertEqual(result, 1)
            payload = json.loads(stdout.getvalue())
            self.assertTrue(payload["ok"])
            self.assertEqual(payload["data"]["status"], "fail")
            self.assertEqual(len(payload["data"]["findings"]), 1)


def write_config(
    root: Path,
    *,
    allowed_paths: list[str] | None = None,
    required_positive: list[tuple[str, str]] | None = None,
) -> None:
    allowed = allowed_paths or []
    required = required_positive or []
    allowed_items = ", ".join(json.dumps(path) for path in allowed)
    required_items = "\n".join(
        f'[[token_boundary.required_positive]]\npath = {json.dumps(path)}\nsymbol = {json.dumps(symbol)}'
        for path, symbol in required
    )
    required_config = "" if required else "required_positive = []\n"
    (root / ".just/lint-config.toml").write_text(
        f'[token_boundary]\ntokens = ["OBSERVE_RUNTIME"]\nallowed_paths = [{allowed_items}]\n'
        f"{required_config}{required_items}\n",
        encoding="utf-8",
    )


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
