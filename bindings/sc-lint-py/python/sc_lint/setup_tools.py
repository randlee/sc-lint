#!/usr/bin/env python3
"""Provision consumer-declared cargo tools and rustup targets during setup.

ADR-017 change 6: ordinary `.sc-lint/bootstrap.py setup --config PATH` runs this
managed-wheel helper after it has ensured the product and the wheel. The helper
reads only the optional `tools` object of the persisted installer input,
`PATH.parent/.sc-lint/install.json`. An absent object, or empty collections,
means there is nothing to do. Every package, version and target is consumer
input; the product declares none.

`--check` inspects and prints the commands setup would run, but never installs
or writes anything.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from typing import Dict, List, Mapping, Optional, Sequence, Tuple


INSTALL_INPUT = Path(".sc-lint") / "install.json"
TOOL_KEYS = ("cargo", "rustup_targets")
EXACT_VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
CARGO_PACKAGE = re.compile(r"[A-Za-z][A-Za-z0-9_-]{0,63}")
TARGET_IDENTIFIER = re.compile(r"\S+")
LISTED_PACKAGE = re.compile(r"^(?P<name>\S+) v(?P<version>\S+?)(?: \(.*\))?:$")
RECOVERY = "fix the cause above, then run `just setup`"
EXIT_CONFIG = 3
EXIT_SETUP = 4
STDERR_TAIL_LINES = 20

InstalledPackages = Dict[str, Tuple[str, Tuple[str, ...]]]


class ToolsConfigError(ValueError):
    """The persisted `tools` declaration is malformed."""


class SetupError(RuntimeError):
    """A declared tool or target could not be inspected, installed or verified."""

    def __init__(
        self,
        item: str,
        command: Sequence[str],
        detail: str,
        *,
        status: Optional[int] = None,
        stderr: str = "",
    ) -> None:
        super().__init__(detail)
        self.item = item
        self.command = tuple(command)
        self.status = status
        self.stderr = stderr

    def report(self) -> str:
        lines = [f"setup: {self.item}: {self}"]
        if self.command:
            lines.append(f"  command: {' '.join(self.command)}")
        if self.status is not None:
            lines.append(f"  status: {self.status}")
        stderr = self.stderr.strip()
        if stderr:
            lines.append("  stderr:")
            lines.extend(f"    {line}" for line in stderr.splitlines())
        lines.append(f"  recovery: {RECOVERY}")
        return "\n".join(lines)


@dataclass(frozen=True)
class ToolDeclaration:
    cargo: Tuple[Tuple[str, str], ...] = ()
    rustup_targets: Tuple[str, ...] = ()

    @property
    def empty(self) -> bool:
        return not self.cargo and not self.rustup_targets


@dataclass(frozen=True)
class SetupContext:
    """The selected consumer root, child environment and mode for one run."""

    root: Path
    env: Mapping[str, str]
    dry_run: bool

    @property
    def cargo_bin(self) -> Path:
        return Path(self.env["CARGO_HOME"]) / "bin"


def _cargo_pins(value: object) -> Tuple[Tuple[str, str], ...]:
    if not isinstance(value, dict):
        raise ToolsConfigError("tools.cargo must be an object mapping package names to exact versions")
    pins = []
    for package, version in value.items():
        if not CARGO_PACKAGE.fullmatch(package):
            raise ToolsConfigError(f"tools.cargo has an invalid cargo package name: {package!r}")
        if not isinstance(version, str) or not EXACT_VERSION.fullmatch(version):
            raise ToolsConfigError(
                f"tools.cargo.{package} must be an exact stable version such as 1.2.3; found {version!r}"
            )
        pins.append((package, version))
    return tuple(pins)


def _rustup_targets(value: object) -> Tuple[str, ...]:
    if not isinstance(value, list):
        raise ToolsConfigError("tools.rustup_targets must be an array of target identifiers")
    for index, target in enumerate(value):
        if not isinstance(target, str) or not TARGET_IDENTIFIER.fullmatch(target):
            raise ToolsConfigError(
                f"tools.rustup_targets[{index}] must be a nonempty target identifier; found {target!r}"
            )
    return tuple(value)


def parse_tools(data: object) -> ToolDeclaration:
    """Validate the `tools` object of one installer input; absent means empty."""
    if not isinstance(data, dict):
        raise ToolsConfigError("install input must be a JSON object")
    if "tools" not in data:
        return ToolDeclaration()
    tools = data["tools"]
    if not isinstance(tools, dict):
        raise ToolsConfigError("tools must be an object")
    unknown = sorted(set(tools) - set(TOOL_KEYS))
    if unknown:
        raise ToolsConfigError(f"tools has unknown keys: {unknown}")
    return ToolDeclaration(
        _cargo_pins(tools.get("cargo", {})),
        _rustup_targets(tools.get("rustup_targets", [])),
    )


def load_tools(root: Path) -> ToolDeclaration:
    path = root / INSTALL_INPUT
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ToolDeclaration()
    except OSError as error:
        raise ToolsConfigError(f"cannot read {path}: {error}") from error
    try:
        data = json.loads(text)
    except json.JSONDecodeError as error:
        raise ToolsConfigError(f"{path} is not valid JSON: {error}") from error
    try:
        return parse_tools(data)
    except ToolsConfigError as error:
        raise ToolsConfigError(f"{path}: {error}") from None


def cargo_home(environ: Mapping[str, str]) -> Path:
    """CARGO_HOME from the environment, else the platform home's `.cargo`."""
    configured = environ.get("CARGO_HOME")
    if configured:
        return Path(configured)
    home = environ.get("USERPROFILE" if os.name == "nt" else "HOME")
    return (Path(home) if home else Path.home()) / ".cargo"


