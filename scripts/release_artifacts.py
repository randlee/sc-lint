#!/usr/bin/env python3
"""Release artifact manifest utilities for the sc-lint release surface."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from textwrap import dedent

PREFLIGHT_FULL = "full"
PREFLIGHT_LOCKED = "locked"
HOMEBREW_PRIMARY_FORMULA = "sc-lint"
HOMEBREW_LEGACY_BOUNDARY_FORMULA = "sc-lint-boundary"
HOMEBREW_TARGETS = (
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
)


def load_manifest(path: Path) -> dict:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        raise SystemExit("unsupported manifest schema_version")

    crates = data.get("crates")
    if not isinstance(crates, list) or not crates:
        raise SystemExit("manifest must define non-empty [[crates]]")
    binaries = data.get("release_binaries")
    if not isinstance(binaries, list) or not binaries:
        raise SystemExit("manifest must define non-empty [[release_binaries]]")

    required = {
        "artifact",
        "package",
        "cargo_toml",
        "required",
        "publish",
        "publish_order",
        "preflight_check",
        "wait_after_publish_seconds",
        "verify_install",
    }
    seen_artifacts: set[str] = set()
    seen_packages: set[str] = set()
    for idx, crate in enumerate(crates):
        if not isinstance(crate, dict):
            raise SystemExit(f"crates[{idx}] must be a table")
        missing = sorted(required - set(crate))
        if missing:
            raise SystemExit(f"crates[{idx}] missing fields: {', '.join(missing)}")
        artifact = require_str(crate, "artifact", f"crates[{idx}]")
        package = require_str(crate, "package", f"crates[{idx}]")
        require_str(crate, "cargo_toml", f"crates[{idx}]")
        mode = require_str(crate, "preflight_check", f"crates[{idx}]")
        if mode not in {PREFLIGHT_FULL, PREFLIGHT_LOCKED}:
            raise SystemExit(f"{artifact}: invalid preflight_check {mode!r}")
        if artifact in seen_artifacts:
            raise SystemExit(f"duplicate artifact {artifact}")
        if package in seen_packages:
            raise SystemExit(f"duplicate package {package}")
        seen_artifacts.add(artifact)
        seen_packages.add(package)

    seen_bins: set[str] = set()
    for idx, entry in enumerate(binaries):
        if not isinstance(entry, dict):
            raise SystemExit(f"release_binaries[{idx}] must be a table")
        name = require_str(entry, "name", f"release_binaries[{idx}]")
        if name in seen_bins:
            raise SystemExit(f"duplicate release binary {name}")
        seen_bins.add(name)

    crates.sort(key=lambda item: (item["publish_order"], item["artifact"]))
    return {"crates": crates, "release_binaries": binaries}


def require_str(obj: dict, key: str, label: str) -> str:
    value = obj.get(key)
    if not isinstance(value, str) or not value.strip():
        raise SystemExit(f"{label}.{key} must be a non-empty string")
    return value


def release_binary_names(manifest: dict) -> list[str]:
    names = []
    for idx, entry in enumerate(manifest["release_binaries"]):
        names.append(require_str(entry, "name", f"release_binaries[{idx}]"))
    return names


def primary_release_binary(manifest: dict) -> str:
    names = release_binary_names(manifest)
    if HOMEBREW_PRIMARY_FORMULA not in names:
        raise SystemExit(
            f"release_binaries must include {HOMEBREW_PRIMARY_FORMULA!r} for Homebrew packaging"
        )
    return HOMEBREW_PRIMARY_FORMULA


def release_archive_name(manifest: dict, version: str, target: str, extension: str) -> str:
    return f"{primary_release_binary(manifest)}_{version}_{target}.{extension}"


def formula_class_name(formula_name: str) -> str:
    return "".join(part.capitalize() for part in formula_name.split("-"))


def homebrew_sha_map(args: argparse.Namespace) -> dict[str, str]:
    mapping = {
        "x86_64-apple-darwin": args.sha256_x86_64_apple_darwin,
        "aarch64-apple-darwin": args.sha256_aarch64_apple_darwin,
        "x86_64-unknown-linux-gnu": args.sha256_x86_64_unknown_linux_gnu,
    }
    for target in HOMEBREW_TARGETS:
        sha = mapping[target]
        if not isinstance(sha, str) or len(sha) != 64:
            raise SystemExit(f"{target} sha256 must be a 64-character hex string")
        if any(ch not in "0123456789abcdefABCDEF" for ch in sha):
            raise SystemExit(f"{target} sha256 must contain only hex characters")
    return mapping


def install_block(binary_names: list[str], indent: str) -> str:
    lines = [f'{indent}def install']
    for binary in binary_names:
        lines.append(f'{indent}  bin.install "{binary}"')
    lines.append(f"{indent}end")
    return "\n".join(lines)


def test_block(binary_names: list[str]) -> str:
    lines = ["  test do"]
    for binary in binary_names:
        lines.append(f'    system "#{{bin}}/{binary}", "--version"')
    lines.append("  end")
    return "\n".join(lines)


def render_homebrew_formula_text(
    manifest: dict,
    *,
    formula_name: str,
    version: str,
    tag: str,
    sha_map: dict[str, str],
) -> str:
    if formula_name == HOMEBREW_PRIMARY_FORMULA:
        desc = "Top-level sc-lint CLI and analyzer toolset for Rust workspaces"
        installed_binaries = release_binary_names(manifest)
    elif formula_name == HOMEBREW_LEGACY_BOUNDARY_FORMULA:
        desc = "Legacy compatibility formula for the sc-lint boundary analyzer"
        if HOMEBREW_LEGACY_BOUNDARY_FORMULA not in release_binary_names(manifest):
            raise SystemExit(
                f"release_binaries must include {HOMEBREW_LEGACY_BOUNDARY_FORMULA!r} to render its legacy formula"
            )
        installed_binaries = [HOMEBREW_LEGACY_BOUNDARY_FORMULA]
    else:
        raise SystemExit(f"unsupported formula {formula_name!r}")

    archive_prefix = primary_release_binary(manifest)
    class_name = formula_class_name(formula_name)
    macos_intel_url = (
        f"https://github.com/randlee/sc-lint/releases/download/{tag}/"
        f"{archive_prefix}_{version}_x86_64-apple-darwin.tar.gz"
    )
    macos_arm_url = (
        f"https://github.com/randlee/sc-lint/releases/download/{tag}/"
        f"{archive_prefix}_{version}_aarch64-apple-darwin.tar.gz"
    )
    linux_intel_url = (
        f"https://github.com/randlee/sc-lint/releases/download/{tag}/"
        f"{archive_prefix}_{version}_x86_64-unknown-linux-gnu.tar.gz"
    )

    formula = dedent(
        f"""\
        # typed: false
        # frozen_string_literal: true

        class {class_name} < Formula
          desc "{desc}"
          homepage "https://github.com/randlee/sc-lint"
          version "{version}"
          license "MIT"

          on_macos do
            on_intel do
              url "{macos_intel_url}"
              sha256 "{sha_map['x86_64-apple-darwin']}"

        {install_block(installed_binaries, "      ")}
            end
            on_arm do
              url "{macos_arm_url}"
              sha256 "{sha_map['aarch64-apple-darwin']}"

        {install_block(installed_binaries, "      ")}
            end
          end

          on_linux do
            on_intel do
              if Hardware::CPU.is_64_bit?
                url "{linux_intel_url}"
                sha256 "{sha_map['x86_64-unknown-linux-gnu']}"

        {install_block(installed_binaries, "        ")}
              end
            end
          end

        {test_block(installed_binaries)}
        end
        """
    )
    return formula


def cargo_search_version_exists(crate: str, version: str) -> bool:
    result = subprocess.run(
        ["cargo", "search", crate, "--limit", "1"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    return f'{crate} = "{version}"' in result.stdout


def emit_inventory(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    generated_at = args.generated_at or datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    items = []
    for crate in manifest["crates"]:
        if not crate["publish"]:
            continue
        verify = [f'cargo search {crate["package"]} --limit 1 | grep -F \'{crate["package"]} = "{args.version}"\'']
        if crate["verify_install"]:
            verify.append(f"cargo install {crate['package']} --version {args.version} --locked --force")
        items.append(
            {
                "artifact": crate["artifact"],
                "version": args.version,
                "sourceRef": args.source_ref,
                "publishTarget": "crates.io",
                "required": crate["required"],
                "publish": crate["publish"],
                "verifyCommands": verify,
            }
        )
    items.sort(key=lambda item: item["artifact"])
    payload = {
        "releaseVersion": args.version,
        "releaseTag": args.tag,
        "releaseCommit": args.commit,
        "generatedAt": generated_at,
        "items": items,
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return 0


def list_cargo_tomls(args: argparse.Namespace) -> int:
    for crate in load_manifest(Path(args.manifest))["crates"]:
        print(crate["cargo_toml"])
    return 0


def list_artifacts(args: argparse.Namespace) -> int:
    for crate in load_manifest(Path(args.manifest))["crates"]:
        if args.publishable_only and not crate["publish"]:
            continue
        print(crate["artifact"])
    return 0


def list_preflight(args: argparse.Namespace) -> int:
    for crate in load_manifest(Path(args.manifest))["crates"]:
        if crate["publish"] and crate["preflight_check"] == args.mode:
            print(crate["package"])
    return 0


def list_publish_plan(args: argparse.Namespace) -> int:
    crates = [crate for crate in load_manifest(Path(args.manifest))["crates"] if crate["publish"]]
    for crate in crates:
        print(f'{crate["package"]}|{crate["wait_after_publish_seconds"]}')
    return 0


def list_release_binaries(args: argparse.Namespace) -> int:
    for name in release_binary_names(load_manifest(Path(args.manifest))):
        print(name)
    return 0


def cargo_build_bin_args(args: argparse.Namespace) -> int:
    print(" ".join(f"--bin {name}" for name in release_binary_names(load_manifest(Path(args.manifest)))))
    return 0


def print_primary_release_binary(args: argparse.Namespace) -> int:
    print(primary_release_binary(load_manifest(Path(args.manifest))))
    return 0


def render_homebrew_formula(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        render_homebrew_formula_text(
            manifest,
            formula_name=args.formula,
            version=args.version,
            tag=args.tag,
            sha_map=homebrew_sha_map(args),
        ),
        encoding="utf-8",
    )
    return 0


def check_version_unpublished(args: argparse.Namespace) -> int:
    published = []
    for crate in load_manifest(Path(args.manifest))["crates"]:
        if crate["publish"] and cargo_search_version_exists(crate["package"], args.version):
            published.append(crate["artifact"])
    if published:
        raise SystemExit("release version already published for: " + ", ".join(sorted(published)))
    print(f"ok: no publishable artifacts found at version {args.version}")
    return 0


def workspace_members(workspace_toml: Path) -> list[str]:
    data = tomllib.loads(workspace_toml.read_text(encoding="utf-8"))
    members = data.get("workspace", {}).get("members", [])
    if not isinstance(members, list):
        raise SystemExit("Cargo.toml [workspace].members must be a list")
    return members


def crate_name(crate_toml: Path) -> str | None:
    data = tomllib.loads(crate_toml.read_text(encoding="utf-8"))
    return data.get("package", {}).get("name")


def crate_is_publishable(crate_toml: Path) -> bool:
    data = tomllib.loads(crate_toml.read_text(encoding="utf-8"))
    publish = data.get("package", {}).get("publish")
    if publish is False:
        return False
    if isinstance(publish, list) and len(publish) == 0:
        return False
    return True


def validate_manifest(args: argparse.Namespace) -> int:
    manifest_packages = {crate["package"] for crate in load_manifest(Path(args.manifest))["crates"]}
    workspace_toml = Path(args.workspace_toml)
    workspace_root = workspace_toml.parent
    missing = []
    for member in workspace_members(workspace_toml):
        crate_toml = workspace_root / member / "Cargo.toml"
        if not crate_toml.exists() or not crate_is_publishable(crate_toml):
            continue
        name = crate_name(crate_toml)
        if name and name not in manifest_packages:
            missing.append(name)
            print(f"MISSING: {name}")
    if missing:
        print(f"\n{len(missing)} publishable crate(s) missing from manifest.", file=sys.stderr)
        return 1
    print("ok: all publishable workspace crates are present in the manifest")
    return 0


def has_workspace_path_deps(crate_toml: Path, workspace_root: Path) -> list[str]:
    data = tomllib.loads(crate_toml.read_text(encoding="utf-8"))
    ws_toml = workspace_root / "Cargo.toml"
    ws_data = tomllib.loads(ws_toml.read_text(encoding="utf-8")) if ws_toml.exists() else {}
    workspace_deps = ws_data.get("workspace", {}).get("dependencies", {})
    crate_dir = crate_toml.parent
    deps: list[str] = []

    def check_table(table: object) -> None:
        if not isinstance(table, dict):
            return
        for dep_name, dep_spec in table.items():
            if isinstance(dep_spec, dict):
                if dep_spec.get("workspace") is True:
                    ws_dep = workspace_deps.get(dep_name, {})
                    if isinstance(ws_dep, dict) and "path" in ws_dep:
                        deps.append(dep_name)
                elif "path" in dep_spec:
                    dep_path = (crate_dir / dep_spec["path"]).resolve()
                    if dep_path.is_relative_to(workspace_root.resolve()):
                        deps.append(dep_name)

    check_table(data.get("dependencies", {}))
    check_table(data.get("build-dependencies", {}))
    for target_data in data.get("target", {}).values():
        if isinstance(target_data, dict):
            check_table(target_data.get("dependencies", {}))
            check_table(target_data.get("build-dependencies", {}))
    return sorted(set(deps))


def validate_preflight_checks(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    workspace_root = Path(args.workspace_toml).parent
    errors = []
    for crate in manifest["crates"]:
        if crate["preflight_check"] != PREFLIGHT_FULL:
            continue
        crate_toml = workspace_root / crate["cargo_toml"]
        path_deps = has_workspace_path_deps(crate_toml, workspace_root)
        if path_deps:
            errors.append(
                f"{crate['artifact']} has workspace path deps ({', '.join(path_deps)}) but preflight_check='full'"
            )
    if errors:
        for error in errors:
            print(error)
        return 1
    print("ok: all preflight_check='full' crates are genuine leaf crates")
    return 0


def workspace_package_map(workspace_toml: Path) -> dict[str, Path]:
    root = workspace_toml.parent
    mapping = {}
    for member in workspace_members(workspace_toml):
        crate_toml = root / member / "Cargo.toml"
        if crate_toml.exists():
            name = crate_name(crate_toml)
            if name:
                mapping[name] = crate_toml
    return mapping


def workspace_dependency_names(crate_toml: Path, workspace_root: Path) -> set[str]:
    data = tomllib.loads(crate_toml.read_text(encoding="utf-8"))
    ws_toml = workspace_root / "Cargo.toml"
    ws_data = tomllib.loads(ws_toml.read_text(encoding="utf-8")) if ws_toml.exists() else {}
    workspace_deps = ws_data.get("workspace", {}).get("dependencies", {})
    workspace_packages = set(workspace_package_map(ws_toml).keys()) if ws_toml.exists() else set()
    crate_dir = crate_toml.parent
    deps: set[str] = set()

    def resolve(dep_name: str, dep_spec: object) -> str | None:
        if isinstance(dep_spec, str):
            return dep_name if dep_name in workspace_packages else None
        if not isinstance(dep_spec, dict):
            return None
        if dep_spec.get("workspace") is True:
            ws_dep = workspace_deps.get(dep_name, {})
            if isinstance(ws_dep, dict):
                package_name = ws_dep.get("package", dep_name)
                if "path" in ws_dep or package_name in workspace_packages:
                    return package_name
            return dep_name if dep_name in workspace_packages else None
        package_name = dep_spec.get("package", dep_name)
        if "path" in dep_spec:
            dep_path = (crate_dir / dep_spec["path"]).resolve()
            if dep_path.is_relative_to(workspace_root.resolve()):
                return package_name
        return package_name if package_name in workspace_packages else None

    def collect(table: object) -> None:
        if not isinstance(table, dict):
            return
        for dep_name, dep_spec in table.items():
            package_name = resolve(dep_name, dep_spec)
            if package_name:
                deps.add(package_name)

    collect(data.get("dependencies", {}))
    collect(data.get("build-dependencies", {}))
    for target_data in data.get("target", {}).values():
        if isinstance(target_data, dict):
            collect(target_data.get("dependencies", {}))
            collect(target_data.get("build-dependencies", {}))
    return deps


def validate_publish_order(args: argparse.Namespace) -> int:
    manifest = load_manifest(Path(args.manifest))
    workspace_root = Path(args.workspace_toml).parent
    publishable = [crate for crate in manifest["crates"] if crate["publish"]]
    order = {crate["package"]: crate["publish_order"] for crate in publishable}
    violations = []
    for crate in publishable:
        crate_toml = workspace_root / crate["cargo_toml"]
        for dep_package in sorted(workspace_dependency_names(crate_toml, workspace_root)):
            if dep_package in order and order[crate["package"]] <= order[dep_package]:
                violations.append(
                    f"{crate['package']} (publish_order={order[crate['package']]}) depends on "
                    f"{dep_package} (publish_order={order[dep_package]})"
                )
    if violations:
        print("publish_order violation(s):")
        for violation in violations:
            print(f"  - {violation}")
        return 1
    print("ok: publish_order matches the workspace dependency graph")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Release artifact manifest utilities")
    subparsers = parser.add_subparsers(dest="command", required=True)

    emit = subparsers.add_parser("emit-inventory")
    emit.add_argument("--manifest", required=True)
    emit.add_argument("--version", required=True)
    emit.add_argument("--tag", required=True)
    emit.add_argument("--commit", required=True)
    emit.add_argument("--source-ref", required=True)
    emit.add_argument("--generated-at")
    emit.add_argument("--output", required=True)
    emit.set_defaults(func=emit_inventory)

    list_tomls = subparsers.add_parser("list-cargo-tomls")
    list_tomls.add_argument("--manifest", required=True)
    list_tomls.set_defaults(func=list_cargo_tomls)

    list_items = subparsers.add_parser("list-artifacts")
    list_items.add_argument("--manifest", required=True)
    list_items.add_argument("--publishable-only", action="store_true")
    list_items.set_defaults(func=list_artifacts)

    list_pre = subparsers.add_parser("list-preflight")
    list_pre.add_argument("--manifest", required=True)
    list_pre.add_argument("--mode", required=True, choices=[PREFLIGHT_FULL, PREFLIGHT_LOCKED])
    list_pre.set_defaults(func=list_preflight)

    list_plan = subparsers.add_parser("list-publish-plan")
    list_plan.add_argument("--manifest", required=True)
    list_plan.set_defaults(func=list_publish_plan)

    list_bins = subparsers.add_parser("list-release-binaries")
    list_bins.add_argument("--manifest", required=True)
    list_bins.set_defaults(func=list_release_binaries)

    build_bins = subparsers.add_parser("cargo-build-bin-args")
    build_bins.add_argument("--manifest", required=True)
    build_bins.set_defaults(func=cargo_build_bin_args)

    primary_bin = subparsers.add_parser("primary-release-binary")
    primary_bin.add_argument("--manifest", required=True)
    primary_bin.set_defaults(func=print_primary_release_binary)

    render_formula = subparsers.add_parser("render-homebrew-formula")
    render_formula.add_argument("--manifest", required=True)
    render_formula.add_argument(
        "--formula",
        required=True,
        choices=[HOMEBREW_PRIMARY_FORMULA, HOMEBREW_LEGACY_BOUNDARY_FORMULA],
    )
    render_formula.add_argument("--version", required=True)
    render_formula.add_argument("--tag", required=True)
    render_formula.add_argument("--output", required=True)
    render_formula.add_argument("--sha256-x86_64-apple-darwin", required=True)
    render_formula.add_argument("--sha256-aarch64-apple-darwin", required=True)
    render_formula.add_argument("--sha256-x86_64-unknown-linux-gnu", required=True)
    render_formula.set_defaults(func=render_homebrew_formula)

    unpublished = subparsers.add_parser("check-version-unpublished")
    unpublished.add_argument("--manifest", required=True)
    unpublished.add_argument("--version", required=True)
    unpublished.set_defaults(func=check_version_unpublished)

    validate_m = subparsers.add_parser("validate-manifest")
    validate_m.add_argument("--manifest", required=True)
    validate_m.add_argument("--workspace-toml", required=True)
    validate_m.set_defaults(func=validate_manifest)

    validate_p = subparsers.add_parser("validate-preflight-checks")
    validate_p.add_argument("--manifest", required=True)
    validate_p.add_argument("--workspace-toml", required=True)
    validate_p.set_defaults(func=validate_preflight_checks)

    validate_o = subparsers.add_parser("validate-publish-order")
    validate_o.add_argument("--manifest", required=True)
    validate_o.add_argument("--workspace-toml", required=True)
    validate_o.set_defaults(func=validate_publish_order)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
