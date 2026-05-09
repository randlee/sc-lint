from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from run_lint import build_tasks
from run_lint import build_transcript
from run_lint import LintResult
from run_lint import LintTask
from run_lint import preview_lines_for_task
from run_lint import prioritize_error_lines
from run_lint import resolve_task_names
from run_lint import strip_ansi


class RunLintTests(unittest.TestCase):
    ROOT_MANIFEST = """\
[workspace]
members = ["crates/sc-lint-boundary"]
resolver = "2"
"""

    def test_resolve_task_names_all_includes_expected_targets(self) -> None:
        names = resolve_task_names("all")
        self.assertIn("manifests", names)
        self.assertIn("deny", names)
        self.assertIn("shear", names)
        self.assertIn("spell", names)
        self.assertIn("pytests", names)
        self.assertIn("sc-boundary", names)
        self.assertIn("sc-portability", names)
        self.assertNotIn("modules", names)

    def test_resolve_task_names_rejects_unknown_target(self) -> None:
        with self.assertRaises(ValueError):
            resolve_task_names("unknown")

    def test_resolve_task_names_accepts_manual_targets(self) -> None:
        self.assertEqual(resolve_task_names("modules"), ["modules"])
        self.assertEqual(resolve_task_names("sc-boundary"), ["sc-boundary"])
        self.assertEqual(resolve_task_names("sc-portability"), ["sc-portability"])

    def test_prioritize_error_lines_prefers_actual_failures(self) -> None:
        lines = [
            "Updating crates.io index",
            "Downloaded crate",
            "error[E0432]: unresolved import `uuid`",
            "could not compile `sc-lint-boundary`",
        ]

        self.assertEqual(
            prioritize_error_lines(lines),
            [
                "error[E0432]: unresolved import `uuid`",
                "could not compile `sc-lint-boundary`",
            ],
        )

    def test_strip_ansi_and_prioritize_error_lines_handles_colored_cargo_output(self) -> None:
        lines = [
            strip_ansi("\x1b[1m\x1b[92m  Downloaded\x1b[0m thiserror v2.0.18"),
            strip_ansi("\x1b[1m\x1b[91merror[E0308]: mismatched types\x1b[0m"),
            strip_ansi("\x1b[1m\x1b[91merror:\x1b[0m could not compile `sc-lint-boundary`"),
        ]

        self.assertEqual(
            prioritize_error_lines(lines),
            [
                "error[E0308]: mismatched types",
                "error: could not compile `sc-lint-boundary`",
            ],
        )

    def test_build_tasks_contains_expected_commands(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            tasks = build_tasks(repo_root)
            self.assertEqual(tasks["modules"].command[-1], str(repo_root / ".just/lint_cargo_modules.py"))
            self.assertEqual(tasks["sc-boundary"].command[-1], str(repo_root / ".just/lint_sc_boundary.py"))
            self.assertEqual(tasks["sc-portability"].command[-1], str(repo_root / ".just/lint_sc_portability.py"))
            self.assertEqual(tasks["manifests"].command[-1], str(repo_root / ".just/lint_manifests.py"))
            self.assertEqual(tasks["deny"].command[-1], str(repo_root / ".just/lint_cargo_deny.py"))
            self.assertEqual(tasks["shear"].command[-1], str(repo_root / ".just/lint_cargo_shear.py"))
            self.assertEqual(tasks["spell"].command[-1], str(repo_root / ".just/lint_codespell.py"))
            self.assertEqual(tasks["pytests"].command[-1], str(repo_root / ".just/run_pytests.py"))

    def test_resolve_task_names_fast_is_low_latency_subset(self) -> None:
        self.assertEqual(
            resolve_task_names("fast"),
            ["fmt", "version", "manifests", "spell", "pytests"],
        )

    def test_build_transcript_adds_crate_inventory_for_crate_scoped_lints(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            (repo_root / "Cargo.toml").write_text(self.ROOT_MANIFEST, encoding="utf-8")
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
            for lint_name in ("fmt", "clippy", "modules", "sc-boundary", "manifests"):
                result = LintResult(
                    task=LintTask(lint_name, ["just", f"_lint-{lint_name}"]),
                    returncode=0,
                    stdout="",
                    stderr="",
                    duration_seconds=0.2,
                    log_path=repo_root / ".just/logs/example.log",
                )

                transcript = build_transcript(result.task, result, repo_root)

                self.assertIn("crates analyzed:", transcript)
                joined = "\n".join(transcript)
                self.assertIn("crate_path", joined)
                self.assertIn("sc-lint-boundary", joined)

    def test_preview_lines_for_sc_boundary_skips_wrapper_banner(self) -> None:
        self.assertEqual(
            preview_lines_for_task(
                "sc-boundary",
                [
                    "sc-boundary failed",
                    "architectural cycle across owners: A, B",
                    "full log: .just/logs/example.log",
                ],
            ),
            ["architectural cycle across owners: A, B"],
        )


if __name__ == "__main__":
    unittest.main()
