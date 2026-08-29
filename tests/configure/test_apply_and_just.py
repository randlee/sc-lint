from __future__ import annotations

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
REQUEST = ROOT / "tests" / "fixtures" / "configure" / "contracts" / "request.json"
FIXTURES = ROOT / "tests" / "fixtures" / "configure" / "apply-and-just"
BINARY = ROOT / "target" / "debug" / ("sc-lint.exe" if sys.platform == "win32" else "sc-lint")


def configure(root: Path, request: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            str(BINARY),
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
    @classmethod
    def setUpClass(cls) -> None:
        # `sc-lint test` is itself running this executable on Windows, where a
        # rebuild cannot replace an open .exe.  The normal repository gates
        # build it first; retain the fallback for focused, standalone runs.
        if not BINARY.is_file():
            subprocess.run(["cargo", "build", "-q", "-p", "sc-lint"], cwd=ROOT, check=True)

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
            self.assertIn(
                "& .\\\\.sc-lint\\\\bootstrap.ps1",
                (root / ".sc-lint" / "justfile").read_text(encoding="utf-8"),
            )
            parsed = subprocess.run(
                ["just", "--justfile", str(root / "Justfile"), "--list"],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(parsed.returncode, 0, parsed.stderr)
            self.assertIn("lint", parsed.stdout)

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
            initial = (
                (FIXTURES / "existing-just" / "Justfile")
                .read_text(encoding="utf-8")
                .replace("\n", "\r\n")
            )
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
            second_plan = self.plan(root, request)
            reapplied = configure(root, request, "--apply", "--plan", str(second_plan))
            self.assertEqual(reapplied.returncode, 0, reapplied.stderr)
            self.assertEqual((root / "Justfile").read_bytes().decode("utf-8"), result)

    def test_malformed_or_duplicate_marker_is_a_no_write_conflict(self) -> None:
        for fixture in ("malformed-marker", "duplicate-marker", "moved-marker", "modified-marker"):
            with self.subTest(fixture=fixture), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
                justfile = root / "Justfile"
                contents = (FIXTURES / fixture / "Justfile").read_text(encoding="utf-8")
                justfile.write_text(contents, encoding="utf-8")
                request = self.request(root)
                plan = self.plan(root, request)
                applied = configure(root, request, "--apply", "--plan", str(plan))
                self.assertEqual(applied.returncode, 3)
                self.assertEqual(json.loads(applied.stderr)["error"]["code"], "CLI.CONFIGURE_UNMANAGED_COLLISION")
                self.assertEqual(justfile.read_text(encoding="utf-8"), contents)

    def test_every_reserved_recipe_is_a_no_write_collision(self) -> None:
        for recipe in ("setup", "lint", "test", "upgrade"):
            with self.subTest(recipe=recipe), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
                contents = (FIXTURES / f"reserved-{recipe}" / "Justfile").read_text(encoding="utf-8")
                justfile = root / "Justfile"
                justfile.write_text(contents, encoding="utf-8")
                request = self.request(root)
                plan = self.plan(root, request)
                applied = configure(root, request, "--apply", "--plan", str(plan))
                self.assertEqual(applied.returncode, 3)
                self.assertEqual(json.loads(applied.stderr)["error"]["code"], "CLI.CONFIGURE_UNMANAGED_COLLISION")
                self.assertEqual(justfile.read_text(encoding="utf-8"), contents)

    def test_legacy_near_miss_is_never_planned_or_removed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copytree(FIXTURES / "legacy-near-miss", root, dirs_exist_ok=True)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            legacy_action = root / ".github" / "actions" / "setup-sc-lint" / "action.yml"
            original = legacy_action.read_bytes()
            request = self.request(root)
            plan = self.plan(root, request)
            planned = json.loads(plan.read_text(encoding="utf-8"))["data"]
            self.assertFalse(
                any(operation["kind"] == "propose_remove" for operation in planned["operations"])
            )

            applied = configure(root, request, "--apply", "--plan", str(plan))

            self.assertEqual(applied.returncode, 0, applied.stderr)
            self.assertEqual(legacy_action.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
