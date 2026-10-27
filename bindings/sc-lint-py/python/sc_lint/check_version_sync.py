#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
import re
import sys
import tomllib
from pathlib import Path, PureWindowsPath
from typing import Any

from sc_lint.lint_common import ConfigError
from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import resolve_table
from sc_lint.lint_common import table_missing_error
from sc_lint.lint_common import workspace_crate_section_lines
from sc_lint.lint_common import workspace_manifest_paths
from sc_lint.python_adapter import AdapterError
from sc_lint.python_adapter import error_payload
from sc_lint.python_adapter import success_payload
from sc_lint.python_adapter import write_json


@dataclass(frozen=True)
class Finding:
    check: str
    file: str
    expected: str
    actual: str

    def to_dict(self) -> dict[str, str]:
        return {
            "check": self.check,
            "file": self.file,
            "expected": self.expected,
            "actual": self.actual,
        }


def read_text(path: Path, root: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise AdapterError("config", f"cannot read {_relative_display(path, root)}: {error}") from error


def read_toml(path: Path, root: Path) -> dict[str, Any]:
    rel = _relative_display(path, root)
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError) as error:
        raise AdapterError("config", f"cannot read {rel}: {error}") from error
    except tomllib.TOMLDecodeError as error:
        raise AdapterError("config", f"failed to parse {rel}: {error}") from error


