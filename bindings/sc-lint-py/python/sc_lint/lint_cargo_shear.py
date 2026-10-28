#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
import argparse
import json
from pathlib import Path
from pathlib import PurePosixPath
from pathlib import PureWindowsPath
import re
import shutil
import subprocess
import sys
import tomllib

from sc_lint.lint_common import CONFIG_TABLE_MISSING
from sc_lint.lint_common import ConfigError
from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import resolve_table
from sc_lint.lint_common import workspace_crate_section_lines
from sc_lint.lint_common import workspace_crates
from sc_lint.lint_common import write_console_output
from sc_lint.python_adapter import AdapterError


ERROR_SECTIONS = {"unlinked_files", "empty_files"}
UNUSED_DEPENDENCY_SECTION = "unused_dependency"
SUMMARY_SECTION = "summary"
CARGO_SHEAR_TABLE = "cargo_shear"
CARGO_SHEAR_ALLOWLIST_FIELD = "cargo_shear.allowed_unused_dependencies"
CARGO_SHEAR_CONFIG_ERROR = "SC_LINT_CARGO_SHEAR_CONFIG_INVALID"
PATH_KEY_MIGRATION_DIAGNOSTIC = (
    'Invalid allowed_unused_dependencies key "{key}": path-like keys are no '
    'longer supported; use <package>.<dependency-key> (the dependency rename '
    'alias when present, otherwise its package name), or workspace.<dependency-key> '
    'only for a virtual-root workspace dependency. Package roots use their '
    'package name, including a package named workspace; if both resolve to the '
    'same key, rename the package or dependency alias to disambiguate it.'
)
TRYBUILD_CALL = re.compile(
    r'(?:\.|\bTestCases\s*::)\s*(?:pass|compile_fail)\s*\(\s*'
    r'(?:&\s*(?:mut\s+)?[A-Za-z_][A-Za-z0-9_]*\s*,\s*)?'
    r'(?:r(?P<h>#*)"(?P<raw>.*?)"(?P=h)|"(?P<cooked>[^"\\\n]*)")',
    re.S,
)


@dataclass(frozen=True)
class CargoShearWorkspaceMetadata:
    manifest_packages: dict[str, str]
    allowed_unused_dependency_keys: frozenset[str]
    ambiguous_unused_dependency_keys: frozenset[str] = frozenset()


class CargoShearMetadataError(RuntimeError):
    pass


@dataclass(frozen=True)
class ShearSection:
    name: str
    header: str
    body_lines: tuple[str, ...]
    file_paths: tuple[str, ...]
    has_unrecognized_file_lines: bool = False


@dataclass(frozen=True)
class ShearPolicyFinding:
    section_name: str
    file_path: str
    reason: str


def build_command(repo_root: Path) -> list[str]:
    return ["cargo-shear"]


def parse_sections(stdout: str) -> list[ShearSection]:
    sections: list[ShearSection] = []
    current_name: str | None = None
    current_body: list[str] = []
    current_header = ""

    for line in stdout.splitlines():
        if line.startswith("shear/"):
            if current_name is not None:
                file_paths, has_unrecognized_file_lines = parse_file_paths(current_body)
                sections.append(
                    ShearSection(
                        name=current_name,
                        header=current_header,
                        body_lines=tuple(current_body),
                        file_paths=tuple(file_paths),
                        has_unrecognized_file_lines=has_unrecognized_file_lines,
                    )
                )
            current_name = line.split("/", 1)[1].strip()
            current_header = line.strip()
            current_body = []
            continue
        if current_name is not None:
            current_body.append(line)

    if current_name is not None:
        file_paths, has_unrecognized_file_lines = parse_file_paths(current_body)
        sections.append(
            ShearSection(
                name=current_name,
                header=current_header,
                body_lines=tuple(current_body),
                file_paths=tuple(file_paths),
                has_unrecognized_file_lines=has_unrecognized_file_lines,
            )
        )
    return sections


