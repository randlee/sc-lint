---
id: C.6
title: Production Path-Literal Portability Parity
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.6 — Production Path-Literal Portability Parity

## Goal

- extend shared path-literal portability linting from test-only scope into
  production code
- add Windows-path parity to the existing Unix-path portability family
- keep path-literal detection separate from broader structural `cfg` parity

## Hard Dependencies

- GitHub issue `#53` — production path portability gap
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./adr/ADR-010-portability-scope-and-parity.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/sc-lint/sprint-C6.md`

## Deliverables

- `sc-lint-portability` adds one production-scope path-literal rule pair:
  - `PORT-006` hardcoded Unix-only absolute path literals in production code
  - `PORT-007` hardcoded Windows-only absolute path literals in production
    code
- the production path-literal rules reuse the existing source-scope walk so
  `PORT-001` remains test-only instead of silently changing semantics
- rule messages point callers toward platform-aware path sources or explicit
  platform-gated abstractions rather than hardcoded OS-specific literals
- `crates/sc-lint-portability/README.md` documents the new production
  path-literal rules and their intended portable alternatives

## Explicit Code Samples

```rust
pub fn unix_socket_dir() -> std::path::PathBuf {
    std::path::PathBuf::from("/var/run/sc-lint")
}

pub fn windows_cache_file() -> std::path::PathBuf {
    std::path::PathBuf::from(r"C:\ProgramData\sc-lint\cache.json")
}
```

```rust
pub fn cache_dir() -> std::path::PathBuf {
    dirs::cache_dir().expect("cache directory")
}
```

## This Sprint Does Not Close

- broader environment-variable portability rules
- shell invocation portability rules
- generic production `cfg(unix)` / `cfg(windows)` parity checks outside the
  path-literal family

## Acceptance Criteria

- the sprint defines `PORT-006` and `PORT-007` as `sc-lint-portability` owned
  production rules rather than extending `PORT-001` beyond test scope
- the sprint explicitly covers both Unix-only and Windows-only absolute path
  literal patterns in production code
- the sprint names at least one platform-aware alternative path source for
  remediation guidance
- the sprint keeps structural `cfg(unix)` companion analysis out of scope and
  assigns that closure to `C.9`
- the sprint references GitHub issue `#53` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
