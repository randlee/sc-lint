---
id: C.7
title: Broad Environment-Variable Portability
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.7 — Broad Environment-Variable Portability

## Goal

- add one shared production-scope environment-variable portability family to
  `sc-lint-portability`
- catch Unix-centric home/user/config lookups that bypass platform-neutral
  abstractions
- keep the new production env family distinct from the existing home-dir
  wrapper and test-only `set_var` rules

## Hard Dependencies

- GitHub issue `#54` — broad env-var portability gap
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./adr/ADR-010-portability-scope-and-parity.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/sc-lint/sprint-C7.md`

## Deliverables

- `sc-lint-portability` adds `PORT-008` for production use of Unix-centric
  environment variables without a platform-neutral abstraction:
  - `HOME`
  - `USER`
  - `XDG_*`
- `crates/sc-lint-portability/src/lib.rs` extends `RuleId` with `Port008`
- the new env-portability rule fires on direct production lookups used as
  path, config-root, or user-identity inputs when no approved abstraction
  layer is present
- the rule text distinguishes this family from:
  - `PORT-002` configured home-dir wrapper enforcement
  - `PORT-003` test-only `std::env::set_var()` mutation
- `crates/sc-lint-portability/README.md` documents the new production
  env-portability rule and the intended abstraction strategy

## Explicit Code Samples

```rust
pub fn config_root() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    std::path::PathBuf::from(home).join(".config").join("sc-lint")
}
```

```rust
pub fn data_root() -> std::path::PathBuf {
    dirs::data_dir().expect("data directory")
}
```

## This Sprint Does Not Close

- production path-literal portability rules
- shell invocation portability rules
- generic `cfg(unix)` / `cfg(windows)` structural parity checks

## Acceptance Criteria

- the sprint defines `PORT-008` as a production env-portability rule in
  `sc-lint-portability`
- the sprint explicitly names `HOME`, `USER`, and `XDG_*` as the initial
  covered variable family
- the sprint keeps `PORT-002` and `PORT-003` semantics distinct instead of
  silently broadening either existing rule
- the sprint names a platform-neutral remediation path for production callers
- the sprint references GitHub issue `#54` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
