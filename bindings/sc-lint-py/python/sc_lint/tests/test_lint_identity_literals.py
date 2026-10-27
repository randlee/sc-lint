from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import unittest


from sc_lint.lint_identity_literals import collect_identity_violations
from sc_lint.lint_identity_literals import load_forbidden_literals
from sc_lint.lint_identity_literals import load_production_canonical_literals
from sc_lint.lint_identity_literals import main
from sc_lint.fixture_constants import TEAM_LEAD_IDENTITY


class IdentityLiteralTests(unittest.TestCase):
    def test_identity_literal_lint_flags_test_and_production_cases(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / ".just/lint-config.toml").write_text(
                f"""\
[identities]
forbidden_literals = ["{TEAM_LEAD_IDENTITY}"]

[identities.production_canonical_literals]
"{TEAM_LEAD_IDENTITY}" = ["crates/demo/src/constants.rs"]
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

    def test_identity_literals_table_falls_back(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / "sc-lint.toml").write_text("[logging]\nconsole = false\n", encoding="utf-8")
            (repo_root / ".just/lint-config.toml").write_text('[identities]\nforbidden_literals = ["x"]\n', encoding="utf-8")
            self.assertEqual(load_forbidden_literals(repo_root), ("x",))

    def test_identity_literal_lint_detects_literal_concat_without_false_positives(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            split_at = len(TEAM_LEAD_IDENTITY) // 2
            prefix, suffix = TEAM_LEAD_IDENTITY[:split_at], TEAM_LEAD_IDENTITY[split_at:]
            (repo_root / ".just/lint-config.toml").write_text(
                f'''[identities]
forbidden_literals = ["{TEAM_LEAD_IDENTITY}"]

[identities.production_canonical_literals]
"{TEAM_LEAD_IDENTITY}" = ["crates/demo/src/constants.rs"]
''',
                encoding="utf-8",
            )
            (repo_root / "crates/demo/src/lib.rs").write_text(
                f'''pub const SINGLE: &str = concat!("{prefix}", "{suffix}");
pub const MULTI: &str = concat!(
    "{prefix}",
    /* comments between literal arguments are allowed */ "{suffix}",
);
pub const NONLITERAL: &str = concat!("{prefix}", IDENTITY_SUFFIX, "{suffix}");
pub const ORDINARY: &str = "concat!(\\"{prefix}\\", \\"{suffix}\\")";
// concat!("{prefix}", "{suffix}")
/* concat!("{prefix}", "{suffix}") */
''',
                encoding="utf-8",
            )
            (repo_root / "crates/demo/src/constants.rs").write_text(
                f'pub const OWNER: &str = concat!("{prefix}", "{suffix}");\n',
                encoding="utf-8",
            )
            (repo_root / "crates/demo/tests/tests.rs").write_text(
                f'''#[test]
fn uses_split_identity() {{
    let owner = concat!("{prefix}", "{suffix}");
    // lint-identity-literals: allow-next-line
    let allowed = concat!("{prefix}", "{suffix}");
    let dynamic = concat!("{prefix}", IDENTITY_SUFFIX, "{suffix}");
    let ordinary = "concat!(\\"{prefix}\\", \\"{suffix}\\")";
    // concat!("{prefix}", "{suffix}")
}}
''',
                encoding="utf-8",
            )

            violations = collect_identity_violations(
                repo_root,
                forbidden_literals=(TEAM_LEAD_IDENTITY,),
                production_canonical_literals={TEAM_LEAD_IDENTITY: ("crates/demo/src/constants.rs",)},
            )

            self.assertEqual(
                [(item.path, item.line_number, item.kind) for item in violations],
                [
                    ("crates/demo/src/lib.rs", 1, "production_scope_canonical_literal"),
                    ("crates/demo/src/lib.rs", 2, "production_scope_canonical_literal"),
                    ("crates/demo/tests/tests.rs", 3, "test_scope_forbidden_literal"),
                ],
            )

    def test_identity_literal_lint_handles_trailing_concat_comments_once(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            split_at = len(TEAM_LEAD_IDENTITY) // 2
            prefix, suffix = TEAM_LEAD_IDENTITY[:split_at], TEAM_LEAD_IDENTITY[split_at:]
            (repo_root / "crates/demo/src/lib.rs").write_text(
                f'''pub const SINGLE: &str = concat!("{TEAM_LEAD_IDENTITY}");
pub const LINE_COMMENT: &str = concat!("{prefix}", "{suffix}", // keep
);
pub const BLOCK_COMMENT: &str = concat!("{prefix}", "{suffix}", /* keep */);
''',
                encoding="utf-8",
            )

            violations = collect_identity_violations(
                repo_root,
                forbidden_literals=(),
                production_canonical_literals={
                    TEAM_LEAD_IDENTITY: ("crates/demo/src/constants.rs",),
                },
            )

            self.assertEqual(
                [(item.path, item.line_number, item.kind) for item in violations],
                [
                    ("crates/demo/src/lib.rs", 1, "production_scope_canonical_literal"),
                    ("crates/demo/src/lib.rs", 2, "production_scope_canonical_literal"),
                    ("crates/demo/src/lib.rs", 4, "production_scope_canonical_literal"),
                ],
            )

    def test_identity_literal_lint_keeps_distinct_same_line_concat_findings(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            split_at = len(TEAM_LEAD_IDENTITY) // 2
            prefix, suffix = TEAM_LEAD_IDENTITY[:split_at], TEAM_LEAD_IDENTITY[split_at:]
            (repo_root / "crates/demo/src/lib.rs").write_text(
                f'''pub const DIRECT: &str = "{TEAM_LEAD_IDENTITY}"; pub const ASSEMBLED: &str = concat!("{prefix}", "{suffix}");
''',
                encoding="utf-8",
            )
            (repo_root / "crates/demo/tests/tests.rs").write_text(
                f'''#[test]
fn detects_both() {{ let direct = "{TEAM_LEAD_IDENTITY}"; let assembled = concat!("{prefix}", "{suffix}"); }}
''',
                encoding="utf-8",
            )

            violations = collect_identity_violations(
                repo_root,
                forbidden_literals=(TEAM_LEAD_IDENTITY,),
                production_canonical_literals={
                    TEAM_LEAD_IDENTITY: ("crates/demo/src/constants.rs",),
                },
            )

            self.assertEqual(
                [(item.path, item.line_number, item.kind) for item in violations],
                [
                    ("crates/demo/src/lib.rs", 1, "production_scope_canonical_literal"),
                    ("crates/demo/src/lib.rs", 1, "production_scope_canonical_literal"),
                    ("crates/demo/tests/tests.rs", 2, "test_scope_forbidden_literal"),
                    ("crates/demo/tests/tests.rs", 2, "test_scope_forbidden_literal"),
                ],
            )

    def test_identity_literal_defaults_do_not_leak_consumer_policy(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / ".just/lint-config.toml").write_text("[identities]\n", encoding="utf-8")

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
    (crate_root / "src/constants.rs").write_text(
        f'pub const OWNER: &str = "{TEAM_LEAD_IDENTITY}";\n',
        encoding="utf-8",
    )
    (crate_root / "src/lib.rs").write_text(
        f"""\
pub fn owner() -> &'static str {{
    "{TEAM_LEAD_IDENTITY}"
}}
""",
        encoding="utf-8",
    )
    (crate_root / "tests/tests.rs").write_text(
        f"""\
#[test]
fn uses_identity_literal() {{
    let owner = "{TEAM_LEAD_IDENTITY}";
    assert_eq!(owner, "{TEAM_LEAD_IDENTITY}");
}}
""",
        encoding="utf-8",
    )
    return repo_root


if __name__ == "__main__":
    unittest.main()
