"""Declared tool provisioning (ADR-017 change 6, issue #197 provisioning half).

Every test that shells out to cargo or rustup uses the stub executables written
by `FakeToolchain` into a temporary directory placed first on PATH, with
CARGO_HOME and HOME pointed into that directory. No test installs a real tool
or reads or writes the real ~/.cargo.
"""
from __future__ import annotations

from contextlib import redirect_stderr
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Dict, List, Optional
import unittest
from unittest import mock

import sc_lint
from sc_lint import setup_tools
from sc_lint.lint_common import discover_repo_root


ROOT = discover_repo_root()
POSIX_BOOTSTRAP = ROOT / "crates" / "sc-lint" / "assets" / "bootstrap"
ADOPTION_INSTALLER = ROOT / "packages" / "sc-lint-adoption" / "install.py"
PACKAGE_ROOT = Path(sc_lint.__file__).resolve().parent.parent
WINDOWS_TARGET = "x86_64-pc-windows-msvc"
XWIN_TOOLS = {"cargo": {"cargo-xwin": "0.18.4"}, "rustup_targets": [WINDOWS_TARGET]}
WINDOWS_CHECK = ["cargo", "xwin", "check", "--target", WINDOWS_TARGET]
WINDOWS_ENVIRONMENT = ("SYSTEMROOT", "SystemRoot", "PATHEXT", "COMSPEC", "ComSpec", "TEMP", "TMP")

# The stub toolchain. It keeps cargo state under CARGO_HOME (as cargo does) and
# rustup state, call log and failure modes beside this script.
FAKE_TOOLCHAIN = r'''
import json, os, shutil, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def read_json(path, default):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return default


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def cargo_home():
    configured = os.environ.get("CARGO_HOME")
    if configured:
        return Path(configured)
    return Path(os.environ["USERPROFILE" if os.name == "nt" else "HOME"]) / ".cargo"


def mode(name):
    return read_json(HERE / "modes.json", {}).get(name, "ok")


def write_launcher(directory, name, tool):
    directory.mkdir(parents=True, exist_ok=True)
    if os.name == "nt":
        (directory / (name + ".exe")).write_bytes(b"fake executable")
        return
    path = directory / name
    path.write_text('#!/bin/sh\nexec "%s" "%s" %s "$@"\n' % (sys.executable, __file__, tool), encoding="utf-8")
    path.chmod(0o755)


def cargo(argv):
    state = cargo_home() / ".fake-crates.json"
    installs = read_json(state, {})
    if argv[:2] == ["install", "--list"]:
        for name in sorted(installs):
            print("%s v%s:" % (name, installs[name]["version"]))
            for executable in installs[name]["bins"]:
                print("    " + executable)
        return 0
    if argv[:1] == ["install"]:
        package = argv[-1]
        version = argv[argv.index("--version") + 1].lstrip("=")
        behaviour = mode("cargo_install")
        if behaviour == "fail":
            print("error: failed to compile `%s v%s`" % (package, version), file=sys.stderr)
            return 101
        if behaviour == "ok":
            installs[package] = {"version": version, "bins": [package]}
            write_json(state, installs)
            write_launcher(cargo_home() / "bin", package, "cargo-subcommand")
        print("  Installed package `%s v%s`" % (package, version), file=sys.stderr)
        return 0
    found = shutil.which("cargo-" + argv[0]) if argv else None
    if found is None:
        print("error: no such command: `%s`" % (argv[0] if argv else ""), file=sys.stderr)
        return 101
    return subprocess.call([found] + argv)


def rustup(argv):
    state = HERE / "rustup-targets.json"
    targets = read_json(state, [])
    if argv == ["target", "list", "--installed"]:
        for target in targets:
            print(target)
        return 0
    if argv[:2] == ["target", "add"] and len(argv) == 3:
        behaviour = mode("rustup_add")
        if behaviour == "fail":
            print("error: toolchain 'stable' does not support target '%s'" % argv[2], file=sys.stderr)
            return 1
        if behaviour == "ok":
            write_json(state, targets + [argv[2]])
        print("info: downloading component 'rust-std' for '%s'" % argv[2], file=sys.stderr)
        return 0
    print("error: unsupported fake rustup invocation: %r" % argv, file=sys.stderr)
    return 2


def cargo_subcommand(argv):
    targets = read_json(HERE / "rustup-targets.json", [])
    if "--target" in argv:
        target = argv[argv.index("--target") + 1]
        if target not in targets:
            print("error[E0463]: can't find crate for `core`", file=sys.stderr)
            print("  = note: the `%s` target may not be installed" % target, file=sys.stderr)
            return 101
    print("    Finished `dev` profile", file=sys.stderr)
    return 0


if __name__ == "__main__":
    tool, argv = sys.argv[1], sys.argv[2:]
    with (HERE / "calls.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(json.dumps({
            "tool": tool,
            "argv": argv,
            "cwd": os.getcwd(),
            "cargo_home": os.environ.get("CARGO_HOME"),
            "path": os.environ.get("PATH", ""),
        }) + "\n")
    tools = {"cargo": cargo, "rustup": rustup, "cargo-subcommand": cargo_subcommand}
    sys.exit(tools[tool](argv))
'''


