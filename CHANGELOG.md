# Changelog

All notable changes to sc-lint are documented here.

## [0.5.0] — 2026-08-12

### Features

- **Compatibility contract and recovery** (E.1) — consumer repositories declare
  a SemVer minimum in `sc-lint.toml`; `sc-lint` exposes a structured version
  probe and compatibility preflight with documented recovery diagnostics.
- **Safe install and upgrade engine** (E.2) — checksum-verified release
  installation, atomic replacement, rollback, and idempotent setup/upgrade
  operations let consumers recover missing or old installations.
- **Consumer CLI and Just integration** (E.3) — `sc-lint init --just` creates
  the product-owned bootstrap and canonical `just setup`, `just lint`,
  `just test`, and `just upgrade` interface.
- **Distributed operator documentation** (E.4/E.5) — the installed release
  includes an offline documentation bundle, package guides, help discovery,
  and release/Homebrew distribution support.
- **Reusable GitHub Action** (E.6) — a versioned Action installs verified
  release artifacts and runs the consumer contract in CI.
- **Dogfood consumer contract fixtures** (E.7) — the root repository is the
  executable model, with cross-platform release-binary fixtures covering
  setup, lint, test, docs discovery, and safe upgrade migration.
- **New shared analyzer support crate** — `sc-lint-analyzer-support` is
  published for the first time in this release, providing the shared
  `source_scan.rs` and `render.rs` implementation extracted to close the
  Phase E `ARCH-001` duplication finding.

### Changes

- **Consumer adoption boundary** — `just` is the memorable consumer interface;
  installed `sc-lint` owns consumer behavior, diagnostics, and documentation,
  while `cargo run` remains source-checkout development machinery.
- **Release and CI delivery** — Phase E validates consumer delivery across
  Ubuntu, macOS, and Windows, including archive, Homebrew, and GitHub Action
  distribution paths where supported.

### Version

Workspace bumped from `0.4.0` → `0.5.0`. `[workspace.package].version`, the
four pinned internal `[workspace.dependencies]` versions (`sc-lint-boundary`,
`sc-lint-directives`, `sc-lint-schema`, and `sc-lint-analyzer-support`), and `Cargo.lock`
are all synchronized at `0.5.0`.

---

## [0.4.0] — 2026-07-02

### Features

- **Named-caller allowlist enforcement** (`SCB-CALLER-001`) — `BoundaryRecord` now accepts a `CallersSection` with `ApprovedCaller` entries; `analyze_named_callers` and `caller_is_exempt` enforce the allowlist at analysis time
- **Observability boundary policy** (`ADR-009`) — `emit_*` wrapper functions are forbidden in non-CLI crates; `CLI-LAYER-OWNS-LOGGER-INITIALIZATION` boundary rule enforced across the workspace
- **Portability scope and parity** (`ADR-010`) — `sc-lint-portability` designated shared owner for Windows-path, env-var, and shell portability lint rules
- **Full Homebrew toolset** — `sc-lint.rb` primary formula installs all four binaries (`sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, `sc-lint-runtime`); legacy `sc-lint-boundary.rb` maintained for tap compatibility
- **QA process hardening** — triage-first dispatch flow; `scripts/find_todos.py` and `scripts/triage_carry_forward.py` added; oxigraph subprocess mocked in tests
- **Package-level dependency policy enforcement** (`SCB-DEPENDENCY-001/002/003`, closes #73) — boundary TOML `[dependencies]` sections (`allowed_dependencies`, `allowed_dependents`, `forbidden_edges`) are now parsed into validated types (`RawDependenciesSection`, `ForbiddenPackageEdge`, `PackageDependencyPolicy`) at inventory load and mechanically enforced against direct workspace package edges via a new `sc-lint-boundary::package_policy` analyzer and `RuleFilter::Dependencies`; malformed policy records fail inventory loading with actionable `CLI.CONFIG_ERROR` diagnostics

### Changes

- **Security dependency update** — bumped `anyhow` from `1.0.102` to
  `1.0.103` to clear `RUSTSEC-2026-0190` in the `cargo-deny` advisories gate
- **Workspace version inheritance** — all seven crates converted from `version = "..."` to `version.workspace = true`; `[workspace.package] version` is the single source of truth; the three internal `[workspace.dependencies]` path-dependency version pins (`sc-lint-boundary`, `sc-lint-directives`, `sc-lint-schema`) are kept in sync by hand alongside it
- **Standalone binary dispatch** — `run_delegated_backend()` now resolves tool binaries as a sibling to `current_exe()` with PATH fallback; removes build-time `env!("CARGO_MANIFEST_DIR")` dependency that prevented Homebrew installs from running `sc-lint lint sc-portability` and `sc-lint lint sc-runtime`
- **Release gate version check** — `scripts/release_gate.sh` now verifies the release version input matches `[workspace.package].version` in `Cargo.toml` via `tomllib`; `gate-and-tag` CI job pins Python 3.11 via `setup-python`
- **Version consistency regression test** — `crates/sc-lint/src/tests.rs` now asserts every workspace member crate's resolved version matches `[workspace.package].version`, catching an accidental per-crate version override before publish

### Version

Workspace bumped from `0.3.1` (unreleased, last tagged release `v0.3.0`) → `0.4.0`. `[workspace.package].version`, the three pinned internal `[workspace.dependencies]` versions, and `Cargo.lock` are all synchronized at `0.4.0`.

---

## [0.1.0] — 2026-05-11

Initial release.

### Features

- **`sc-lint` CLI** — top-level composition root; `lint` subcommand dispatches to backend analyzers; structured JSON output via `sc-lint-schema`
- **`sc-lint-boundary`** — AST-sensitive boundary analyzer; loads `boundary.toml` inventory; carries dependency-policy metadata such as `forbidden_edges` for later enforcement; `emit_*` rules and `banned_imports`; named-caller groundwork
- **`sc-lint-portability`** — AST-sensitive portability analyzer; Windows-path, env-var, and shell-portability lint rules
- **`sc-lint-runtime`** — AST-sensitive runtime/concurrency analyzer
- **`sc-lint-schema`** — shared machine output schema types used by all analyzer crates
- **`sc-lint-attributes`** — proc-macro crate for source-level declarative lint suppression attributes
- **`sc-lint-directives`** — shared directive parser for proc-macro and analyzer crates
- **Homebrew tap** — `update-homebrew` CI job rewrites `sc-lint.rb` and `sc-lint-boundary.rb` after each release
- **Release workflow** — cross-platform binary builds (macOS intel/arm, Linux x86\_64, Windows); `cargo publish` in dependency order; `publish-artifacts.toml` manifest drives all release steps