def parse_file_paths(
    body_lines: list[str] | tuple[str, ...],
) -> tuple[list[str], bool]:
    paths: list[str] = []
    has_unrecognized_file_lines = False
    for line in body_lines:
        stripped = line.strip()
        if not stripped.startswith("│ "):
            continue
        candidate = stripped[2:].strip()
        if not candidate:
            continue
        is_path = "/" in candidate or "\\" in candidate
        is_crate_root_rust_file = Path(candidate).name == candidate and candidate.endswith(".rs")
        if not is_path and not is_crate_root_rust_file:
            has_unrecognized_file_lines = True
            continue
        paths.append(candidate.replace("\\", "/"))
    return paths, has_unrecognized_file_lines


def extract_file_paths(body_lines: list[str] | tuple[str, ...]) -> list[str]:
    paths, _ = parse_file_paths(body_lines)
    return paths


def package_to_crate_dir(repo_root: Path) -> dict[str, str]:
    return {
        crate.package_name: Path(crate.manifest_path).parent.as_posix()
        for crate in workspace_crates(repo_root)
    }


def manifest_to_package_name(repo_root: Path) -> dict[str, str]:
    # workspace_crates returns repo-relative manifest paths; resolve them
    # against repo_root, not the process cwd, so --root works from anywhere.
    resolved_root = repo_root.resolve()
    return {
        (repo_root / crate.manifest_path).resolve().relative_to(resolved_root).as_posix(): crate.package_name
        for crate in workspace_crates(repo_root)
    }


def load_cargo_shear_table(repo_root: Path) -> dict:
    # Raises ConfigError when no searched config file defines [cargo_shear].
    table, _config_path = resolve_table(CARGO_SHEAR_TABLE, None, repo_root)
    return table


def load_policy_config(repo_root: Path) -> dict[str, dict[str, str]]:
    cargo_shear = load_cargo_shear_table(repo_root)

    def table(name: str, *, require_reason: bool = False) -> dict[str, str]:
        value = cargo_shear.get(name, {})
        if not isinstance(value, dict):
            return {}
        normalized: dict[str, str] = {}
        for key, reason in value.items():
            if not isinstance(key, str) or not isinstance(reason, str):
                continue
            if require_reason and not reason.strip():
                continue
            normalized[key.replace("\\", "/")] = reason.strip() if require_reason else reason
        return normalized

    return {
        "allowed_empty_files": table("allowed_empty_files"),
        "allowed_unlinked_files": table("allowed_unlinked_files"),
        "allowed_unused_dependencies": table(
            "allowed_unused_dependencies", require_reason=True
        ),
    }


def configured_unused_dependency_allowlist(
    repo_root: Path,
) -> tuple[bool, object]:
    cargo_shear = load_cargo_shear_table(repo_root)
    if "allowed_unused_dependencies" not in cargo_shear:
        return False, {}
    return True, cargo_shear["allowed_unused_dependencies"]