def write_executable(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")
    path.chmod(0o755)


class FakeToolchain:
    """Stub cargo and rustup executables on a private PATH directory."""

    def __init__(self, directory: Path) -> None:
        self.directory = directory
        self.bin = directory / "bin"
        self.script = directory / "fake_toolchain.py"
        directory.mkdir(parents=True)
        self.script.write_text(FAKE_TOOLCHAIN, encoding="utf-8")
        for tool in ("cargo", "rustup"):
            self._launcher(tool)

    def _launcher(self, tool: str) -> None:
        if os.name == "nt":
            launcher = f'@"{sys.executable}" "{self.script}" {tool} %*\r\n@exit /b %ERRORLEVEL%\r\n'
            write_executable(self.bin / f"{tool}.cmd", launcher)
        else:
            write_executable(self.bin / tool, f'#!/bin/sh\nexec "{sys.executable}" "{self.script}" {tool} "$@"\n')

    def remove(self, tool: str) -> None:
        for candidate in self.bin.glob(f"{tool}*"):
            candidate.unlink()

    def set_mode(self, name: str, value: str) -> None:
        path = self.directory / "modes.json"
        modes = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
        modes[name] = value
        path.write_text(json.dumps(modes), encoding="utf-8")

    def calls(self, tool: Optional[str] = None) -> List[Dict[str, Any]]:
        path = self.directory / "calls.jsonl"
        if not path.exists():
            return []
        records = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
        return [record for record in records if tool is None or record["tool"] == tool]

    def argvs(self, tool: str) -> List[List[str]]:
        return [record["argv"] for record in self.calls(tool)]

    def targets(self) -> List[str]:
        path = self.directory / "rustup-targets.json"
        return json.loads(path.read_text(encoding="utf-8")) if path.exists() else []

    def add_target(self, target: str) -> None:
        (self.directory / "rustup-targets.json").write_text(json.dumps([*self.targets(), target]), encoding="utf-8")

    @staticmethod
    def installs(cargo_home: Path) -> Dict[str, Any]:
        path = cargo_home / ".fake-crates.json"
        return json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}

    @staticmethod
    def preinstall(cargo_home: Path, package: str, version: str, *, executable: bool = True) -> None:
        installs = FakeToolchain.installs(cargo_home)
        installs[package] = {"version": version, "bins": [package]}
        cargo_home.mkdir(parents=True, exist_ok=True)
        (cargo_home / ".fake-crates.json").write_text(json.dumps(installs), encoding="utf-8")
        if executable:
            (cargo_home / "bin").mkdir(parents=True, exist_ok=True)
            setup_tools.executable_path(cargo_home / "bin", package).write_bytes(b"installed")


def base_environment(path: str, home: Path) -> Dict[str, str]:
    environment = {name: os.environ[name] for name in WINDOWS_ENVIRONMENT if name in os.environ}
    if os.name == "nt":
        system32 = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32"
        path = os.pathsep.join([path, str(system32)])
        environment["USERPROFILE"] = str(home)
    environment["PATH"] = path
    environment["HOME"] = str(home)
    return environment


