from __future__ import annotations

from sc_lint.lint_common import discover_repo_root

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import hashlib
import http.server
import json
import tarfile
import threading
import tomllib
import unittest
from unittest import mock
import zipfile
from contextlib import contextmanager
from functools import partial


ROOT = discover_repo_root()
VERSION = "0.7.3"
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
        # The bootstrap provisions `.sc-lint/venv` from PyPI; point it at the
        # wheels `just setup` built so fixtures stay offline.
        wheel_dir = ROOT / ".sc-lint" / "wheels"
        if not os.environ.get("SC_LINT_WHEEL_DIR") and not wheel_dir.is_dir():
            raise unittest.SkipTest("run `just setup` to build .sc-lint/wheels")
        cls._wheel_environment = mock.patch.dict(
            os.environ, {"SC_LINT_WHEEL_DIR": os.environ.get("SC_LINT_WHEEL_DIR", str(wheel_dir))}
        )
        cls._wheel_environment.start()
        binary_name = "sc-lint.exe" if os.name == "nt" else "sc-lint"
        cls.product_binary = ROOT / "target" / "debug" / binary_name
        cls.release_binaries = [
            cls.product_binary,
            *(
                ROOT / "target" / "debug" / f"{name}{'.exe' if os.name == 'nt' else ''}"
                for name in (
                    "sc-lint-boundary",
                    "sc-lint-portability",
                    "sc-lint-runtime",
                )
            ),
        ]
        if any(not binary.is_file() for binary in cls.release_binaries):
            subprocess.run(
                [
                    "cargo",
                    "build",
                    "-p",
                    "sc-lint",
                    "-p",
                    "sc-lint-boundary",
                    "-p",
                    "sc-lint-portability",
                    "-p",
                    "sc-lint-runtime",
                ],
                cwd=ROOT,
                check=True,
            )

    @classmethod
    def tearDownClass(cls) -> None:
        cls._wheel_environment.stop()

    def make_release_fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
        temporary = tempfile.TemporaryDirectory(prefix="sc-lint-e7-")
        root = Path(temporary.name)
        release = root / "release"
        consumer = root / "consumer"
        release.mkdir()
        consumer.mkdir()
        shutil.copy2(self.product_binary, release / self.product_binary.name)
        for binary in self.release_binaries[1:]:
            shutil.copy2(binary, release / binary.name)
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

    @staticmethod
    @contextmanager
    def release_server(root: Path):
        class QuietHandler(http.server.SimpleHTTPRequestHandler):
            def log_message(self, format: str, *args: object) -> None:
                return

        server = http.server.ThreadingHTTPServer(
            ("127.0.0.1", 0), partial(QuietHandler, directory=str(root))
        )
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            thread.join()
            server.server_close()

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

    def write_release_archive(
        self,
        root: Path,
        binary: Path,
        *,
        layout: str = "nested",
        include_binary: bool = True,
        flat_marker: bytes = b"",
        nested_failure: bool = False,
    ) -> Path:
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
        archive_root = f"sc-lint_{VERSION}_{triple}"
        flat_binary = binary
        if flat_marker:
            flat_binary = root / f"{binary_name}.flat"
            shutil.copy2(binary, flat_binary)
            with flat_binary.open("ab") as fixture_binary:
                fixture_binary.write(flat_marker)
        nested_binary = binary
        if nested_failure:
            nested_binary = root / f"{binary_name}.nested"
            if os.name == "nt":
                nested_binary.write_bytes(b"nested bootstrap launcher selected")
            else:
                nested_binary.write_bytes(
                    b"#!/bin/sh\nprintf '%s\\n' nested-bootstrap-launcher-selected >&2\nexit 1\n"
                )
                nested_binary.chmod(0o755)
        backend_binaries = self.release_binaries[1:]
        archive_entries = []
        if layout in ("nested", "both"):
            archive_entries.append((nested_binary, f"{archive_root}/bin/{binary_name}"))
            archive_entries.extend(
                (backend, f"{archive_root}/bin/{backend.name}")
                for backend in backend_binaries
            )
        if layout in ("flat", "both"):
            archive_entries.append((flat_binary, binary_name))
            archive_entries.extend((backend, backend.name) for backend in backend_binaries)
        if extension == "zip":
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
                if include_binary:
                    for archive_binary, archive_entry in archive_entries:
                        package.write(archive_binary, arcname=archive_entry)
                    docs_prefix = (
                        f"{archive_root}/share/sc-lint/sc-lint-docs"
                        if layout == "nested"
                        else "docs-bundle"
                    )
                    for guide in (ROOT / "docs-bundle").rglob("*"):
                        if guide.is_file():
                            package.write(
                                guide,
                                arcname=f"{docs_prefix}/{guide.relative_to(ROOT / 'docs-bundle')}",
                            )
                else:
                    package.writestr(f"{archive_root}/missing-binary", "fixture")
        else:
            with tarfile.open(archive, "w:gz") as package:
                if include_binary:
                    for archive_binary, archive_entry in archive_entries:
                        package.add(archive_binary, arcname=archive_entry)
                    docs_prefix = (
                        f"{archive_root}/share/sc-lint/sc-lint-docs"
                        if layout == "nested"
                        else "docs-bundle"
                    )
                    package.add(ROOT / "docs-bundle", arcname=docs_prefix)
                else:
                    package.addfile(tarfile.TarInfo(f"{archive_root}/missing-binary"))
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (release_dir / "checksums.txt").write_text(
            f"{digest} {archive.name}\n", encoding="utf-8"
        )
        return root / "published"

    def write_old_probe(self, binary: Path) -> None:
        source = binary.with_suffix(".rs")
        source.write_text(
            'fn main() {\n'
            '    if std::env::args().any(|argument| argument == "compatibility") {\n'
            '        std::process::exit(1);\n'
            '    }\n'
            '    print!("{}", r#"{\"ok\":true,\"command\":\"version\",\"data\":{\"contract_schema\":\"sc-lint-version-v1\",\"tool\":\"sc-lint\",\"version\":\"0.3.0\"}}"#);\n'
            '}\n',
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

    def test_cold_start_bootstrap_installs_a_verified_release_then_lints(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            config = consumer / "sc-lint.toml"
            config.write_text(
                "[tool.sc-lint]\n"
                f'minimum_version = "{VERSION}"\n\n'
                "[[tool.sc-lint.lint]]\n"
                'name = "release-version-probe"\n'
                f"command = {json.dumps([str(self.product_binary), '--json', 'version'])}\n\n"
                "[[tool.sc-lint.test]]\n"
                'name = "release-version-probe"\n'
                f"command = {json.dumps([str(self.product_binary), '--json', 'version'])}\n",
                encoding="utf-8",
            )
            published = self.write_release_archive(Path(temporary.name), self.product_binary)
            just = shutil.which("just")
            self.assertIsNotNone(just, "just is required for the consumer fixture")
            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment.pop("SC_LINT_RELEASE_BASE_URL", None)
            environment["SC_LINT_INSTALL_DIR"] = str(Path(temporary.name) / "clean-install")
            # Keep only OS command directories: this fixture intentionally leaves
            # every directory containing a preinstalled sc-lint off PATH.
            command_dirs = [Path("/usr/bin"), Path("/bin")]
            if os.name == "nt":
                for command in ("pwsh", "curl"):
                    resolved = shutil.which(command)
                    if resolved:
                        command_dirs.append(Path(resolved).parent)
                # Windows has no /usr/bin: the bootstrap needs a host python
                # on PATH to create the consumer venv.
                command_dirs.append(Path(sys.executable).parent)
            environment["PATH"] = os.pathsep.join(
                str(path) for path in command_dirs if path.is_dir()
            )
            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                setup = self.run_command([just, "setup"], cwd=consumer, environment=environment)
                self.assertEqual(setup.returncode, 0, setup.stderr)
                lint = self.run_command([just, "lint"], cwd=consumer, environment=environment)
                self.assertEqual(lint.returncode, 0, lint.stderr)
            self.assertTrue(
                (Path(environment["SC_LINT_INSTALL_DIR"]) / self.product_binary.name).is_file()
            )

    @unittest.skipIf(os.name == "nt", "POSIX PATH bootstrap")
    def test_incompatible_path_binary_is_replaced_by_a_verified_managed_release(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            old_bin = Path(temporary.name) / "old-bin"
            old_bin.mkdir()
            old_binary = old_bin / "sc-lint"
            calls = Path(temporary.name) / "old-binary-calls.log"
            old_binary.write_text(
                "#!/bin/sh\n"
                f"printf '%s\\n' \"$*\" >> '{calls}'\n"
                'for argument in "$@"; do [ "$argument" = compatibility ] && exit 1; done\n'
                "exit 64\n",
                encoding="utf-8",
            )
            old_binary.chmod(0o755)
            self.write_release_archive(Path(temporary.name), release / self.product_binary.name)
            just = shutil.which("just")
            self.assertIsNotNone(just, "just is required for the consumer fixture")
            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            command_dirs = [old_bin, Path(just).parent, Path("/usr/bin"), Path("/bin")]
            environment["PATH"] = os.pathsep.join(
                str(path) for path in command_dirs if path.is_dir()
            )

            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                result = self.run_command([just, "setup"], cwd=consumer, environment=environment)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertTrue((managed / self.product_binary.name).is_file())
            self.assertEqual(
                calls.read_text(encoding="utf-8").splitlines(),
                [f"--config sc-lint.toml compatibility check --binary {old_binary}"],
            )
            installed = self.run_command(
                [str(managed / self.product_binary.name), "--json", "version"],
                cwd=consumer,
                environment=environment,
            )
            self.assertEqual(installed.returncode, 0, installed.stderr)
            self.assertIn(f'"version": "{VERSION}"', installed.stdout)

    @unittest.skipIf(os.name == "nt", "POSIX PATH bootstrap")
    def test_compatible_path_binary_is_reused_without_managed_install(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            path_bin = Path(temporary.name) / "path-bin"
            path_bin.mkdir()
            shutil.copy2(self.product_binary, path_bin / "sc-lint")
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            environment["SC_LINT_RELEASE_BASE_URL"] = "http://127.0.0.1:9/unreachable"
            environment["PATH"] = os.pathsep.join(
                str(path)
                for path in (path_bin, Path("/usr/bin"), Path("/bin"))
                if path.is_dir()
            )

            result = self.run_command(
                ["sh", str(consumer / ".sc-lint/bootstrap"), "setup", "--config", "sc-lint.toml", "--dry-run"],
                cwd=consumer,
                environment=environment,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertNotIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", result.stdout + result.stderr)
            self.assertFalse((managed / self.product_binary.name).exists())

    @unittest.skipIf(os.name == "nt", "POSIX PATH bootstrap")
    def test_incompatible_path_binary_dry_run_does_not_fetch_or_override_explicit_bin(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            old_bin = Path(temporary.name) / "old-bin"
            old_bin.mkdir()
            old_binary = old_bin / "sc-lint"
            calls = Path(temporary.name) / "old-binary-calls.log"
            old_binary.write_text(
                "#!/bin/sh\n"
                f"printf '%s\\n' \"$*\" >> '{calls}'\n"
                'for argument in "$@"; do [ "$argument" = compatibility ] && exit 1; done\n'
                "exit 0\n",
                encoding="utf-8",
            )
            old_binary.chmod(0o755)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            environment = os.environ.copy()
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            environment["SC_LINT_RELEASE_BASE_URL"] = "http://127.0.0.1:9/unreachable"
            environment.pop("SC_LINT_BIN", None)
            environment["PATH"] = os.pathsep.join(
                str(path)
                for path in (old_bin, Path("/usr/bin"), Path("/bin"))
                if path.is_dir()
            )
            bootstrap = str(consumer / ".sc-lint/bootstrap")
            config = str(consumer / "sc-lint.toml")
            path_result = self.run_command(
                ["sh", bootstrap, "setup", "--config", config, "--dry-run"],
                cwd=consumer,
                environment=environment,
            )

            self.assertEqual(path_result.returncode, 0, path_result.stdout + path_result.stderr)
            self.assertNotIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", path_result.stdout + path_result.stderr)
            self.assertFalse((managed / self.product_binary.name).exists())
            self.assertEqual(
                calls.read_text(encoding="utf-8").splitlines(),
                [
                    f"--config {config} compatibility check --binary {old_binary}",
                    f"--config {config} setup --dry-run",
                    f"--config {config} setup --dry-run",
                ],
            )

            calls.unlink()
            environment["SC_LINT_BIN"] = str(old_binary)
            override_result = self.run_command(
                ["sh", str(consumer / ".sc-lint/bootstrap"), "setup", "--config", str(consumer / "sc-lint.toml"), "--dry-run"],
                cwd=consumer,
                environment=environment,
            )

            self.assertEqual(override_result.returncode, 0, override_result.stdout + override_result.stderr)
            self.assertNotIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", override_result.stdout + override_result.stderr)
            self.assertFalse((managed / self.product_binary.name).exists())
            self.assertEqual(
                calls.read_text(encoding="utf-8").splitlines(),
                [
                    f"--config {config} compatibility check --binary {old_binary}",
                    f"--config {config} setup --dry-run",
                    f"--config {config} setup --dry-run",
                ],
            )

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

    def assert_posix_managed_old_binary_self_upgrades_through_just_setup(
        self,
        *,
        layout: str = "nested",
        flat_marker: bytes = b"",
        nested_failure: bool = False,
    ) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            managed_binary = managed / self.product_binary.name
            self.write_old_probe(managed_binary)
            self.write_release_archive(
                Path(temporary.name),
                release / self.product_binary.name,
                layout=layout,
                flat_marker=flat_marker,
                nested_failure=nested_failure,
            )
            just = shutil.which("just")
            self.assertIsNotNone(just, "just is required for this fixture")

            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            command_dirs = [Path("/usr/bin"), Path("/bin"), Path(just).parent]
            environment["PATH"] = os.pathsep.join(
                str(path) for path in command_dirs if path.is_dir()
            )

            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                result = self.run_command([just, "setup"], cwd=consumer, environment=environment)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("verified sc-lint release upgraded", result.stdout)
            installed = self.run_command(
                [str(managed_binary), "--json", "version"],
                cwd=consumer,
                environment=environment,
            )
            self.assertEqual(installed.returncode, 0, installed.stderr)
            self.assertIn(f'"version": "{VERSION}"', installed.stdout)
            if flat_marker:
                self.assertTrue(managed_binary.read_bytes().endswith(flat_marker))

    @unittest.skipIf(os.name == "nt", "POSIX managed-binary replacement contract")
    def test_posix_managed_old_binary_self_upgrades_through_just_setup(self) -> None:
        self.assert_posix_managed_old_binary_self_upgrades_through_just_setup()

    @unittest.skipIf(os.name == "nt", "POSIX legacy flat archive compatibility")
    def test_posix_managed_old_binary_accepts_legacy_flat_archive(self) -> None:
        self.assert_posix_managed_old_binary_self_upgrades_through_just_setup(layout="flat")

    @unittest.skipIf(os.name == "nt", "POSIX flat archive selection")
    def test_posix_managed_old_binary_prefers_flat_when_both_layouts_exist(self) -> None:
        self.assert_posix_managed_old_binary_self_upgrades_through_just_setup(
            layout="both",
            flat_marker=b"sc-lint-flat-layout-precedence",
            nested_failure=True,
        )

    def assert_windows_managed_old_binary_self_upgrades_through_just_setup(
        self,
        *,
        layout: str = "nested",
        flat_marker: bytes = b"",
        nested_failure: bool = False,
    ) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            managed_binary = managed / self.product_binary.name
            self.write_old_probe(managed_binary)
            self.write_release_archive(
                Path(temporary.name),
                release / self.product_binary.name,
                layout=layout,
                flat_marker=flat_marker,
                nested_failure=nested_failure,
            )

            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            command_dirs = []
            for command in ("just", "pwsh", "curl"):
                resolved = shutil.which(command)
                self.assertIsNotNone(resolved, f"{command} is required for this fixture")
                command_dirs.append(str(Path(resolved).parent))
            # The bootstrap needs a host python on PATH to create the consumer venv.
            command_dirs.append(str(Path(sys.executable).parent))
            environment["PATH"] = os.pathsep.join(command_dirs)

            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                result = self.run_command(["just", "setup"], cwd=consumer, environment=environment)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("verified sc-lint", result.stdout)
            installed = self.run_command(
                [str(managed_binary), "--json", "version"],
                cwd=consumer,
                environment=environment,
            )
            self.assertEqual(installed.returncode, 0, installed.stderr)
            self.assertIn(f'"version": "{VERSION}"', installed.stdout)
            if flat_marker:
                self.assertTrue(managed_binary.read_bytes().endswith(flat_marker))

    @unittest.skipUnless(os.name == "nt", "Windows managed-binary replacement contract")
    def test_windows_managed_old_binary_self_upgrades_through_just_setup(self) -> None:
        self.assert_windows_managed_old_binary_self_upgrades_through_just_setup()

    @unittest.skipUnless(os.name == "nt", "Windows legacy archive compatibility")
    def test_windows_managed_old_binary_accepts_legacy_flat_archive(self) -> None:
        self.assert_windows_managed_old_binary_self_upgrades_through_just_setup(layout="flat")

    @unittest.skipUnless(os.name == "nt", "Windows flat archive selection")
    def test_windows_managed_old_binary_prefers_flat_when_both_layouts_exist(self) -> None:
        self.assert_windows_managed_old_binary_self_upgrades_through_just_setup(
            layout="both",
            flat_marker=b"sc-lint-flat-layout-precedence",
            nested_failure=True,
        )

    @unittest.skipUnless(os.name == "nt", "Windows malformed archive rejection")
    def test_windows_bootstrap_rejects_archive_without_executable(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            self.write_release_archive(
                Path(temporary.name),
                release / self.product_binary.name,
                include_binary=False,
            )

            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            command_dirs = []
            for command in ("pwsh", "curl"):
                resolved = shutil.which(command)
                self.assertIsNotNone(resolved, f"{command} is required for this fixture")
                command_dirs.append(str(Path(resolved).parent))
            command_dirs.append(str(Path(sys.executable).parent))
            environment["PATH"] = os.pathsep.join(command_dirs)

            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                result = self.run_command(
                    [
                        "pwsh",
                        "-NoLogo",
                        "-NonInteractive",
                        "-File",
                        str(consumer / ".sc-lint/bootstrap.ps1"),
                        "setup",
                        "--config",
                        "sc-lint.toml",
                    ],
                    cwd=consumer,
                    environment=environment,
                )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", result.stdout + result.stderr)
            self.assertIn("did not contain sc-lint.exe", result.stdout + result.stderr)

    @unittest.skipUnless(os.name == "nt", "Windows PATH bootstrap")
    def test_windows_incompatible_path_binary_is_replaced_by_a_verified_managed_release(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            old_bin = Path(temporary.name) / "old-bin"
            old_bin.mkdir()
            old_binary = old_bin / "sc-lint.exe"
            self.write_old_probe(old_binary)
            self.write_release_archive(Path(temporary.name), release / self.product_binary.name)
            just = shutil.which("just")
            self.assertIsNotNone(just, "just is required for the consumer fixture")
            command_dirs = []
            for command in ("pwsh", "curl"):
                resolved = shutil.which(command)
                self.assertIsNotNone(resolved, f"{command} is required for this fixture")
                command_dirs.append(str(Path(resolved).parent))
            command_dirs.append(str(Path(sys.executable).parent))
            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            environment["PATH"] = os.pathsep.join(
                [str(old_bin), str(Path(just).parent), *command_dirs]
            )

            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                result = self.run_command([just, "setup"], cwd=consumer, environment=environment)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertTrue((managed / self.product_binary.name).is_file())
            installed = self.run_command(
                [str(managed / self.product_binary.name), "--json", "version"],
                cwd=consumer,
                environment=environment,
            )
            self.assertEqual(installed.returncode, 0, installed.stderr)
            self.assertIn(f'"version": "{VERSION}"', installed.stdout)

    @unittest.skipUnless(os.name == "nt", "Windows managed-binary dry-run contract")
    def test_windows_stale_managed_upgrade_dry_run_does_not_replace_or_download(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            managed = Path(temporary.name) / "managed"
            managed.mkdir()
            managed_binary = managed / self.product_binary.name
            self.write_old_probe(managed_binary)
            before = hashlib.sha256(managed_binary.read_bytes()).hexdigest()

            environment = os.environ.copy()
            environment.pop("SC_LINT_BIN", None)
            environment["SC_LINT_INSTALL_DIR"] = str(managed)
            command_dirs = []
            for command in ("just", "pwsh"):
                resolved = shutil.which(command)
                self.assertIsNotNone(resolved, f"{command} is required for this fixture")
                command_dirs.append(str(Path(resolved).parent))
            # The bootstrap needs a host python on PATH to create the consumer venv.
            command_dirs.append(str(Path(sys.executable).parent))
            environment["PATH"] = os.pathsep.join(command_dirs)
            environment["SC_LINT_RELEASE_BASE_URL"] = "http://127.0.0.1:9/unreachable"

            result = self.run_command(
                [
                    "pwsh",
                    "-NoLogo",
                    "-NonInteractive",
                    "-File",
                    str(consumer / ".sc-lint/bootstrap.ps1"),
                    "upgrade",
                    "--config",
                    "sc-lint.toml",
                    "--dry-run",
                ],
                cwd=consumer,
                environment=environment,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertNotIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", result.stdout + result.stderr)
            self.assertEqual(before, hashlib.sha256(managed_binary.read_bytes()).hexdigest())

    def test_fixture_matrix_declares_each_supported_release_platform(self) -> None:
        mappings = {
            "Ubuntu": ("x86_64-unknown-linux-gnu", ".tar.gz", "sc-lint"),
            "macOS": ("aarch64-apple-darwin", ".tar.gz", "sc-lint"),
            "Windows": ("x86_64-pc-windows-msvc", ".zip", "sc-lint.exe"),
        }
        manifest = tomllib.loads(
            (ROOT / "release/publish-artifacts.toml").read_text(encoding="utf-8")
        )
        release_targets = {entry["target"] for entry in manifest["release_targets"]}
        for platform, (target, extension, binary) in mappings.items():
            self.assertIn(target, release_targets, platform)
            self.assertTrue(extension.startswith("."), platform)
            self.assertTrue(binary.startswith("sc-lint"), platform)


if __name__ == "__main__":
    unittest.main()