def load_cargo_shear_workspace_metadata(
    repo_root: Path,
) -> CargoShearWorkspaceMetadata:
    completed = subprocess.run(
        ["cargo", "metadata", "--offline", "--format-version", "1", "--no-deps"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise CargoShearMetadataError(detail or "cargo metadata exited unsuccessfully")

    try:
        metadata = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise CargoShearMetadataError(f"cargo metadata returned invalid JSON: {error}") from error
    packages = metadata.get("packages")
    if not isinstance(packages, list):
        raise CargoShearMetadataError("cargo metadata did not return a package list")

    resolved_root = repo_root.resolve()
    manifest_packages: dict[str, str] = {}
    allowed_keys: set[str] = set()
    key_origins: dict[str, set[str]] = {}
    for package in packages:
        if not isinstance(package, dict):
            continue
        package_name = package.get("name")
        if not isinstance(package_name, str):
            continue

        manifest_path = package.get("manifest_path")
        package_origin = (
            Path(manifest_path).resolve().as_posix()
            if isinstance(manifest_path, str)
            else f"<package:{package.get('id', package_name)}>"
        )
        dependencies = package.get("dependencies", [])
        if isinstance(dependencies, list):
            for dependency in dependencies:
                if not isinstance(dependency, dict):
                    continue
                dependency_name = dependency.get("rename") or dependency.get("name")
                if isinstance(dependency_name, str):
                    key = f"{package_name}.{dependency_name}"
                    allowed_keys.add(key)
                    key_origins.setdefault(key, set()).add(package_origin)

        if not isinstance(manifest_path, str):
            continue
        try:
            relative_manifest = Path(manifest_path).resolve().relative_to(resolved_root).as_posix()
        except ValueError:
            continue
        manifest_packages[relative_manifest] = package_name
        manifest_packages[Path(manifest_path).resolve().as_posix()] = package_name

    root_manifest_path = resolved_root / "Cargo.toml"
    try:
        root_manifest = tomllib.loads(root_manifest_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise CargoShearMetadataError(f"could not read workspace root Cargo.toml: {error}") from error

    # A virtual workspace has no package name of its own. Only that root may
    # use the reserved `workspace.<dependency>` spelling.
    if "package" not in root_manifest:
        workspace = root_manifest.get("workspace", {})
        workspace_dependencies = workspace.get("dependencies", {}) if isinstance(workspace, dict) else {}
        if isinstance(workspace_dependencies, dict):
            for dependency_name in workspace_dependencies:
                if isinstance(dependency_name, str):
                    key = f"workspace.{dependency_name}"
                    allowed_keys.add(key)
                    key_origins.setdefault(key, set()).add("<virtual-workspace-root>")
        manifest_packages["Cargo.toml"] = "workspace"
        manifest_packages[root_manifest_path.as_posix()] = "workspace"

    return CargoShearWorkspaceMetadata(
        manifest_packages=manifest_packages,
        allowed_unused_dependency_keys=frozenset(allowed_keys),
        ambiguous_unused_dependency_keys=frozenset(
            key for key, origins in key_origins.items() if len(origins) > 1
        ),
    )


def is_path_like_dependency_key(key: str) -> bool:
    return (
        "/" in key
        or "\\" in key
        or key.startswith((".", "~"))
        or bool(PureWindowsPath(key).drive)
    )


def validate_unused_dependency_allowlist(
    configured: object,
    metadata: CargoShearWorkspaceMetadata,
) -> list[str]:
    if not isinstance(configured, dict):
        return [
            f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
            'cause=expected-table action="declare a TOML table of canonical dependency keys and reasons"'
        ]

    diagnostics: list[str] = []
    for key, reason in configured.items():
        if not isinstance(key, str):
            continue
        if not isinstance(reason, str):
            diagnostics.append(
                f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
                f'key={key!r} cause=non-string-reason '
                'action="supply a nonblank string reason"'
            )
        elif not reason.strip():
            diagnostics.append(
                f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
                f'key={key!r} cause=blank-reason '
                'action="supply a nonblank string reason"'
            )
        elif is_path_like_dependency_key(key):
            diagnostics.extend(
                (
                    f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
                    f'key={key!r} cause=path-like-key',
                    PATH_KEY_MIGRATION_DIAGNOSTIC.format(key=key),
                )
            )
        elif key in metadata.ambiguous_unused_dependency_keys:
            diagnostics.append(
                f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
                f'key={key!r} cause=ambiguous-dependency-key '
                'action="disambiguate the virtual workspace root from a package named workspace by renaming the package or dependency alias"'
            )
        elif key not in metadata.allowed_unused_dependency_keys:
            diagnostics.append(
                f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
                f'key={key!r} cause=dependency-key-not-found '
                'action="remove the stale key or use <package>.<dependency-key> from Cargo metadata"'
            )
    return diagnostics


def render_metadata_failure(error: Exception) -> str:
    detail = str(error).replace("\n", " ").strip()
    return (
        f'{CARGO_SHEAR_CONFIG_ERROR} field={CARGO_SHEAR_ALLOWLIST_FIELD} '
        f'cause=cargo-metadata-failed detail={detail!r} '
        'action="fix the workspace metadata so cargo metadata --offline --format-version 1 --no-deps succeeds"'
    )


def trybuild_harness_globs(
    package_dir: Path, exclude: frozenset[str]
) -> list[tuple[str, str, int]]:
    matches: list[tuple[str, str, int]] = []
    harnesses = sorted(
        (
            path
            for path in package_dir.rglob("*.rs")
            if path.is_file()
        ),
        key=lambda path: path.relative_to(package_dir).as_posix(),
    )
    for harness in harnesses:
        relative_harness = harness.relative_to(package_dir).as_posix()
        relative_parts = Path(relative_harness).parts
        if "target" in relative_parts or relative_harness in exclude:
            continue
        source = harness.read_text(encoding="utf-8", errors="replace")
        for match in TRYBUILD_CALL.finditer(source):
            glob = match.group("raw")
            if glob is None:
                glob = match.group("cooked")
            if glob is not None:
                line_number = source.count("\n", 0, match.start()) + 1
                matches.append((glob, relative_harness, line_number))
    return matches


def _safe_trybuild_glob(glob: str) -> bool:
    if not glob.strip() or "\\" in glob or "\0" in glob:
        return False
    if PurePosixPath(glob).is_absolute() or PureWindowsPath(glob).drive:
        return False
    return ".." not in PurePosixPath(glob).parts


def trybuild_consumed(
    sections: list[ShearSection], repo_root: Path
) -> dict[tuple[int, str], str]:
    if not any(section.name == "unlinked_files" and section.file_paths for section in sections):
        return {}
    try:
        crate_map = package_to_crate_dir(repo_root)
    except (OSError, tomllib.TOMLDecodeError):
        return {}
    consumed: dict[tuple[int, str], str] = {}
    for section_index, section in enumerate(sections):
        if section.name != "unlinked_files" or not section.file_paths:
            continue
        crate_dir = crate_map.get(infer_package_name(section.body_lines))
        if crate_dir is None:
            continue
        package_dir = repo_root / crate_dir
        reported = set(section.file_paths)
        harness_globs = trybuild_harness_globs(package_dir, frozenset(reported))
        for glob, harness, line_number in harness_globs:
            if not _safe_trybuild_glob(glob):
                continue
            try:
                matches = sorted(package_dir.glob(glob), key=lambda path: path.as_posix())
            except (ValueError, NotImplementedError, OSError):
                continue
            for path in matches:
                if not path.is_file():
                    continue
                relative_path = path.relative_to(package_dir).as_posix()
                key = (section_index, relative_path)
                if relative_path in reported and key not in consumed:
                    consumed[key] = f"{harness}:{line_number}"
    return consumed


def render_consumed_notes(
    sections: list[ShearSection],
    consumed: dict[tuple[int, str], str],
    repo_root: Path,
) -> list[str]:
    if not consumed:
        return []
    crate_map = package_to_crate_dir(repo_root)
    notes: list[str] = []
    for section_index, section in enumerate(sections):
        if section.name != "unlinked_files":
            continue
        package_name = infer_package_name(section.body_lines)
        crate_dir = crate_map.get(package_name, package_name)
        for path in sorted(section.file_paths):
            harness = consumed.get((section_index, path))
            if harness is None:
                continue
            location = f"{path}" if crate_dir in ("", ".") else f"{crate_dir}/{path}"
            notes.append(
                f"shear note: {location} consumed by trybuild harness {harness} [unlinked_files]"
            )
    return notes


def evaluate_policy(
    sections: list[ShearSection],
    policy: dict[str, dict[str, str]],
    consumed: dict[tuple[int, str], str] | None = None,
) -> tuple[list[ShearPolicyFinding], list[str]]:
    findings: list[ShearPolicyFinding] = []
    downgraded: list[str] = []
    consumed = consumed or {}

    for section_index, section in enumerate(sections):
        if section.name not in ERROR_SECTIONS:
            continue
        allowed = (
            policy["allowed_empty_files"]
            if section.name == "empty_files"
            else policy["allowed_unlinked_files"]
        )
        for file_path in section.file_paths:
            if section.name == "unlinked_files" and (section_index, file_path) in consumed:
                continue
            reason = allowed.get(file_path)
            if reason is not None:
                downgraded.append(
                    f"{section.name}: downgraded {file_path} ({reason})"
                )
                continue
            findings.append(
                ShearPolicyFinding(
                    section_name=section.name,
                    file_path=file_path,
                    reason=f"{section.name} must be fixed or explicitly downgraded in .just/lint-config.toml",
                )
            )
    return findings, downgraded


def render_policy_findings(findings: list[ShearPolicyFinding]) -> list[str]:
    rendered: list[str] = []
    for finding in findings:
        rendered.append(
            f"shear policy error: {finding.file_path} [{finding.section_name}] {finding.reason}"
        )
    return rendered


def extract_unused_dependency(
    section: ShearSection, manifest_packages: dict[str, str]
) -> str | None:
    dependency_name: str | None = None
    manifest_path: str | None = None

    for line in section.body_lines:
        stripped = line.strip()
        prefix = "× unused dependency `"
        if stripped.startswith(prefix) and stripped.endswith("`"):
            dependency_name = stripped[len(prefix) : -1]
        if stripped.startswith("╭─[") and stripped.endswith("]"):
            location = stripped[3:-1]
            location_parts = location.rsplit(":", 2)
            if len(location_parts) == 3 and all(part.isdigit() for part in location_parts[1:]):
                manifest_path = location_parts[0].replace("\\", "/")

    if dependency_name is None or manifest_path is None:
        return None
    package_name = manifest_packages.get(manifest_path)
    if package_name is None:
        return None
    return f"{package_name}.{dependency_name}"


def evaluate_unused_dependency_policy(
    sections: list[ShearSection],
    policy: dict[str, dict[str, str]],
    manifest_packages: dict[str, str],
) -> tuple[list[ShearPolicyFinding], list[str], bool]:
    findings: list[ShearPolicyFinding] = []
    downgraded: list[str] = []
    saw_unused_dependency = False

    for section in sections:
        # evaluate_policy owns unlinked/empty files, so they compose here.
        if section.name == SUMMARY_SECTION:
            continue
        if section.name in ERROR_SECTIONS:
            if not section.file_paths or section.has_unrecognized_file_lines:
                return findings, downgraded, False
            continue
        if section.name != UNUSED_DEPENDENCY_SECTION:
            return findings, downgraded, False
        dependency_key = extract_unused_dependency(section, manifest_packages)
        if dependency_key is None:
            return findings, downgraded, False
        saw_unused_dependency = True
        reason = policy["allowed_unused_dependencies"].get(dependency_key)
        if reason is None:
            findings.append(
                ShearPolicyFinding(
                    section_name=section.name,
                    file_path=dependency_key,
                    reason=(
                        "unused dependency must be fixed or explicitly downgraded "
                        "in .just/lint-config.toml"
                    ),
                )
            )
            continue
        downgraded.append(
            f"{section.name}: downgraded {dependency_key} ({reason})"
        )
    return findings, downgraded, saw_unused_dependency


def annotate_sections(sections: list[ShearSection], repo_root: Path) -> list[str]:
    crate_map = package_to_crate_dir(repo_root)
    rendered: list[str] = []
    for section in sections:
        if not section.file_paths:
            continue
        crate_name = infer_package_name(section.body_lines)
        crate_dir = crate_map.get(crate_name, crate_name)
        for path in section.file_paths:
            rendered.append(f"shear note: {crate_dir}/{path} [{section.name}]")
    return rendered


def infer_package_name(body_lines: tuple[str, ...]) -> str:
    for line in body_lines:
        stripped = line.strip()
        if " in `" in stripped and stripped.endswith("`"):
            return stripped.split(" in `", 1)[1][:-1]
    return "unknown-package"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run cargo-shear for dependency pruning checks.")
    parser.add_argument("--root", help="Repo root to inspect.")
    args = parser.parse_args(argv[1:])
    repo_root = discover_repo_root(args.root)

    try:
        allowlist_supplied, configured_allowlist = configured_unused_dependency_allowlist(
            repo_root
        )
    except (ConfigError, AdapterError) as caught:
        if isinstance(caught, ConfigError):
            write_console_output(sys.stderr, f"{CONFIG_TABLE_MISSING}: {caught.message}\n")
        else:
            write_console_output(sys.stderr, f"{caught.message}\n")
        return 3
    metadata: CargoShearWorkspaceMetadata | None = None
    if allowlist_supplied:
        try:
            metadata = load_cargo_shear_workspace_metadata(repo_root)
        except (CargoShearMetadataError, OSError) as error:
            write_console_output(sys.stderr, f"{render_metadata_failure(error)}\n")
            return 2
        diagnostics = validate_unused_dependency_allowlist(configured_allowlist, metadata)
        if diagnostics:
            for diagnostic in diagnostics:
                write_console_output(sys.stderr, f"{diagnostic}\n")
            return 2

    if shutil.which("cargo-shear") is None:
        print("cargo-shear is not installed; install it to run this lint", file=sys.stderr)
        return 2

    for line in workspace_crate_section_lines(repo_root):
        write_console_output(sys.stdout, f"{line}\n")

    completed = subprocess.run(
        build_command(repo_root),
        cwd=repo_root,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )

    stdout = completed.stdout
    sections = parse_sections(stdout)
    consumed = trybuild_consumed(sections, repo_root)
    consumed_notes = render_consumed_notes(sections, consumed, repo_root)
    policy = load_policy_config(repo_root)
    policy_findings, downgraded = evaluate_policy(sections, policy, consumed)

    if completed.returncode == 1:
        unused_findings, unused_downgraded, only_unused_dependencies = (
            evaluate_unused_dependency_policy(
                sections,
                policy,
                metadata.manifest_packages
                if metadata is not None
                # Preserve absent-table parity: without metadata, retain the
                # existing manifest resolver for child-finding package mapping.
                else manifest_to_package_name(repo_root),
            )
        )
        if only_unused_dependencies:
            all_findings = unused_findings + policy_findings
            for line in render_policy_findings(all_findings):
                write_console_output(sys.stdout, f"{line}\n")
            for line in unused_downgraded + downgraded:
                write_console_output(sys.stdout, f"{line}\n")
            for line in consumed_notes:
                write_console_output(sys.stdout, f"{line}\n")
            if stdout:
                write_console_output(sys.stdout, stdout)
            if completed.stderr:
                write_console_output(sys.stderr, completed.stderr)
            return 1 if all_findings else 0

    if completed.returncode != 0:
        if stdout:
            write_console_output(sys.stdout, stdout)
        if completed.stderr:
            write_console_output(sys.stderr, completed.stderr)
        return completed.returncode

    if policy_findings:
        for line in render_policy_findings(policy_findings):
            write_console_output(sys.stdout, f"{line}\n")
        for line in annotate_sections(sections, repo_root):
            if any(f.file_path in line for f in policy_findings):
                write_console_output(sys.stdout, f"{line}\n")
        if downgraded:
            for line in downgraded:
                write_console_output(sys.stdout, f"{line}\n")
        for line in consumed_notes:
            write_console_output(sys.stdout, f"{line}\n")
        if stdout:
            write_console_output(sys.stdout, stdout)
        return 1

    if downgraded:
        for line in downgraded:
            write_console_output(sys.stdout, f"{line}\n")
    for line in consumed_notes:
        write_console_output(sys.stdout, f"{line}\n")
    if stdout:
        write_console_output(sys.stdout, stdout)
    if completed.stderr:
        write_console_output(sys.stderr, completed.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
