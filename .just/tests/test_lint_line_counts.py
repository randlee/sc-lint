from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from lint_line_counts import collect_file_counts
from lint_line_counts import evaluate_limits
from lint_line_counts import load_config
from lint_line_counts import main


class LineCountsTests(unittest.TestCase):
    def test_line_count_limit_is_config_driven(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / ".just/lint-config.toml").write_text(
                """\
[line_counts]
max_production_lines = 1
""",
                encoding="utf-8",
            )

            config = load_config(repo_root)
            counts = collect_file_counts(repo_root, config)
            failures = evaluate_limits(counts, config)

            self.assertEqual(len(failures), 1)
            self.assertIn("exceeds limit 1", failures[0])

    def test_line_count_exclusion_skips_named_file(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = seed_workspace(Path(tempdir))
            (repo_root / ".just/lint-config.toml").write_text(
                """\
[line_counts]
max_production_lines = 1

[line_counts.exclusions]
"crates/demo/src/lib.rs" = "fixture exception"
""",
                encoding="utf-8",
            )

            result = main(["lint_line_counts.py", "--root", str(repo_root), "--json"])

            self.assertEqual(result, 0)
            summary = json.loads(
                (repo_root / "artifacts/findings/line-counts/summary.json").read_text(encoding="utf-8")
            )
            self.assertEqual(summary["findings"], [])


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
    crate_dir = repo_root / "crates/demo/src"
    crate_dir.mkdir(parents=True, exist_ok=True)
    (repo_root / "crates/demo/Cargo.toml").write_text(
        """\
[package]
name = "demo"
version = "0.1.0"
edition = "2024"
""",
        encoding="utf-8",
    )
    (crate_dir / "lib.rs").write_text(
        """\
pub fn alpha() {}
pub fn beta() {}

#[cfg(test)]
mod tests {
    #[test]
    fn sample() {}
}
""",
        encoding="utf-8",
    )
    return repo_root


if __name__ == "__main__":
    unittest.main()
