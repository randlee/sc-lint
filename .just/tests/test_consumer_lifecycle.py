from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import hashlib
import tarfile
import unittest
import zipfile


ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.5.0"
GUIDES = (
    "README.md",
    "installation",
    "using-sc-lint",
    "configuration",
    "just-setup",
    "ci",
    "upgrade",
    "troubleshooting",
    "best-practices",
    "sc-lint",
    "sc-lint-attributes",
    "sc-lint-boundary",
    "sc-lint-directives",
    "sc-lint-portability",
    "sc-lint-runtime",
    "sc-lint-schema",
)


class ConsumerLifecycleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        binary_name = "sc-lint.exe" if os.name == "nt" else "sc-lint"
        cls.product_binary = ROOT / "target" / "debug" / binary_name
        if not cls.product_binary.is_file():
            subprocess.run(
                ["cargo", "build", "--bin", "sc-lint"], cwd=ROOT, check=True
            )

    def make_release_fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
        temporary = tempfile.TemporaryDirectory(prefix="sc-lint-e7-")
        root = Path(temporary.name)
        release = root / "release"
        consumer = root / "consumer"
        release.mkdir()
        consumer.mkdir()
        shutil.copy2(self.product_binary, release / self.product_binary.name)
        shutil.copytree(ROOT / "docs-bundle", release / "sc-lint-docs")
        return temporary, release, consumer

    @staticmethod
    def fixture_environment(release: Path) -> dict[str, str]:
        environment = os.environ.copy()
        binary_name = "sc-lint.exe" if os.name == "nt" else "sc-lint"
        environment["SC_LINT_BIN"] = str(release / binary_name)
        environment["SC_LINT_INSTALL_DIR"] = str(release)
        environment["PATH"] = str(release) + os.pathsep + environment.get("PATH", "")
        return environment

    @staticmethod
    def run_command(command: list[str], *, cwd: Path, environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            command,
            cwd=cwd,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )

    def initialize_rust_consumer(self, release: Path, consumer: Path) -> dict[str, str]:
        environment = self.fixture_environment(release)
        init = self.run_command(
            [str(release / self.product_binary.name), "init", "--just"],
            cwd=consumer,
            environment=environment,
        )
        self.assertEqual(init.returncode, 0, init.stderr)
        (consumer / "Cargo.toml").write_text(
            "[package]\nname = \"e7_fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            encoding="utf-8",
        )
        source = consumer / "src"
        source.mkdir()
        (source / "lib.rs").write_text(
            "pub fn value() -> u8 {\n    1\n}\n", encoding="utf-8"
        )
        return environment

    def write_release_archive(self, root: Path, binary: Path) -> Path:
        binary_name = self.product_binary.name
        release_dir = root / "published" / f"v{VERSION}"
        release_dir.mkdir(parents=True)
        if os.name == "nt":
            triple, extension = "x86_64-pc-windows-msvc", "zip"
        elif os.uname().sysname == "Darwin":
            triple = "aarch64-apple-darwin" if os.uname().machine == "arm64" else "x86_64-apple-darwin"
            extension = "tar.gz"
        else:
            triple, extension = "x86_64-unknown-linux-gnu", "tar.gz"
        archive = release_dir / f"sc-lint_{VERSION}_{triple}.{extension}"
        if extension == "zip":
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
                package.write(binary, arcname=binary_name)
        else:
            with tarfile.open(archive, "w:gz") as package:
                package.add(binary, arcname=binary_name)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (release_dir / "checksums.txt").write_text(
            f"{digest} {archive.name}\n", encoding="utf-8"
        )
        return root / "published"

    def write_old_probe(self, binary: Path) -> None:
        source = binary.with_suffix(".rs")
        source.write_text(
            'fn main() { print!("{}", r#"{\"ok\":true,\"command\":\"version\",\"data\":{\"contract_schema\":\"sc-lint-version-v1\",\"tool\":\"sc-lint\",\"version\":\"0.3.0\"}}"#); }\n',
            encoding="utf-8",
        )
        result = self.run_command(
            ["rustc", str(source), "-o", str(binary)],
            cwd=binary.parent,
            environment=os.environ.copy(),
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_fresh_release_binary_fixture_runs_full_consumer_lifecycle(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            environment = self.initialize_rust_consumer(release, consumer)
            for recipe in ("setup", "lint", "test"):
                result = self.run_command(["just", recipe], cwd=consumer, environment=environment)
                self.assertEqual(result.returncode, 0, f"just {recipe}: {result.stderr}")
                self.assertNotIn("cargo run", result.stdout + result.stderr)

            upgrade = self.run_command(
                [str(release / self.product_binary.name), "--config", "sc-lint.toml", "upgrade", "--check", "--json"],
                cwd=consumer,
                environment=environment,
            )
            self.assertEqual(upgrade.returncode, 0, upgrade.stderr)
            self.assertIn('"status": "current"', upgrade.stdout)

            for guide in GUIDES:
                resolved = self.run_command(
                    [str(release / self.product_binary.name), "docs", guide, "--path"],
                    cwd=consumer,
                    environment=environment,
                )
                self.assertEqual(resolved.returncode, 0, f"{guide}: {resolved.stderr}")
                self.assertIn("sc-lint-docs", resolved.stdout)

    def test_missing_binary_stops_lint_and_test_before_profiles(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            environment = self.initialize_rust_consumer(release, consumer)
            environment["SC_LINT_BIN"] = str(consumer / "missing-sc-lint")
            marker = consumer / "profile-ran"
            for recipe in ("lint", "test"):
                result = self.run_command(["just", recipe], cwd=consumer, environment=environment)
                combined = result.stdout + result.stderr
                self.assertNotEqual(result.returncode, 0, recipe)
                self.assertIn("CLI.SC_LINT_BINARY_NOT_FOUND", combined)
                self.assertIn("Run `just setup`", combined)
                self.assertNotIn("traceback", combined.lower())
                self.assertFalse(marker.exists(), f"{recipe} profile ran despite failed preflight")

    def test_too_old_binary_produces_structured_release_recovery(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            environment = self.initialize_rust_consumer(release, consumer)
            config_path = consumer / "sc-lint.toml"
            config_path.write_text(
                config_path.read_text(encoding="utf-8").replace(VERSION, "999.0.0"),
                encoding="utf-8",
            )
            environment["SC_LINT_RELEASE_BASE_URL"] = "file:///nonexistent-sc-lint-release"
            result = self.run_command(["just", "lint"], cwd=consumer, environment=environment)
            combined = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("CLI.SC_LINT_VERSION_TOO_OLD", combined)
            self.assertRegex(combined, r"CLI\.SC_LINT_(RELEASE_UNAVAILABLE|POST_INSTALL_VERSION_FAILED)")
            self.assertNotIn("traceback", combined.lower())

    def test_upgrade_migrates_a_real_old_binary_and_preserves_consumer_files(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            environment = self.initialize_rust_consumer(release, consumer)
            readme = consumer / "README.md"
            source = consumer / "src/lib.rs"
            readme.write_text("consumer-owned README\n", encoding="utf-8")
            source_before = source.read_text(encoding="utf-8")
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            old_binary = managed / self.product_binary.name
            self.write_old_probe(old_binary)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            environment["SC_LINT_RELEASE_BASE_URL"] = self.write_release_archive(
                Path(temporary.name), release / self.product_binary.name
            ).as_uri()

            result = self.run_command(["just", "upgrade"], cwd=consumer, environment=environment)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("verified sc-lint release upgraded", result.stdout)
            self.assertEqual(readme.read_text(encoding="utf-8"), "consumer-owned README\n")
            self.assertEqual(source.read_text(encoding="utf-8"), source_before)
            installed = self.run_command(
                [str(managed / self.product_binary.name), "--json", "version"],
                cwd=consumer,
                environment=environment,
            )
            self.assertEqual(installed.returncode, 0, installed.stderr)
            self.assertIn(f'"version": "{VERSION}"', installed.stdout)

    def test_fixture_matrix_declares_each_supported_release_platform(self) -> None:
        mappings = {
            "Ubuntu": ("x86_64-unknown-linux-gnu", ".tar.gz", "sc-lint"),
            "macOS": ("aarch64-apple-darwin", ".tar.gz", "sc-lint"),
            "Windows": ("x86_64-pc-windows-msvc", ".zip", "sc-lint.exe"),
        }
        release_workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        for platform, (target, extension, binary) in mappings.items():
            self.assertIn(target, release_workflow, platform)
            self.assertTrue(extension.startswith("."), platform)
            self.assertTrue(binary.startswith("sc-lint"), platform)


if __name__ == "__main__":
    unittest.main()
