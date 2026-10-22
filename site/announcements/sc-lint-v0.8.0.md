# sc-lint v0.8.0 — Boundary Policies, Consumer Profiles, One Portable Bootstrap

**Released:** October 19, 2026 · **Install:** `brew install randlee/tap/sc-lint` (Homebrew), `pip install sc-lint` (PyPI), `cargo install sc-lint` (crates.io), or the platform archives on the [release page](https://github.com/randlee/sc-lint/releases/tag/v0.8.0)

[Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-lint/releases/tag/v0.8.0) · [Operator manual](https://github.com/randlee/sc-lint/blob/main/docs-bundle/README.md)

---

## Architecture-Boundary Enforcer

**As a maintainer, I want sc-lint to enforce architecture boundaries for agents, so that agent-authored changes stay inside the intended module/crate boundaries instead of leaking across layers.**

v0.8.0 makes boundaries express policy instead of only adjacency. The dependency families now accept a single trailing `*` prefix pattern — `"codex-*"` — so a crate can allow a family without enumerating every member, and the new `SCB-FEATURE-001` rule enforces feature independence through an optional `allowed_feature_dependencies` field. Boundary also gains three optional manifest policy families: `manifest_dependency_allowlists`, `global_dependency_ownership`, and `manifest_section_rules`, with semantic inventory validation and stale dependent-permission reporting. All of it is inert until declared.

Two escape hatches arrive alongside. `exclude_paths_by_rule` maps exact rule IDs to workspace-relative globs and drops only that rule's findings at matching paths, and portability `platform_adapters` declares explicit Unix/Windows module pairs — with a review reason — for equivalent implementations whose names differ.

Compatibility is a deliberate gate: existing boundary records keep their results, and feature independence is opt-in, but older binaries reject records declaring `allowed_feature_dependencies`, so consumers must pin a release that understands the field. Tightened inventory validation can surface new `SCB-DEPENDENCY-004`, `SCB-BOUNDARY-004`, and `SCB-MANIFEST-006` findings in declarations that previously passed; the recovery steps are documented before you upgrade. Path-like cargo-shear `allowed_unused_dependencies` keys are now rejected with `SC_LINT_CARGO_SHEAR_CONFIG_INVALID` — use the canonical `"{package}.{dependency_key}"` form, or `workspace.{dependency_key}` for virtual-root workspace dependencies.

---

## Agent Orchestrator

**As an orchestrator, I want sc-lint to keep agent-authored code conformant to shared conventions, so that parallel agents' output stays consistent without per-agent review.**

Fleet uniformity is now selectable rather than assumed. Consumer named lint profiles (`[tool.sc-lint.lint_profiles]`) and `--platform-scope` (`host` by default, or `all`, `linux`, `macos`, `windows`) choose which declared lint steps run, so one repository can declare a fast agent lane and a full cross-platform lane from the same configuration, and the adoption kit renders platform-aware profiles and CI resolution.

Adoption itself is now one implementation. A single Python bootstrap, `.sc-lint/bootstrap.py`, is identical on Linux, macOS and Windows; recipes run `<python> .sc-lint/bootstrap.py <operation> --config sc-lint.toml` (`python` on Windows, `python3` elsewhere), and on Windows `just` invokes recipes through `cmd.exe /c`. pwsh is no longer used by recipes, actions or tests (ADR-012, ADR-016). `.sc-lint/bootstrap` and `.sc-lint/bootstrap.ps1` are no longer shipped: upgrading removes them when their digest matches the kit's install manifest, reports a modified copy as a conflict (exit 2) and leaves it in place. The bootstrap requires Python 3.11 or newer.

Provisioning is now declarative. `just setup` installs only the cargo tools (exact versions) and rustup targets a consumer declares under `tools` in `.sc-lint/install.json` — no product default chooses tools, and lint never installs them. `.sc-lint/bootstrap.py exec -- <command>` runs a command inside the managed product environment, preserving arguments and exit status, without running setup or selecting a profile.

---

## Agent Team Member

**As an agent on an SC team, I want excellent, actionable error messages, so that I can self-correct my code without a human pointing at the mistake.**

The built-in source lint plan now runs independent steps concurrently and continues through all planned steps after a failure, reporting every failure by plan index in plan order — one run shows every problem instead of the first. Consumer-configured lint and test profiles stay sequential, declaration-ordered and fail-fast. Portability and runtime analysis now error when a finding's source path cannot be made repository-relative, including paths outside the repository root, instead of silently skipping that finding; the boundary inventory likewise rejects nested `planning.toml` files rather than ignoring planning metadata outside `boundaries/planning.toml`. The cargo-deny step retries a DNS resolution failure up to three times before reporting it.

---

## What's Next

Boundary policy, consumer profiles and the unified bootstrap land together at 0.8.0. The SC fleet adopts the release as the single linter across every repository; consumers that add `allowed_feature_dependencies` should pin 0.8.0 or newer before rolling the record out.