def _relative_display(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def dependency_sections(manifest: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    sections: list[tuple[str, dict[str, Any]]] = []
    for section_name in ("dependencies", "dev-dependencies", "build-dependencies"):
        dependencies = manifest.get(section_name)
        if isinstance(dependencies, dict):
            sections.append((section_name, dependencies))

    targets = manifest.get("target", {})
    if isinstance(targets, dict):
        for target_name, target in targets.items():
            if not isinstance(target, dict):
                continue
            for section_name in ("dependencies", "dev-dependencies", "build-dependencies"):
                dependencies = target.get(section_name)
                if isinstance(dependencies, dict):
                    sections.append((f"target.{target_name}.{section_name}", dependencies))
    return sections


def extract_version_from_url(url: str) -> str | None:
    match = re.search(r"/download/v(?P<version>\d+\.\d+\.\d+)/", url)
    if match:
        return match.group("version")
    match = re.search(r"[_-](?P<version>\d+\.\d+\.\d+)[_/]", url)
    if match:
        return match.group("version")
    return None


def validate_workspace_version(repo_root: Path) -> str:
    cargo_toml = read_toml(repo_root / "Cargo.toml", repo_root)
    workspace = cargo_toml.get("workspace")
    package = workspace.get("package") if isinstance(workspace, dict) else None
    version = package.get("version") if isinstance(package, dict) else None
    if not isinstance(version, str) or not version.strip():
        raise AdapterError("config", "workspace version missing from Cargo.toml")
    return version


def expected_package_version(manifest: dict[str, Any], workspace_version: str) -> str | None:
    package = manifest.get("package")
    if not isinstance(package, dict):
        return None
    version_value = package.get("version")
    if isinstance(version_value, str) and version_value.strip():
        return version_value
    if isinstance(version_value, dict) and version_value.get("workspace") is True:
        return workspace_version
    return None


def describe_invalid_version(manifest: dict[str, Any]) -> str:
    package = manifest.get("package")
    if not isinstance(package, dict):
        return "no [package] table"
    if "version" not in package:
        return "absent"
    return "invalid"


def validate_crate_versions(repo_root: Path, workspace_version: str) -> list[Finding]:
    manifests = sorted(workspace_manifest_paths(repo_root))
    if not manifests:
        raise AdapterError("config", "no workspace member manifests found")

    workspace_member_dirs = {manifest_path.parent.resolve() for manifest_path in manifests}
    manifest_payloads: dict[Path, tuple[dict[str, Any], str]] = {}
    expected_versions: dict[Path, str | None] = {}
    findings: list[Finding] = []

    for path in manifests:
        manifest = read_toml(path, repo_root)
        rel_manifest = _relative_display(path, repo_root)
        manifest_payloads[path] = (manifest, rel_manifest)
        expected = expected_package_version(manifest, workspace_version)
        expected_versions[path.parent.resolve()] = expected
        if expected is None:
            findings.append(
                Finding(
                    "crate_version",
                    rel_manifest,
                    "non-empty string or version.workspace = true",
                    describe_invalid_version(manifest),
                )
            )

    for path, (manifest, rel_manifest) in manifest_payloads.items():
        for section_name, dependencies in dependency_sections(manifest):
            for dependency_name, dependency in dependencies.items():
                if not isinstance(dependency, dict):
                    continue
                dependency_path = dependency.get("path")
                if not isinstance(dependency_path, str):
                    continue
                resolved_path = (path.parent / dependency_path).resolve()
                if resolved_path not in workspace_member_dirs:
                    continue
                target_version = expected_versions[resolved_path]
                if target_version is None:
                    continue
                pinned_version = dependency.get("version")
                actual = pinned_version if isinstance(pinned_version, str) else (
                    "absent" if pinned_version is None else "invalid"
                )
                if actual != target_version:
                    findings.append(
                        Finding(
                            "internal_path_dep",
                            rel_manifest,
                            f"{section_name}.{dependency_name} version {target_version}",
                            actual,
                        )
                    )
    return findings


def validate_lockfile(repo_root: Path, workspace_version: str) -> tuple[list[Finding], list[dict[str, str]]]:
    lock_path = repo_root / "Cargo.lock"
    if not lock_path.exists():
        return [], [{"check": "cargo_lock", "file": "Cargo.lock", "reason": "missing"}]

    lock = read_toml(lock_path, repo_root)
    packages = lock.get("package", [])
    if not isinstance(packages, list):
        packages = []
    locked_versions: dict[str, set[str]] = {}
    workspace_packages: dict[str, str] = {}
    for manifest_path in sorted(workspace_manifest_paths(repo_root)):
        manifest = read_toml(manifest_path, repo_root)
        package = manifest.get("package")
        package_name = package.get("name") if isinstance(package, dict) else None
        expected = expected_package_version(manifest, workspace_version)
        if isinstance(package_name, str) and expected is not None:
            workspace_packages[package_name] = expected
    for package in packages:
        if not isinstance(package, dict):
            continue
        name = package.get("name")
        version = package.get("version")
        if isinstance(name, str) and isinstance(version, str):
            locked_versions.setdefault(name, set()).add(version)

    findings = []
    for package_name in sorted(workspace_packages):
        expected = workspace_packages[package_name]
        actual_versions = locked_versions.get(package_name, set())
        if expected in actual_versions:
            continue
        actual = f"{package_name} {sorted(actual_versions)[0]}" if actual_versions else f"{package_name} absent"
        if actual != f"{package_name} {expected}":
            findings.append(Finding("cargo_lock", "Cargo.lock", f"{package_name} {expected}", actual))
    return findings, []


def subtable(config: dict[str, Any], name: str) -> dict[str, Any]:
    value = config.get(name, {})
    if not isinstance(value, dict):
        raise AdapterError("config", f"[version_sync.{name}] must be a TOML table")
    return value


def python_settings(config: dict[str, Any]) -> tuple[bool, list[str]]:
    python = subtable(config, "python")
    cargo_lock = python.get("cargo_lock", True)
    if not isinstance(cargo_lock, bool):
        raise AdapterError("config", "[version_sync.python].cargo_lock must be a boolean")
    python_manifests = python.get("python_manifests", [])
    if not isinstance(python_manifests, list) or not all(isinstance(item, str) for item in python_manifests):
        raise AdapterError("config", "[version_sync.python].python_manifests must be an array of strings")
    return cargo_lock, python_manifests


def repo_relative(repo_root: Path, value: str, key: str) -> Path:
    relative = Path(value)
    windows_relative = PureWindowsPath(value)
    invalid = (
        relative.is_absolute()
        or windows_relative.is_absolute()
        or any(part == ".." for part in relative.parts)
        or any(part == ".." for part in windows_relative.parts)
    )
    candidate = (repo_root / relative).resolve()
    if invalid or not candidate.is_relative_to(repo_root.resolve()):
        raise AdapterError("config", f"[version_sync.{key.rsplit('.', 1)[0]}].{key.rsplit('.', 1)[1]} must be a repo-relative path inside the root")
    return candidate


def validate_python_manifests(
    root: Path,
    ws: str,
    config: dict[str, Any],
) -> tuple[bool, list[Finding]]:
    _cargo_lock_enabled, python_manifests = python_settings(config)
    findings: list[Finding] = []
    for manifest_name in python_manifests:
        manifest_path = repo_relative(root, manifest_name, "python.python_manifests")
        rel_manifest = manifest_path.relative_to(root.resolve()).as_posix()
        if not manifest_path.is_file():
            findings.append(Finding("python.manifest_missing", rel_manifest, "file exists", "absent"))
            continue
        project_file = read_toml(manifest_path, root)
        project = project_file.get("project")
        project = project if isinstance(project, dict) else {}
        version = project.get("version")
        if isinstance(version, str):
            actual = version
        elif version is None:
            dynamic = project.get("dynamic", [])
            actual = "dynamic" if isinstance(dynamic, list) and "version" in dynamic else "absent"
        else:
            actual = "invalid"
        if actual != ws:
            findings.append(Finding("python.version", rel_manifest, ws, actual))
    return bool(python_manifests), findings


def _enabled(table: dict[str, Any], table_name: str) -> bool:
    enabled = table.get("enabled", False)
    if not isinstance(enabled, bool):
        raise AdapterError("config", f"[version_sync.{table_name}].enabled must be a boolean")
    return enabled


def validate_winget_manifests(
    repo_root: Path,
    workspace_version: str,
    config: dict[str, Any],
) -> tuple[bool, list[Finding]]:
    winget = subtable(config, "winget")
    if not _enabled(winget, "winget"):
        return False, []

    manifest_glob = winget.get("manifest_glob")
    if not isinstance(manifest_glob, str) or not manifest_glob.strip():
        raise AdapterError("config", "[version_sync.winget].manifest_glob must be a non-empty string when enabled")
    glob_path = repo_relative(repo_root, manifest_glob, "winget.manifest_glob")
    relative_glob = glob_path.relative_to(repo_root.resolve()).as_posix()
    fields = {
        "package_version_field": winget.get("package_version_field", "PackageVersion"),
        "manifest_version_field": winget.get("manifest_version_field", "ManifestVersion"),
        "installer_url_field": winget.get("installer_url_field", "InstallerUrl"),
    }
    if not all(isinstance(value, str) and value for value in fields.values()):
        raise AdapterError("config", "[version_sync.winget] field names must be non-empty strings")

    manifest_paths = sorted((path for path in repo_root.glob(relative_glob) if path.is_file()), key=lambda path: path.as_posix())
    if not manifest_paths:
        return True, [Finding("winget.manifest_glob", manifest_glob, "at least one manifest", "none")]

    findings: list[Finding] = []
    for manifest_path in manifest_paths:
        rel_manifest = _relative_display(manifest_path, repo_root)
        text = read_text(manifest_path, repo_root)
        for key, check in (("package_version_field", "winget.package_version"), ("manifest_version_field", "winget.manifest_version")):
            field = fields[key]
            match = re.search(rf"^{re.escape(field)}:\s*(?P<value>\S+)\s*$", text, re.MULTILINE)
            if match is None:
                findings.append(Finding("winget.missing_field", rel_manifest, field, "absent"))
                continue
            actual = match.group("value")
            if actual != workspace_version:
                findings.append(Finding(check, rel_manifest, f"{field} {workspace_version}", f"{field} {actual}"))

        url_field = fields["installer_url_field"]
        installer_urls = re.findall(rf"^\s*{re.escape(url_field)}:\s*(?P<value>\S+)\s*$", text, re.MULTILINE)
        if not installer_urls:
            findings.append(Finding("winget.missing_field", rel_manifest, url_field, "absent"))
        for installer_url in installer_urls:
            installer_version = extract_version_from_url(installer_url)
            if installer_version != workspace_version:
                findings.append(
                    Finding(
                        "winget.installer_url",
                        rel_manifest,
                        f"{url_field} download version {workspace_version}",
                        installer_url,
                    )
                )
    return True, findings


def validate_release_wiring(repo_root: Path, config: dict[str, Any]) -> tuple[bool, list[Finding]]:
    release_wiring = subtable(config, "release_wiring")
    if not _enabled(release_wiring, "release_wiring"):
        return False, []

    file_name = release_wiring.get("file")
    fragments = release_wiring.get("required_fragments", [])
    if not isinstance(file_name, str) or not file_name.strip():
        raise AdapterError("config", "[version_sync.release_wiring].file must be a non-empty string when enabled")
    if not isinstance(fragments, list) or not all(isinstance(item, str) for item in fragments):
        raise AdapterError("config", "[version_sync.release_wiring].required_fragments must be an array of strings")
    workflow_path = repo_relative(repo_root, file_name, "release_wiring.file")
    if not workflow_path.is_file():
        return True, [Finding("release_wiring.file_missing", file_name, "file exists", "absent")]
    text = read_text(workflow_path, repo_root)
    findings = [
        Finding("release_wiring.fragment", file_name, fragment, "absent")
        for fragment in fragments
        if fragment not in text
    ]
    return True, findings


def success_message(workspace_version: str, executed_checks: list[str]) -> str:
    return (
        f"version sync check passed: workspace_version={workspace_version}; "
        + ", ".join(executed_checks)
        + " are aligned."
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check version synchronization in a Cargo/Python workspace.")
    parser.add_argument("--root", type=Path, help="repository root (defaults to discovery from the current directory)")
    parser.add_argument("--config", type=Path, help="explicit config file used for version_sync table resolution")
    parser.add_argument("--json", action="store_true", help="write the sc-lint Python adapter JSON envelope")
    return parser.parse_args(argv[1:])


def _evaluate(root: Path, explicit_config: Path | None) -> tuple[str, list[Finding], list[dict[str, str]], list[str]]:
    config, _config_path = resolve_table("version_sync", explicit_config, root)
    workspace_version = validate_workspace_version(root)
    cargo_lock_enabled, _python_manifests = python_settings(config)
    findings = validate_crate_versions(root, workspace_version)
    skipped: list[dict[str, str]] = []
    executed = ["workspace member versions", "internal path deps"]
    if cargo_lock_enabled:
        lock_findings, lock_skipped = validate_lockfile(root, workspace_version)
        findings.extend(lock_findings)
        skipped.extend(lock_skipped)
        if not lock_skipped:
            executed.append("Cargo.lock")
    python_ran, python_findings = validate_python_manifests(root, workspace_version, config)
    findings.extend(python_findings)
    if python_ran:
        executed.append("python manifests")
    winget_ran, winget_findings = validate_winget_manifests(root, workspace_version, config)
    findings.extend(winget_findings)
    if winget_ran:
        executed.append("winget")
    release_ran, release_findings = validate_release_wiring(root, config)
    findings.extend(release_findings)
    if release_ran:
        executed.append("release wiring")
    findings.sort(key=lambda finding: (finding.check, finding.file))
    return workspace_version, findings, skipped, executed


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        root = discover_repo_root(str(args.root) if args.root is not None else None)
        workspace_version, findings, skipped, executed = _evaluate(root, args.config)
        if args.json:
            ok = not findings
            summary = success_message(workspace_version, executed) if ok else (
                f"version sync failed: {len(findings)} finding(s); workspace_version={workspace_version}"
            )
            write_json(
                success_payload(
                    summary=summary,
                    data={
                        "ok": ok,
                        "findings": [finding.to_dict() for finding in findings],
                        "skipped": skipped,
                        "workspace_version": workspace_version,
                    },
                )
            )
        elif findings:
            for finding in findings:
                print(
                    f"{finding.check} {finding.file}: expected {finding.expected}, actual {finding.actual}",
                    file=sys.stderr,
                )
        else:
            for line in workspace_crate_section_lines(root):
                print(line)
            if skipped:
                for item in skipped:
                    print(f"skipped: {item['file']} ({item['reason']})")
            print(success_message(workspace_version, executed))
        return 1 if findings else 0
    except ConfigError as error:
        return _handle_error(table_missing_error(error), args.json)
    except AdapterError as error:
        return _handle_error(error, args.json)
    except Exception as error:  # The adapter must keep unexpected failures machine-readable.
        wrapped = AdapterError("backend_protocol", f"failed to evaluate version sync: {error}")
        return _handle_error(wrapped, args.json)


def _handle_error(error: AdapterError, as_json: bool) -> int:
    if as_json:
        write_json(error_payload(error))
    else:
        print(error.message, file=sys.stderr)
    return 3 if error.kind == "config" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
