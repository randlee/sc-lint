# sc-lint

Rust lint tooling for repository policy enforcement, AST-sensitive boundary
analysis, and portability checks.

## Workspace

This repository currently ships these primary crates:

- `sc-lint`
  - top-level CLI crate and canonical machine-contract surface
- `sc-lint-directives`
  - shared parsing/types for `#[sc_lint(...)]` directives
- `sc-lint-attributes`
  - proc-macro crate that provides the `#[sc_lint(...)]` attribute namespace
- `sc-lint-schema`
  - shared findings/report schema used across analyzer crates
- `sc-lint-boundary`
  - CLI and library for `syn`-based analysis, findings output, and graph export
- `sc-lint-portability`
  - portability analyzer crate
- `sc-lint-runtime`
  - runtime/concurrency analyzer crate

The workspace version is managed from the root `Cargo.toml` via `version.workspace = true` in each crate.

## Homebrew

The primary supported Homebrew install path is:

```bash
brew install randlee/tap/sc-lint
```

That formula is intended to install the released end-user toolset together:

- `sc-lint`
- `sc-lint-boundary`
- `sc-lint-portability`
- `sc-lint-runtime`

The older `randlee/tap/sc-lint-boundary` formula may remain as a legacy
compatibility surface, but it is not the normal user install path.

## Current Lint Surface

The repo exposes its lint surface through `just lint`.

Default CI-gated checks:

- `fmt`
  - `cargo fmt --all --check`
- `clippy`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- `deny`
  - `cargo-deny` advisories, bans, licenses, and sources checks
- `shear`
  - `cargo-shear` unused-dependency and empty/unlinked file policy checks
- `version`
  - workspace version alignment and internal path dependency version pinning
  - optional release wiring and packaging checks when configured
- `manifests`
  - Cargo manifest policy:
    - required `[workspace.package]` inheritance fields
    - internal path dependency version consistency
- `spell`
  - codespell content checks
- `pytests`
  - Python unit tests for the repo-local lint runner infrastructure

Available but intentionally manual/advisory:

- `modules`
  - `cargo modules dependencies --acyclic`
  - internal module dependency cycle detection per workspace crate
- `sc-boundary`
  - `sc-lint lint sc-boundary`
  - preliminary `syn`-based architectural linting and boundary analysis
- `sc-portability`
  - `sc-lint lint sc-portability`
  - preliminary portability analysis
- `sc-runtime`
  - `sc-lint lint sc-runtime`
  - std runtime/concurrency analysis

Fast local subset:

- `just lint fast`
  - `fmt`
  - `version`
  - `manifests`
  - `spell`
  - `pytests`

## sc-lint-boundary

`sc-lint-boundary` is the main shipped analyzer today.

Current implemented rule families:

- `SCB-CYCLE-001`
  - multi-owner architectural cycles
- `SCB-CYCLE-002`
  - type/method self-loop classification
- `SCB-CYCLE-003`
  - trait-impl self-loop classification
- `SCB-BOUNDARY-001`
  - `boundary.internal_only` visibility violation
- `SCB-BOUNDARY-002`
  - `boundary.internal_only` external reference violation
- `SCB-BOUNDARY-003`
  - `boundary.forbid_external_impls` violation

## sc-lint-portability

`sc-lint-portability` owns the shipped portability rule family.

Current implemented rule families:

- `PORT-001`
  - hardcoded Unix-only absolute paths in test code
- `PORT-002`
  - `dirs::home_dir()` without configured override handling
- `PORT-003`
  - `std::env::set_var()` in test code
- `PORT-004`
  - ungated `std::os::unix` imports in production code
- `PORT-005`
  - `#[cfg_attr(not(unix), allow(dead_code))]` portability suppressors

Supported outputs:

- findings:
  - text
  - JSON
- graph export:
  - JSON
  - Turtle

Example commands:

```bash
just lint
just lint fast
just lint sc-boundary
just lint sc-portability
cargo run -p sc-lint-boundary -- analyze --root . --format text
cargo run -p sc-lint-portability -- analyze --root . --format text
cargo run -p sc-lint-runtime -- analyze --root . --format text
cargo run -p sc-lint-boundary -- export-graph --root . --format turtle
```

## Config

The repo-local lint runner currently uses:

- `.just/lint-config.toml`

Current live config knobs include:

- portability override env configuration
- `cargo-shear` downgrade tables for allowed empty/unlinked files

Startup prompt injection for `team-lead` is configured in:

- `.atm.toml`

## Development

Core local commands:

```bash
just help
just lint
just test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The current GitHub Actions sequence mirrors the local workflow:

- format
- clippy
- `just lint`
- workspace tests

## Docs

Detailed design and planning material lives under:

- [`docs/sc-lint/README.md`](docs/sc-lint/README.md)
- [`crates/sc-lint-boundary/README.md`](crates/sc-lint-boundary/README.md)
- [`crates/sc-lint-portability/README.md`](crates/sc-lint-portability/README.md)
- [`crates/sc-lint-runtime/README.md`](crates/sc-lint-runtime/README.md)
- [`crates/sc-lint-schema/README.md`](crates/sc-lint-schema/README.md)
- [`crates/sc-lint-directives/README.md`](crates/sc-lint-directives/README.md)
- [`crates/sc-lint-attributes/README.md`](crates/sc-lint-attributes/README.md)
- [`docs/sc-lint-boundary/requirements.md`](docs/sc-lint-boundary/requirements.md)
- [`docs/sc-lint-boundary/graph-schema.md`](docs/sc-lint-boundary/graph-schema.md)
- [`docs/sc-lint-boundary/boundary-enforcement-model.md`](docs/sc-lint-boundary/boundary-enforcement-model.md)
- [`docs/sc-lint-boundary/boundary-toml-migration.md`](docs/sc-lint-boundary/boundary-toml-migration.md)
