from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from lint_identity_literals import collect_identity_violations
from lint_identity_literals import load_forbidden_literals
from lint_identity_literals import load_production_canonical_literals
from lint_identity_literals import main


class IdentityLiteralTests(unittest.TestCase):
    def test_identity_literal_lint_flags_test_and_production_cases(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / ".just/lint-config.toml").write_text(
                """\
[identities]
forbidden_literals = ["team-lead"]

[identities.production_canonical_literals]
"team-lead" = ["crates/demo/src/constants.rs"]
""",
                encoding="utf-8",
            )
            forbidden = load_forbidden_literals(repo_root)
            canonical = load_production_canonical_literals(repo_root)

            violations = collect_identity_violations(
                repo_root,
                forbidden_literals=forbidden,
                production_canonical_literals=canonical,
            )

            self.assertEqual(len(violations), 3)
            rendered = [item.render() for item in violations]
            self.assertTrue(any("tests.rs" in item for item in rendered))
            self.assertTrue(any("lib.rs" in item for item in rendered))

    def test_identity_literal_defaults_do_not_leak_consumer_policy(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / ".just/lint-config.toml").write_text("", encoding="utf-8")

            result = main(["lint_identity_literals.py", "--root", str(repo_root), "--json"])

            self.assertEqual(result, 0)
            summary = json.loads(
                (repo_root / "artifacts/findings/identity-literals/summary.json").read_text(encoding="utf-8")
            )
            self.assertEqual(summary["forbidden_literals"], [])
            self.assertEqual(summary["production_canonical_literals"], {})


def seed_workspace(repo_root: Path) -> Path:
    (repo_root / ".just").mkdir(parents=True, exist_ok=True)
    (repo_root / "Cargo.toml").write_text(
        """\
[workspace]
members = ["crates/demo"]
resolver = "2"
""",
        encoding="utf-8",
    )
    crate_root = repo_root / "crates/demo"
    (crate_root / "src").mkdir(parents=True, exist_ok=True)
    (crate_root / "tests").mkdir(parents=True, exist_ok=True)
    (crate_root / "Cargo.toml").write_text(
        """\
[package]
name = "demo"
version = "0.1.0"
edition = "2024"
""",
        encoding="utf-8",
    )
    (crate_root / "src/constants.rs").write_text('pub const OWNER: &str = "team-lead";\n', encoding="utf-8")
    (crate_root / "src/lib.rs").write_text(
        """\
pub fn owner() -> &'static str {
    "team-lead"
}
""",
        encoding="utf-8",
    )
    (crate_root / "tests/tests.rs").write_text(
        """\
#[test]
fn uses_identity_literal() {
    let owner = "team-lead";
    assert_eq!(owner, "team-lead");
}
""",
        encoding="utf-8",
    )
    return repo_root


if __name__ == "__main__":
    unittest.main()