def tool_environment(environ: Mapping[str, str]) -> Dict[str, str]:
    """Child environment with the effective CARGO_HOME and its `bin` on PATH."""
    home = cargo_home(environ)
    env = dict(environ)
    env["CARGO_HOME"] = str(home)
    inherited = environ.get("PATH", "")
    env["PATH"] = os.pathsep.join(part for part in (str(home / "bin"), inherited) if part)
    return env


def _resolve(command: Sequence[str], item: str, context: SetupContext) -> List[str]:
    executable = shutil.which(command[0], path=context.env.get("PATH"))
    if executable is None:
        raise SetupError(
            item,
            command,
            f"required installer `{command[0]}` was not found on PATH (SETUP_TOOLS.INSTALLER_MISSING)",
        )
    return [executable, *command[1:]]


def command_output(command: Sequence[str], item: str, context: SetupContext) -> str:
    """Run one inspection command and return its combined output.

    Ported from atm-core `tools/bootstrap.py` `command_output`; it runs from the
    selected consumer root and reports failures as a `SetupError`.
    """
    result = subprocess.run(
        _resolve(command, item, context),
        capture_output=True,
        check=False,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=context.root,
        env=dict(context.env),
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise SetupError(
            item,
            command,
            f"{' '.join(command)} failed: {detail} (SETUP_TOOLS.COMMAND_FAILED)",
            status=result.returncode,
            stderr=result.stderr,
        )
    return f"{result.stdout}\n{result.stderr}".strip()


def run(command: Sequence[str], item: str, context: SetupContext) -> None:
    """Execute one installation step, or only print it during `--check`.

    Ported from atm-core `tools/bootstrap.py` `run`. Standard error is still
    streamed to the operator, and its tail is kept for the failure report.
    """
    print("+", " ".join(command), flush=True)
    if context.dry_run:
        return
    process = subprocess.Popen(
        _resolve(command, item, context),
        cwd=context.root,
        env=dict(context.env),
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    captured: List[str] = []
    assert process.stderr is not None
    for line in process.stderr:
        sys.stderr.write(line)
        captured.append(line)
    status = process.wait()
    if status != 0:
        raise SetupError(
            item,
            command,
            f"install command failed with exit {status} (SETUP_TOOLS.COMMAND_FAILED)",
            status=status,
            stderr="".join(captured[-STDERR_TAIL_LINES:]),
        )


def parse_cargo_install_list(listing: str) -> InstalledPackages:
    """Map each `cargo install --list` package to its version and executables."""
    packages: Dict[str, Tuple[str, List[str]]] = {}
    current: Optional[str] = None
    for line in listing.splitlines():
        match = LISTED_PACKAGE.match(line)
        if match:
            current = match.group("name")
            packages[current] = (match.group("version"), [])
        elif current is not None and line[:1].isspace() and line.strip():
            packages[current][1].append(line.strip())
    return {name: (version, tuple(executables)) for name, (version, executables) in packages.items()}


def executable_path(bin_dir: Path, name: str) -> Path:
    if os.name == "nt" and not name.lower().endswith(".exe"):
        name = f"{name}.exe"
    return bin_dir / name


def cargo_tool_matches(packages: InstalledPackages, package: str, version: str, bin_dir: Path) -> bool:
    """Only the exact version with every listed executable present matches."""
    installed = packages.get(package)
    if installed is None or installed[0] != version or not installed[1]:
        return False
    return all(executable_path(bin_dir, name).is_file() for name in installed[1])


def cargo_install_command(package: str, version: str) -> List[str]:
    return ["cargo", "install", "--locked", "--version", f"={version}", package]


def _cargo_item(declaration: ToolDeclaration) -> str:
    return "cargo tools " + ", ".join(f"{package} {version}" for package, version in declaration.cargo)


def installed_cargo_packages(declaration: ToolDeclaration, context: SetupContext) -> InstalledPackages:
    listing = command_output(["cargo", "install", "--list"], _cargo_item(declaration), context)
    return parse_cargo_install_list(listing)


def installed_rustup_targets(declaration: ToolDeclaration, context: SetupContext) -> frozenset:
    item = "rustup targets " + ", ".join(declaration.rustup_targets)
    listing = command_output(["rustup", "target", "list", "--installed"], item, context)
    return frozenset(line.strip() for line in listing.splitlines() if line.strip())


def provision_cargo_tools(declaration: ToolDeclaration, context: SetupContext) -> None:
    packages = installed_cargo_packages(declaration, context)
    for package, version in declaration.cargo:
        item = f"cargo tool {package} {version}"
        if cargo_tool_matches(packages, package, version, context.cargo_bin):
            print(f"setup: {item} is installed")
            continue
        run(cargo_install_command(package, version), item, context)


def provision_rustup_targets(declaration: ToolDeclaration, context: SetupContext) -> None:
    installed = installed_rustup_targets(declaration, context)
    for target in declaration.rustup_targets:
        item = f"rustup target {target}"
        if target in installed:
            print(f"setup: {item} is installed")
            continue
        run(["rustup", "target", "add", target], item, context)


def verify_declared_tools(declaration: ToolDeclaration, context: SetupContext) -> None:
    """Prove every declared tool and target is present rather than trusting installs."""
    if declaration.cargo:
        packages = installed_cargo_packages(declaration, context)
        for package, version in declaration.cargo:
            if not cargo_tool_matches(packages, package, version, context.cargo_bin):
                raise SetupError(
                    f"cargo tool {package} {version}",
                    cargo_install_command(package, version),
                    "`cargo install --list` does not show this exact version with its executables in "
                    f"{context.cargo_bin} (SETUP_TOOLS.UNVERIFIED)",
                )
    if declaration.rustup_targets:
        installed = installed_rustup_targets(declaration, context)
        for target in declaration.rustup_targets:
            if target not in installed:
                raise SetupError(
                    f"rustup target {target}",
                    ["rustup", "target", "add", target],
                    "`rustup target list --installed` does not show this target (SETUP_TOOLS.UNVERIFIED)",
                )


def provision(declaration: ToolDeclaration, context: SetupContext) -> None:
    """Install what is missing, then verify the complete declaration."""
    if declaration.cargo:
        provision_cargo_tools(declaration, context)
    if declaration.rustup_targets:
        provision_rustup_targets(declaration, context)
    if not context.dry_run:
        verify_declared_tools(declaration, context)


def main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(prog="python -m sc_lint.setup_tools", description=__doc__.splitlines()[0])
    parser.add_argument("--config", required=True, type=Path, help="consumer sc-lint.toml")
    parser.add_argument("--check", action="store_true", help="inspect and print commands without installing")
    args = parser.parse_args(list(argv[1:]))
    root = args.config.absolute().parent
    try:
        declaration = load_tools(root)
    except ToolsConfigError as error:
        print(
            f"setup: invalid tools declaration: {error} (SETUP_TOOLS.CONFIG_INVALID)\n"
            "  recovery: correct the tools object in .sc-lint/install.json, then run `just setup`",
            file=sys.stderr,
        )
        return EXIT_CONFIG
    if declaration.empty:
        return 0
    context = SetupContext(root, tool_environment(os.environ), args.check)
    try:
        provision(declaration, context)
    except SetupError as error:
        print(error.report(), file=sys.stderr)
        return EXIT_SETUP
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
