---
id: C.9
title: Cross-Platform cfg Parity Enforcement
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.9 — Cross-Platform cfg Parity Enforcement

## Goal

- add one shared structural parity rule to `sc-lint-portability`
- detect production `#[cfg(unix)]` items that expose one platform branch with
  no Windows companion or explicit portable fallback
- keep structural branch-parity analysis distinct from path, env, and shell
  leaf-pattern rules

## Hard Dependencies

- GitHub issue `#56` — cross-platform parity enforcement gap
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./adr/ADR-010-portability-scope-and-parity.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/sc-lint/sprint-C9.md`

## Deliverables

- `sc-lint-portability` adds `PORT-010` for production `cfg` parity drift when
  a Unix-only implementation branch has no Windows companion or explicit
  portable fallback
- the rule defines the accepted parity shapes for production code:
  - sibling `#[cfg(unix)]` and `#[cfg(windows)]` items
  - one `#[cfg(unix)]` item plus one explicit unguarded portable fallback item
- the rule operates on structural production items rather than only leaf
  string patterns, with planned coverage for:
  - free functions
  - modules
  - impl methods
- `crates/sc-lint-portability/README.md` documents the new structural parity
  rule and the accepted companion patterns

## Explicit Code Samples

```rust
#[cfg(unix)]
pub fn runtime_socket_name() -> &'static str {
    "/var/run/sc-lint.sock"
}
```

```rust
#[cfg(unix)]
pub fn runtime_socket_name() -> &'static str {
    "/var/run/sc-lint.sock"
}

#[cfg(windows)]
pub fn runtime_socket_name() -> &'static str {
    r"\\.\pipe\sc-lint"
}
```

## This Sprint Does Not Close

- production path-literal portability rules on their own
- broad production environment-variable portability rules
- shell invocation portability rules

## Acceptance Criteria

- the sprint defines `PORT-010` as a structural production parity rule in
  `sc-lint-portability`
- the sprint states the accepted companion forms for a Unix-only branch:
  Windows sibling or explicit portable fallback
- the sprint names the production item categories the structural scan must
  cover
- the sprint keeps leaf-pattern path, env, and shell portability findings in
  `C.6`, `C.7`, and `C.8` rather than duplicating those closures here
- the sprint references GitHub issue `#56` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
