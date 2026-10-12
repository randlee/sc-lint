#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
import argparse
from pathlib import Path
import shutil
import subprocess
import sys

from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import load_lint_config
from sc_lint.lint_common import workspace_crate_section_lines
from sc_lint.lint_common import workspace_crates
from sc_lint.lint_common import write_console_output


ERROR_SECTIONS = {"unlinked_files", "empty_files"}
UNUSED_DEPENDENCY_SECTION = "unused_dependency"
SUMMARY_SECTION = "summary"


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


def load_policy_config(repo_root: Path) -> dict[str, dict[str, str]]:
    config = load_lint_config(repo_root)
    cargo_shear = config.get("cargo_shear", {})
    if not isinstance(cargo_shear, dict):
        return {
            "allowed_empty_files": {},
            "allowed_unlinked_files": {},
            "allowed_unused_dependencies": {},
        }

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


def evaluate_policy(sections: list[ShearSection], policy: dict[str, dict[str, str]]) -> tuple[list[ShearPolicyFinding], list[str]]:
    findings: list[ShearPolicyFinding] = []
    downgraded: list[str] = []

    for section in sections:
        if section.name not in ERROR_SECTIONS:
            continue
        allowed = (
            policy["allowed_empty_files"]
            if section.name == "empty_files"
            else policy["allowed_unlinked_files"]
        )
        for file_path in section.file_paths:
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
    policy = load_policy_config(repo_root)
    policy_findings, downgraded = evaluate_policy(sections, policy)

    if completed.returncode == 1:
        unused_findings, unused_downgraded, only_unused_dependencies = (
            evaluate_unused_dependency_policy(
                sections,
                policy,
                manifest_to_package_name(repo_root),
            )
        )
        if only_unused_dependencies:
            all_findings = unused_findings + policy_findings
            for line in render_policy_findings(all_findings):
                write_console_output(sys.stdout, f"{line}\n")
            for line in unused_downgraded + downgraded:
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
        if stdout:
            write_console_output(sys.stdout, stdout)
        return 1

    if downgraded:
        for line in downgraded:
            write_console_output(sys.stdout, f"{line}\n")
    if stdout:
        write_console_output(sys.stdout, stdout)
    if completed.stderr:
        write_console_output(sys.stderr, completed.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
