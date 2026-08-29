from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
REQUEST = ROOT / "tests" / "fixtures" / "configure" / "contracts" / "request.json"


def configure(root: Path, request: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "sc-lint",
            "--",
            "--json",
            "--root",
            str(root),
            "configure",
            "--request",
            str(request),
            *arguments,
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )


class ApplyAndJustTests(unittest.TestCase):
    def request(self, root: Path) -> Path:
        request = json.loads(REQUEST.read_text(encoding="utf-8"))
        request["request"]["just"]["mode"] = "generate_managed_import"
        path = root / "request.json"
        path.write_text(json.dumps(request), encoding="utf-8")
        return path

    def plan(self, root: Path, request: Path) -> Path:
        planned = configure(root, request)
        self.assertEqual(planned.returncode, 0, planned.stderr)
        plan = root / "plan.json"
        plan.write_text(planned.stdout, encoding="utf-8")
        return plan

    def test_empty_repository_applies_only_reviewed_generated_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            request = self.request(root)
            plan = self.plan(root, request)

            applied = configure(root, request, "--apply", "--plan", str(plan))

            self.assertEqual(applied.returncode, 0, applied.stderr)
            data = json.loads(applied.stdout)["data"]
            self.assertEqual(data["status"], "applied")
            self.assertIn("minimum_version = \"0.5.0\"", (root / "sc-lint.toml").read_text(encoding="utf-8"))
            self.assertTrue((root / ".sc-lint" / "justfile").is_file())
            self.assertTrue((root / ".sc-lint" / "bootstrap").is_file())
            self.assertTrue((root / ".sc-lint" / "bootstrap.ps1").is_file())
            self.assertIn("lint:", (root / "Justfile").read_text(encoding="utf-8"))

    def test_changed_target_rejects_stale_plan_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            (root / "Justfile").write_text("user:\n    @echo user\n", encoding="utf-8")
            request = self.request(root)
            plan = self.plan(root, request)
            (root / "Justfile").write_text("user:\n    @echo changed\n", encoding="utf-8")

            applied = configure(root, request, "--apply", "--plan", str(plan))

            self.assertEqual(applied.returncode, 3)
            failure = json.loads(applied.stderr)
            self.assertEqual(failure["command"], "configure.apply")
            self.assertEqual(failure["error"]["code"], "CLI.CONFIGURE_STALE_PLAN")
            self.assertNotIn("sc-lint.toml", {path.name for path in root.iterdir()})
            self.assertEqual((root / "Justfile").read_text(encoding="utf-8"), "user:\n    @echo changed\n")

    def test_existing_justfile_preserves_crlf_and_adds_one_marker_block(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            initial = "# user comment\r\nuser:\r\n    @echo user\r\n"
            (root / "Justfile").write_bytes(initial.encode("utf-8"))
            request = self.request(root)
            plan = self.plan(root, request)

            applied = configure(root, request, "--apply", "--plan", str(plan))

            self.assertEqual(applied.returncode, 0, applied.stderr)
            result = (root / "Justfile").read_bytes().decode("utf-8")
            self.assertTrue(result.startswith(initial))
            self.assertEqual(result.count("# >>> sc-lint managed integration >>>"), 1)
            self.assertIn("import '.sc-lint/justfile'\r\n", result)
            self.assertTrue((root / ".sc-lint" / "justfile").is_file())


if __name__ == "__main__":
    unittest.main()
