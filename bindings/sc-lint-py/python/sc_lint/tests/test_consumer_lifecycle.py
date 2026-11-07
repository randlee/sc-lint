from __future__ import annotations

from sc_lint.lint_common import discover_repo_root
from sc_lint.tests.test_setup_tools import FakeToolchain
from sc_lint.tests.test_setup_tools import WINDOWS_TARGET
from sc_lint.tests.test_setup_tools import XWIN_TOOLS
from sc_lint.tests.test_setup_tools import install_input
from sc_lint.tests.test_setup_tools import HelperFixture, write_executable, WINDOWS_CHECK
import sc_lint
from sc_lint import setup_tools
from typing import Optional, List

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
from sc_lint import _toml
import unittest
import venv
from unittest import mock
import zipfile
from contextlib import contextmanager
from functools import partial


ROOT = discover_repo_root()
BOOTSTRAP = ROOT / "crates/sc-lint/assets/bootstrap.py"
ADOPTION_INSTALLER = ROOT / "packages/sc-lint-adoption/install.py"
PACKAGE_ROOT = Path(sc_lint.__file__).resolve().parent.parent
VERSION = "0.8.6"
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


class WheelConsumerTests(unittest.TestCase):
    def test_wheel_pytests_pass_in_a_consumer_without_repository_assets(self) -> None:
        with tempfile.TemporaryDirectory(prefix="sc-lint-wheel-consumer-") as directory:
            root = Path(directory)
            consumer = root / "consumer"
            consumer.mkdir()
            environment = root / "venv"
            venv.EnvBuilder(with_pip=True).create(environment)
            python = environment / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
            wheels = os.environ.get("SC_LINT_WHEEL_DIR", str(ROOT / ".sc-lint/wheels"))
            project = _toml.loads((ROOT / "bindings/sc-lint-py/pyproject.toml").read_text())
            version = project["project"]["version"]
            excluded = project["tool"]["maturin"]["exclude"]
            modules = {Path(pattern).stem for pattern in excluded if Path(pattern).stem.startswith("test_")}
            built_wheels = list(Path(wheels).glob(f"sc_lint-{version}-*.whl"))
            self.assertEqual(len(built_wheels), 1, f"expected exactly one current source wheel: {built_wheels}")
            wheel = built_wheels[0]
            with zipfile.ZipFile(wheel) as archive:
                self.assertFalse([name for name in archive.namelist() if Path(name).name.split(".")[0] in modules])
            subprocess.run(
                [str(python), "-m", "pip", "install", "--no-index", "--find-links", wheels, str(wheel)],
                check=True, capture_output=True, text=True, timeout=120,
            )
            self.assertFalse((consumer / "scripts").exists())
            self.assertFalse((consumer / "release").exists())
            # Keep SC_LINT_BIN: the wheel's pytests must run against the binary
            # built from this tree, not a stale managed or PATH install.
            child_env = {key: value for key, value in os.environ.items()
                         if key not in {"PYTHONPATH", "PYTHONHOME", "SC_LINT_SOURCE_ROOT", "SC_LINT_INSTALL_DIR"}}
            # The Justfile exports a relative path; the child runs in `consumer`.
            if child_env.get("SC_LINT_BIN"):
                child_env["SC_LINT_BIN"] = os.path.abspath(child_env["SC_LINT_BIN"])
            result = subprocess.run(
                [str(python), "-I", "-m", "sc_lint.run_pytests"], env=child_env,
                cwd=consumer, capture_output=True, text=True, timeout=180,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("pytests fixtures total:", result.stdout)
            self.assertIn("RunPytestsTests", result.stdout)
            for module in modules:
                self.assertNotIn(module + ".", result.stdout)


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
        binary = os.environ.get("SC_LINT_BIN")
        if binary:
            cls.product_binary = Path(binary)
            if not cls.product_binary.is_absolute():
                cls.product_binary = ROOT / cls.product_binary
        else:
            # Direct pytest runs lack just's SC_LINT_BIN; honor cargo's target override.
            target_dir = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
            if not target_dir.is_absolute():
                target_dir = ROOT / target_dir
            cls.product_binary = target_dir / "debug" / binary_name
        binary_dir = cls.product_binary.parent
        cls.release_binaries = [
            cls.product_binary,
            *(
                binary_dir / f"{name}{'.exe' if os.name == 'nt' else ''}"
                for name in (
                    "sc-lint-boundary",
                    "sc-lint-portability",
                    "sc-lint-runtime",
                )
            ),
        ]
        if any(not binary.is_file() for binary in cls.release_binaries):
            command = [
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
            ]
            try:
                subprocess.run(
                    command,
                    cwd=ROOT,
                    check=True,
                    stdin=subprocess.DEVNULL,
                    timeout=300,
                )
            except subprocess.TimeoutExpired as error:
                raise AssertionError(
                    f"{command!r} timed out after {error.timeout}s; "
                    f"stdout={error.stdout!r}; stderr={error.stderr!r}"
                ) from error

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

    def run_command(
        self,
        command: list[str],
        *,
        cwd: Path,
        environment: dict[str, str],
    ) -> subprocess.CompletedProcess[str]:
        try:
            return subprocess.run(
                command,
                cwd=cwd,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
                stdin=subprocess.DEVNULL,
                timeout=180,
            )
        except subprocess.TimeoutExpired as error:
            self.fail(
                f"{command!r} timed out after {error.timeout}s; "
                f"stdout={error.stdout!r}; stderr={error.stderr!r}"
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
        server.daemon_threads = True
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            thread.join(timeout=10)
            try:
                assert not thread.is_alive(), "release server thread did not stop after shutdown"
            finally:
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
            command_dirs = [Path(sys.executable).parent, Path("/usr/bin"), Path("/bin")]
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
            command_dirs = [old_bin, Path(just).parent, Path(sys.executable).parent, Path("/usr/bin"), Path("/bin")]
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
                [sys.executable, str(consumer / ".sc-lint/bootstrap.py"), "setup", "--config", "sc-lint.toml", "--dry-run"],
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
            bootstrap = str(consumer / ".sc-lint/bootstrap.py")
            config = str(consumer / "sc-lint.toml")
            path_result = self.run_command(
                [sys.executable, bootstrap, "setup", "--config", config, "--dry-run"],
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
                [sys.executable, str(consumer / ".sc-lint/bootstrap.py"), "setup", "--config", str(consumer / "sc-lint.toml"), "--dry-run"],
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
            command_dirs = [Path(sys.executable).parent, Path("/usr/bin"), Path("/bin"), Path(just).parent]
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
            for command in ("just",):
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
            command_dirs.append(str(Path(sys.executable).parent))
            environment["PATH"] = os.pathsep.join(command_dirs)

            with self.release_server(Path(temporary.name)) as release_base:
                environment["SC_LINT_RELEASE_BASE_URL"] = f"{release_base}/published"
                result = self.run_command(
                    [
                        sys.executable,
                        str(consumer / ".sc-lint/bootstrap.py"),
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
            for command in ("just",):
                resolved = shutil.which(command)
                self.assertIsNotNone(resolved, f"{command} is required for this fixture")
                command_dirs.append(str(Path(resolved).parent))
            # The bootstrap needs a host python on PATH to create the consumer venv.
            command_dirs.append(str(Path(sys.executable).parent))
            environment["PATH"] = os.pathsep.join(command_dirs)
            environment["SC_LINT_RELEASE_BASE_URL"] = "http://127.0.0.1:9/unreachable"

            result = self.run_command(
                [
                    sys.executable,
                    str(consumer / ".sc-lint/bootstrap.py"),
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

    def bootstrap_setup(self, consumer: Path, environment: dict[str, str], *flags: str) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(consumer / ".sc-lint/bootstrap.py")]
        return self.run_command(
            [*command, "setup", "--config", "sc-lint.toml", *flags], cwd=consumer, environment=environment
        )

    def tool_environment(self, release: Path, root: Path) -> tuple[dict[str, str], FakeToolchain, Path]:
        """Stub cargo/rustup first on PATH and a private CARGO_HOME; the real ~/.cargo is never used."""
        fake = FakeToolchain(root / "fake-toolchain")
        cargo_home = root / "cargo-home"
        environment = self.fixture_environment(release)
        environment["PATH"] = str(fake.bin) + os.pathsep + environment["PATH"]
        environment["CARGO_HOME"] = str(cargo_home)
        return environment, fake, cargo_home

    def test_just_setup_provisions_declared_tools_from_the_persisted_install_input(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            environment, fake, cargo_home = self.tool_environment(release, Path(temporary.name))
            data = install_input(XWIN_TOOLS)
            data["minimum_version"] = VERSION
            persisted = consumer / ".sc-lint" / "install.json"
            persisted.write_text(json.dumps(data), encoding="utf-8")
            just = shutil.which("just")
            self.assertIsNotNone(just, "just is required for the consumer fixture")

            # Dry-run before the managed wheel exists reports, never bootstraps.
            unavailable = self.bootstrap_setup(consumer, environment, "--dry-run")
            self.assertEqual(unavailable.returncode, 4, unavailable.stdout + unavailable.stderr)
            self.assertIn("SETUP_TOOLS.INSPECTION_UNAVAILABLE", unavailable.stderr)
            self.assertFalse((consumer / ".sc-lint" / "venv").exists())
            self.assertEqual(fake.calls(), [])

            setup = self.run_command([just, "setup"], cwd=consumer, environment=environment)
            self.assertEqual(setup.returncode, 0, setup.stdout + setup.stderr)
            self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", setup.stdout)
            self.assertIn(f"+ rustup target add {WINDOWS_TARGET}", setup.stdout)
            self.assertEqual(
                fake.argvs("cargo"),
                [["install", "--list"], ["install", "--locked", "--version", "=0.18.4", "cargo-xwin"], ["install", "--list"]],
            )
            self.assertEqual(
                fake.argvs("rustup"),
                [["target", "list", "--installed"], ["target", "add", WINDOWS_TARGET], ["target", "list", "--installed"]],
            )
            for record in fake.calls():
                self.assertEqual(record["cargo_home"], str(cargo_home))
                self.assertEqual(Path(record["cwd"]).resolve(), consumer.resolve())
            self.assertEqual(FakeToolchain.installs(cargo_home)["cargo-xwin"]["version"], "0.18.4")
            self.assertTrue(
                (cargo_home / "bin" / ("cargo-xwin.exe" if os.name == "nt" else "cargo-xwin")).is_file()
            )
            self.assertEqual(fake.targets(), [WINDOWS_TARGET])
            self.assertEqual(json.loads(persisted.read_text(encoding="utf-8")), data)

            again = self.run_command([just, "setup"], cwd=consumer, environment=environment)
            self.assertEqual(again.returncode, 0, again.stdout + again.stderr)
            self.assertNotIn("+ ", again.stdout)
            check = self.bootstrap_setup(consumer, environment, "--dry-run")
            self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
            installs = [argv for argv in fake.argvs("cargo") if argv[:2] == ["install", "--locked"]]
            self.assertEqual(len(installs), 1)

    def test_just_setup_without_declared_tools_does_no_tool_work(self) -> None:
        temporary, release, consumer = self.make_release_fixture()
        with temporary:
            self.initialize_rust_consumer(release, consumer)
            environment, fake, cargo_home = self.tool_environment(release, Path(temporary.name))
            just = shutil.which("just")
            self.assertIsNotNone(just, "just is required for the consumer fixture")
            persisted = consumer / ".sc-lint" / "install.json"
            for label, data in (("no installer input", None), ("empty tools", install_input({}))):
                with self.subTest(label):
                    if data is not None:
                        persisted.write_text(json.dumps(data), encoding="utf-8")
                    setup = self.run_command([just, "setup"], cwd=consumer, environment=environment)
                    self.assertEqual(setup.returncode, 0, setup.stdout + setup.stderr)
                    self.assertNotIn("SETUP_TOOLS", setup.stdout + setup.stderr)
            self.assertEqual(fake.calls(), [])
            self.assertFalse(cargo_home.exists())

    def test_fixture_matrix_declares_each_supported_release_platform(self) -> None:
        mappings = {
            "Ubuntu": ("x86_64-unknown-linux-gnu", ".tar.gz", "sc-lint"),
            "macOS": ("aarch64-apple-darwin", ".tar.gz", "sc-lint"),
            "Windows": ("x86_64-pc-windows-msvc", ".zip", "sc-lint.exe"),
        }
        manifest = _toml.loads(
            (ROOT / "release/publish-artifacts.toml").read_text(encoding="utf-8")
        )
        release_targets = {entry["target"] for entry in manifest["release_targets"]}
        for platform, (target, extension, binary) in mappings.items():
            self.assertIn(target, release_targets, platform)
            self.assertTrue(extension.startswith("."), platform)
            self.assertTrue(binary.startswith("sc-lint"), platform)


@unittest.skipIf(os.name == "nt", "POSIX bootstrap; Windows bootstrap runs in test_consumer_lifecycle and phase_k_7")
class PosixBootstrapToolTests(HelperFixture):
    """Drive the product bootstrap asset with a stub product and seeded venv."""

    def setUp(self) -> None:
        super().setUp()
        self.log = self.tmp / "interpreters.log"
        self.managed = self.tmp / "managed"
        write_executable(
            self.managed / "sc-lint",
            f"#!/bin/sh\nprintf 'product %s\\n' \"$*\" >> '{self.log}'\nexit 0\n",
        )
        self.seed = self.tmp / "seed-bin"
        write_executable(
            self.seed / "python3",
            f"#!/bin/sh\nprintf 'seed %s\\n' \"$1\" >> '{self.log}'\nexec '{sys.executable}' \"$@\"\n",
        )
        (self.consumer / ".sc-lint" / "bootstrap.py").write_bytes(BOOTSTRAP.read_bytes())
        self.version = sc_lint.__version__.split("+")[0]
        self.config.write_text(f'[tool.sc-lint]\nminimum_version = "{self.version}"\n', encoding="utf-8")

    def seed_venv(self, consumer: Optional[Path] = None) -> None:
        """A ready `.sc-lint/venv` whose interpreter runs the sc_lint under test."""
        python = (consumer or self.consumer) / ".sc-lint" / "venv" / "bin" / "python3"
        write_executable(
            python,
            "#!/bin/sh\n"
            f"if [ \"$1\" = -m ]; then printf 'venv %s\\n' \"$*\" >> '{self.log}'; "
            f"else printf 'venv %s\\n' \"$1\" >> '{self.log}'; fi\n"
            f"PYTHONPATH='{PACKAGE_ROOT}' exec '{sys.executable}' \"$@\"\n",
        )

    def bootstrap(self, *arguments: str, consumer: Optional[Path] = None) -> subprocess.CompletedProcess:
        root = consumer or self.consumer
        environment = dict(self.environment)
        environment["PATH"] = f"{self.fake.bin}:{self.seed}:/usr/bin:/bin"
        environment["SC_LINT_INSTALL_DIR"] = str(self.managed)
        try:
            return subprocess.run(
                [sys.executable, str(root / ".sc-lint" / "bootstrap.py"), *arguments],
                cwd=root,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
                timeout=120,
            )
        except subprocess.TimeoutExpired as error:
            self.fail(f"bootstrap {arguments} timed out; stdout={error.stdout!r}; stderr={error.stderr!r}")

    def interpreter_log(self) -> List[str]:
        return self.log.read_text(encoding="utf-8").splitlines() if self.log.exists() else []

    def test_issue_197_declared_missing_tool_reproduces_then_setup_provisions(self) -> None:
        # The consumer declares its cross-target tool in its own input file.
        consumer = self.tmp / "issue-197-consumer"
        consumer.mkdir()
        data = install_input(XWIN_TOOLS)
        data["minimum_version"] = self.version
        data["profiles"] = {"clippy-windows": WINDOWS_CHECK}
        input_path = consumer / ".sc-lint" / "install.json"
        input_path.parent.mkdir()
        input_path.write_text(json.dumps(data), encoding="utf-8")
        persisted_before = input_path.read_bytes()
        install = subprocess.run(
            [sys.executable, str(ADOPTION_INSTALLER), "--input", str(input_path), str(consumer)],
            text=True,
            capture_output=True,
            check=False,
            timeout=120,
        )
        self.assertEqual(install.returncode, 0, install.stderr)
        persisted = consumer / ".sc-lint" / "install.json"
        self.assertEqual(json.loads(persisted.read_text(encoding="utf-8"))["tools"], XWIN_TOOLS)
        self.assertEqual(persisted.read_bytes(), persisted_before)
        self.assertFalse((consumer / ".sc-lint" / "installed.json").exists())
        self.seed_venv(consumer)
        check_environment = dict(self.environment)
        check_environment["PATH"] = f"{self.cargo_home / 'bin'}:{self.fake.bin}:/usr/bin:/bin"

        def consumer_check() -> subprocess.CompletedProcess:
            return subprocess.run(
                WINDOWS_CHECK, cwd=consumer, env=check_environment, text=True, capture_output=True, check=False, timeout=60
            )

        # Red: issue #197 — the declared cross-target step cannot run because
        # nothing provisioned cargo-xwin or the Windows target.
        red = consumer_check()
        self.assertEqual(red.returncode, 101, red.stderr)
        self.assertIn("error: no such command: `xwin`", red.stderr)
        self.assertNotIn("cargo-xwin", FakeToolchain.installs(self.cargo_home))
        self.assertNotIn(WINDOWS_TARGET, self.fake.targets())

        # Green: ordinary setup from the same consumer input provisions and
        # verifies both declarations without rerunning install.py.
        setup = self.bootstrap("setup", "--config", "sc-lint.toml", consumer=consumer)
        self.assertEqual(setup.returncode, 0, setup.stdout + setup.stderr)
        self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", setup.stdout)
        self.assertIn(f"+ rustup target add {WINDOWS_TARGET}", setup.stdout)
        self.assertIn("venv -m sc_lint.setup_tools --config sc-lint.toml", self.interpreter_log())
        self.assertEqual(
            self.fake.argvs("cargo")[-3:],
            [["install", "--list"], ["install", "--locked", "--version", "=0.18.4", "cargo-xwin"], ["install", "--list"]],
        )
        self.assertEqual(self.fake.argvs("rustup")[-1], ["target", "list", "--installed"])
        self.assertEqual(FakeToolchain.installs(self.cargo_home)["cargo-xwin"]["version"], "0.18.4")
        self.assertEqual(self.fake.targets(), [WINDOWS_TARGET])
        green = consumer_check()
        self.assertEqual(green.returncode, 0, green.stderr)
        self.assertIn("Finished", green.stderr)
        self.assertEqual(persisted.read_bytes(), persisted_before)
        self.assertFalse((consumer / ".sc-lint" / "installed.json").exists())

        rerun = self.bootstrap("setup", "--config", "sc-lint.toml", consumer=consumer)
        self.assertEqual(rerun.returncode, 0, rerun.stderr)
        self.assertNotIn("+ ", rerun.stdout)

    def test_dry_run_missing_helper_does_not_bootstrap(self) -> None:
        self.declare(XWIN_TOOLS)
        result = self.bootstrap("setup", "--config", str(self.config), "--dry-run")
        self.assertEqual(result.returncode, 4, result.stdout + result.stderr)
        self.assertIn("(SETUP_TOOLS.INSPECTION_UNAVAILABLE); run just setup", result.stderr)
        self.assertIn("declares tools that cannot be inspected", result.stderr)
        self.assertFalse((self.consumer / ".sc-lint" / "venv").exists())
        log = self.interpreter_log()
        self.assertNotIn("seed -m", log)
        self.assertEqual(self.fake.calls(), [])

        # With the managed helper present, dry-run forwards --check: inspect only.
        self.seed_venv()
        result = self.bootstrap("setup", "--config", str(self.config), "--dry-run")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"venv -m sc_lint.setup_tools --config {self.config} --check", self.interpreter_log())
        self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", result.stdout)
        self.assertNotIn(["install", "--locked", "--version", "=0.18.4", "cargo-xwin"], self.fake.argvs("cargo"))
        self.assertNotIn(["target", "add", WINDOWS_TARGET], self.fake.argvs("rustup"))
        self.assertFalse(self.cargo_home.exists())

    def test_absent_and_empty_tools_keep_simple_consumer_setup_unchanged(self) -> None:
        self.seed_venv()
        install_json = self.consumer / ".sc-lint" / "install.json"
        outcomes = []
        for label, data in (
            ("no installer input", None),
            ("absent", install_input(include_tools=False)),
            ("empty object", install_input({})),
            ("empty collections", install_input({"cargo": {}, "rustup_targets": []})),
        ):
            with self.subTest(label):
                if data is not None:
                    install_json.write_text(json.dumps(data), encoding="utf-8")
                for flags in ((), ("--dry-run",)):
                    result = self.bootstrap("setup", "--config", "sc-lint.toml", *flags)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    outcomes.append((flags, result.stdout, result.stderr))
        self.assertEqual(len({outcome for outcome in outcomes if outcome[0] == ()}), 1)
        self.assertEqual(len({outcome for outcome in outcomes if outcome[0] == ("--dry-run",)}), 1)
        self.assertFalse(any("sc_lint.setup_tools" in line for line in self.interpreter_log()))
        self.assertEqual(self.fake.calls(), [])

        install_json.write_text(json.dumps(install_input({"cargo": []})), encoding="utf-8")
        result = self.bootstrap("setup", "--config", "sc-lint.toml")
        self.assertEqual(result.returncode, setup_tools.EXIT_CONFIG, result.stdout + result.stderr)
        self.assertIn("tools.cargo must be an object", result.stderr)
        self.assertIn("venv -m sc_lint.setup_tools --config sc-lint.toml", self.interpreter_log())


if __name__ == "__main__":
    unittest.main()