def install_input(tools: Any = None, *, include_tools: bool = True) -> Dict[str, Any]:
    data: Dict[str, Any] = {
        "minimum_version": "0.7.0",
        "profiles": {"ci": ["cargo", "check"]},
        "ci": {"os": ["linux"], "enabled": False},
        "analyzers": {},
    }
    if include_tools:
        data["tools"] = tools
    return data


class HelperFixture(unittest.TestCase):
    """Runs `setup_tools.main` in process against one temporary consumer."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory(prefix="sc-lint-setup-tools-")
        self.addCleanup(temporary.cleanup)
        self.tmp = Path(temporary.name)
        self.home = self.tmp / "home"
        self.home.mkdir()
        self.cargo_home = self.tmp / "cargo-home"
        self.consumer = self.tmp / "unrelated-widget-service"
        (self.consumer / ".sc-lint").mkdir(parents=True)
        self.config = self.consumer / "sc-lint.toml"
        self.config.write_text('[tool.sc-lint]\nminimum_version = "0.7.0"\n', encoding="utf-8")
        self.fake = FakeToolchain(self.tmp / "fake")
        self.environment = base_environment(str(self.fake.bin), self.home)
        self.environment["CARGO_HOME"] = str(self.cargo_home)

    def declare(self, tools: Any = None, *, include_tools: bool = True) -> None:
        data = install_input(tools, include_tools=include_tools)
        (self.consumer / ".sc-lint" / "install.json").write_text(json.dumps(data), encoding="utf-8")

    def run_helper(self, *arguments: str) -> tuple:
        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.dict(os.environ, self.environment, clear=True), redirect_stdout(stdout), redirect_stderr(stderr):
            code = setup_tools.main(["setup_tools", "--config", str(self.config), *arguments])
        return code, stdout.getvalue(), stderr.getvalue()

    def assert_failure_report(self, stderr: str, *fragments: str) -> None:
        for fragment in (*fragments, "recovery: fix the cause above, then run `just setup`"):
            self.assertIn(fragment, stderr)


class HelperProvisioningTests(HelperFixture):
    def test_exact_version_is_idempotent(self) -> None:
        self.declare(XWIN_TOOLS)
        FakeToolchain.preinstall(self.cargo_home, "cargo-xwin", "0.18.4")
        self.fake.add_target(WINDOWS_TARGET)
        for _ in range(2):
            code, stdout, stderr = self.run_helper()
            self.assertEqual(code, 0, stderr)
            self.assertIn("setup: cargo tool cargo-xwin 0.18.4 is installed", stdout)
            self.assertIn(f"setup: rustup target {WINDOWS_TARGET} is installed", stdout)
            self.assertNotIn("+ ", stdout)
        self.assertEqual(self.fake.argvs("cargo"), [["install", "--list"]] * 4)
        self.assertEqual(self.fake.argvs("rustup"), [["target", "list", "--installed"]] * 4)

    def test_wrong_version_is_reinstalled_and_verified(self) -> None:
        # Exact-version mismatch input from atm-core test_bootstrap.py
        # (`test_binstall_receipt_proves_exact_prebuilt_version`).
        listing = "cargo-audit v0.22.2:\n    cargo-audit\n"
        packages = setup_tools.parse_cargo_install_list(listing)
        self.assertEqual(packages, {"cargo-audit": ("0.22.2", ("cargo-audit",))})
        FakeToolchain.preinstall(self.cargo_home, "cargo-audit", "0.22.2")
        bin_dir = self.cargo_home / "bin"
        self.assertTrue(setup_tools.cargo_tool_matches(packages, "cargo-audit", "0.22.2", bin_dir))
        self.assertFalse(setup_tools.cargo_tool_matches(packages, "cargo-audit", "0.22.3", bin_dir))
        prefixed = setup_tools.parse_cargo_install_list("cargo-audit v0.22.20:\n    cargo-audit\n")
        self.assertFalse(setup_tools.cargo_tool_matches(prefixed, "cargo-audit", "0.22.2", bin_dir))

        self.declare({"cargo": {"cargo-audit": "0.22.3"}})
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, 0, stderr)
        self.assertIn("+ cargo install --locked --version =0.22.3 cargo-audit", stdout)
        self.assertEqual(
            self.fake.argvs("cargo"),
            [["install", "--list"], ["install", "--locked", "--version", "=0.22.3", "cargo-audit"], ["install", "--list"]],
        )
        self.assertEqual(FakeToolchain.installs(self.cargo_home)["cargo-audit"]["version"], "0.22.3")

    def test_matching_version_with_missing_executable_is_reinstalled(self) -> None:
        self.declare({"cargo": {"cargo-xwin": "0.18.4"}})
        FakeToolchain.preinstall(self.cargo_home, "cargo-xwin", "0.18.4", executable=False)
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, 0, stderr)
        self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", stdout)
        self.assertTrue(setup_tools.executable_path(self.cargo_home / "bin", "cargo-xwin").is_file())

    def test_rustup_target_is_added_only_when_missing(self) -> None:
        self.declare({"rustup_targets": ["aarch64-unknown-linux-gnu", WINDOWS_TARGET]})
        self.fake.add_target("aarch64-unknown-linux-gnu")
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, 0, stderr)
        self.assertIn("setup: rustup target aarch64-unknown-linux-gnu is installed", stdout)
        self.assertIn(f"+ rustup target add {WINDOWS_TARGET}", stdout)
        self.assertEqual(
            self.fake.argvs("rustup"),
            [["target", "list", "--installed"], ["target", "add", WINDOWS_TARGET], ["target", "list", "--installed"]],
        )
        self.assertEqual(self.fake.targets(), ["aarch64-unknown-linux-gnu", WINDOWS_TARGET])
        self.assertEqual(self.fake.argvs("cargo"), [])

        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, 0, stderr)
        self.assertNotIn("+ rustup target add", stdout)
        self.assertEqual([argv for argv in self.fake.argvs("rustup") if argv[:2] == ["target", "add"]], [["target", "add", WINDOWS_TARGET]])

    def test_setup_reports_missing_or_failed_installer(self) -> None:
        self.declare(XWIN_TOOLS)
        self.fake.set_mode("cargo_install", "fail")
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, setup_tools.EXIT_SETUP, stderr)
        self.assert_failure_report(
            stderr,
            "setup: cargo tool cargo-xwin 0.18.4: install command failed with exit 101 (SETUP_TOOLS.COMMAND_FAILED)",
            "command: cargo install --locked --version =0.18.4 cargo-xwin",
            "status: 101",
            "error: failed to compile `cargo-xwin v0.18.4`",
        )
        self.assertNotIn(["target", "add", WINDOWS_TARGET], self.fake.argvs("rustup"))

        self.declare({"rustup_targets": [WINDOWS_TARGET]})
        self.fake.set_mode("rustup_add", "fail")
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, setup_tools.EXIT_SETUP, stderr)
        self.assert_failure_report(
            stderr,
            f"setup: rustup target {WINDOWS_TARGET}: install command failed with exit 1",
            f"command: rustup target add {WINDOWS_TARGET}",
            "status: 1",
            f"does not support target '{WINDOWS_TARGET}'",
        )

        for tool, tools, command in (
            ("cargo", {"cargo": {"cargo-xwin": "0.18.4"}}, "command: cargo install --list"),
            ("rustup", {"rustup_targets": [WINDOWS_TARGET]}, "command: rustup target list --installed"),
        ):
            with self.subTest(missing=tool):
                self.fake.remove(tool)
                self.declare(tools)
                code, stdout, stderr = self.run_helper()
                self.assertEqual(code, setup_tools.EXIT_SETUP, stderr)
                self.assert_failure_report(
                    stderr,
                    f"required installer `{tool}` was not found on PATH (SETUP_TOOLS.INSTALLER_MISSING)",
                    command,
                )

    def test_successful_installer_without_expected_result_fails(self) -> None:
        self.declare({"cargo": {"cargo-xwin": "0.18.4"}})
        self.fake.set_mode("cargo_install", "noop")
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, setup_tools.EXIT_SETUP, stderr)
        self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", stdout)
        self.assert_failure_report(
            stderr,
            "setup: cargo tool cargo-xwin 0.18.4: `cargo install --list` does not show this exact version",
            "(SETUP_TOOLS.UNVERIFIED)",
            "command: cargo install --locked --version =0.18.4 cargo-xwin",
        )

        self.declare({"rustup_targets": [WINDOWS_TARGET]})
        self.fake.set_mode("rustup_add", "noop")
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, setup_tools.EXIT_SETUP, stderr)
        self.assert_failure_report(
            stderr,
            f"setup: rustup target {WINDOWS_TARGET}: `rustup target list --installed` does not show this target",
            "(SETUP_TOOLS.UNVERIFIED)",
            f"command: rustup target add {WINDOWS_TARGET}",
        )

    def test_nondefault_cargo_home_is_used(self) -> None:
        self.declare({"cargo": {"cargo-xwin": "0.18.4"}})
        custom = self.tmp / "custom cargo home"
        self.environment["CARGO_HOME"] = str(custom)
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, 0, stderr)
        self.assertTrue(setup_tools.executable_path(custom / "bin", "cargo-xwin").is_file())
        self.assertFalse((self.home / ".cargo").exists())
        for record in self.fake.calls("cargo"):
            self.assertEqual(record["cargo_home"], str(custom))
            self.assertEqual(record["path"].split(os.pathsep)[0], str(custom / "bin"))
            self.assertEqual(Path(record["cwd"]).resolve(), self.consumer.resolve())

        del self.environment["CARGO_HOME"]
        code, stdout, stderr = self.run_helper()
        self.assertEqual(code, 0, stderr)
        default = self.home / ".cargo"
        self.assertTrue(setup_tools.executable_path(default / "bin", "cargo-xwin").is_file())
        self.assertEqual(self.fake.calls("cargo")[-1]["cargo_home"], str(default))
        self.assertEqual(setup_tools.cargo_home({"HOME": "/h", "USERPROFILE": "/h"}), Path("/h") / ".cargo")
        self.assertEqual(setup_tools.cargo_home({"CARGO_HOME": "/elsewhere", "HOME": "/h"}), Path("/elsewhere"))

    def test_dry_run_never_executes_installers(self) -> None:
        # Ported from atm-core test_bootstrap.py: with every tool missing, a
        # dry run renders install commands but never executes or verifies them.
        self.declare(XWIN_TOOLS)
        with (
            mock.patch.object(setup_tools, "installed_cargo_packages", return_value={}),
            mock.patch.object(setup_tools, "cargo_tool_matches", return_value=False),
            mock.patch.object(setup_tools, "installed_rustup_targets", return_value=frozenset()),
            mock.patch.object(setup_tools, "verify_declared_tools") as verify,
            mock.patch.object(setup_tools.subprocess, "Popen") as execute,
            mock.patch.object(setup_tools.subprocess, "run") as run,
        ):
            code, stdout, stderr = self.run_helper("--check")
        self.assertEqual(code, 0, stderr)
        self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", stdout)
        execute.assert_not_called()
        run.assert_not_called()
        verify.assert_not_called()

        # Against the stub toolchain, --check inspects but never installs or writes.
        code, stdout, stderr = self.run_helper("--check")
        self.assertEqual(code, 0, stderr)
        self.assertIn("+ cargo install --locked --version =0.18.4 cargo-xwin", stdout)
        self.assertIn(f"+ rustup target add {WINDOWS_TARGET}", stdout)
        self.assertEqual(self.fake.argvs("cargo"), [["install", "--list"]])
        self.assertEqual(self.fake.argvs("rustup"), [["target", "list", "--installed"]])
        self.assertFalse(self.cargo_home.exists())
        self.assertEqual(self.fake.targets(), [])

    def test_tools_absent_empty_and_malformed(self) -> None:
        install_json = self.consumer / ".sc-lint" / "install.json"
        self.assertEqual(self.run_helper(), (0, "", ""))
        for label, data in (
            ("absent", install_input(include_tools=False)),
            ("empty object", install_input({})),
            ("empty collections", install_input({"cargo": {}, "rustup_targets": []})),
        ):
            with self.subTest(label):
                install_json.write_text(json.dumps(data), encoding="utf-8")
                self.assertEqual(self.run_helper(), (0, "", ""))
                self.assertEqual(self.run_helper("--check"), (0, "", ""))
        self.assertEqual(self.fake.calls(), [])

        malformed = (
            (install_input([]), "tools must be an object"),
            (install_input(None), "tools must be an object"),
            (install_input({"pip": {}}), "tools has unknown keys: ['pip']"),
            (install_input({"cargo": []}), "tools.cargo must be an object"),
            (install_input({"cargo": {"cargo-xwin": "0.18"}}), "tools.cargo.cargo-xwin must be an exact stable version"),
            (install_input({"cargo": {"cargo-xwin": "^0.18.4"}}), "found '^0.18.4'"),
            (install_input({"cargo": {"cargo-xwin": "0.18.4-beta.1"}}), "found '0.18.4-beta.1'"),
            (install_input({"cargo": {"cargo-xwin": "01.2.3"}}), "found '01.2.3'"),
            (install_input({"cargo": {"cargo-xwin": 18}}), "found 18"),
            (install_input({"cargo": {"cargo xwin": "0.18.4"}}), "invalid cargo package name: 'cargo xwin'"),
            (install_input({"cargo": {"9xwin": "0.18.4"}}), "invalid cargo package name: '9xwin'"),
            (install_input({"rustup_targets": WINDOWS_TARGET}), "tools.rustup_targets must be an array"),
            (install_input({"rustup_targets": [""]}), "tools.rustup_targets[0] must be a nonempty target identifier"),
            (install_input({"rustup_targets": [WINDOWS_TARGET, " "]}), "tools.rustup_targets[1]"),
            (install_input({"rustup_targets": [7]}), "found 7"),
            ([], "install input must be a JSON object"),
        )
        for data, fragment in malformed:
            with self.subTest(fragment):
                install_json.write_text(json.dumps(data), encoding="utf-8")
                code, stdout, stderr = self.run_helper()
                self.assertEqual(code, setup_tools.EXIT_CONFIG, stderr)
                self.assertIn(fragment, stderr)
                self.assertIn("(SETUP_TOOLS.CONFIG_INVALID)", stderr)
                self.assertIn("then run `just setup`", stderr)
        install_json.write_text("{not json", encoding="utf-8")
        code, stdout, stderr = self.run_helper("--check")
        self.assertEqual(code, setup_tools.EXIT_CONFIG)
        self.assertIn("is not valid JSON", stderr)
        self.assertEqual(self.fake.calls(), [])


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
        (self.consumer / ".sc-lint" / "bootstrap").write_bytes(POSIX_BOOTSTRAP.read_bytes())
        (self.consumer / ".sc-lint" / "bootstrap").chmod(0o755)
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
                ["sh", str(root / ".sc-lint" / "bootstrap"), *arguments],
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
        # The consumer is installed once by the adoption kit with a declared
        # cross-target tool, persisting `.sc-lint/install.json`.
        consumer = self.tmp / "issue-197-consumer"
        consumer.mkdir()
        data = install_input(XWIN_TOOLS)
        data["minimum_version"] = self.version
        data["profiles"] = {"clippy-windows": WINDOWS_CHECK}
        input_path = self.tmp / "install-input.json"
        input_path.write_text(json.dumps(data), encoding="utf-8")
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
        persisted_before = persisted.read_bytes()
        manifest_before = (consumer / ".sc-lint" / "installed.json").read_bytes()
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

        # Green: ordinary setup from the same persisted input provisions and
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
        self.assertEqual((consumer / ".sc-lint" / "installed.json").read_bytes(), manifest_before)

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
        self.assertIn("seed -c", log)
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
