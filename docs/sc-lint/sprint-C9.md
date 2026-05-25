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
- detect production `#[cfg(unix)]` items that define one platform branch with
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
- `crates/sc-lint-portability/src/source_scan.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/sc-lint/sprint-C9.md`

## Deliverables

- `sc-lint-portability` adds `PORT-010` for production `cfg` parity drift when
  a Unix-only implementation branch has no Windows companion or explicit
  portable fallback
- `crates/sc-lint-portability/src/lib.rs` extends `RuleId` with `Port010`
- the rule defines the accepted parity shapes for production code:
  - sibling `#[cfg(unix)]` and `#[cfg(windows)]` items
  - one `#[cfg(unix)]` item plus one explicit unguarded portable fallback item
- the rule defines companion detection in machine-checkable terms:
  - sibling items share the same immediate parent scope:
    - file top-level
    - module body
    - impl block
  - an explicit portable fallback is an item with the same identifier as the
    `#[cfg(unix)]` item and no `#[cfg(...)]` attribute
  - a Windows companion is a sibling item with the same identifier and an
    explicit `#[cfg(windows)]` gate
- the structural detection seam is explicit and uses the same parent-scope
  walk that already classifies production items through `source_scan.rs`:

```rust
fn has_windows_companion(unix_item: &Item, sibling_items: &[Item]) -> bool {
    sibling_items.iter().any(|candidate| {
        same_identifier(unix_item, candidate) && item_has_cfg_windows(candidate)
    })
}

fn has_portable_fallback(unix_item: &Item, sibling_items: &[Item]) -> bool {
    sibling_items.iter().any(|candidate| {
        same_identifier(unix_item, candidate) && !item_has_any_cfg(candidate)
    })
}
```

- the rule operates on structural production items rather than only leaf
  string patterns, and its production scan covers:
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
- leaf portability findings inside explicit `#[cfg(unix)]` items stay owned by
  `C.6`, `C.7`, and `C.8`; `PORT-010` owns the structural decision about
  whether that Unix-only branch has an acceptable companion or portable
  fallback

## Acceptance Criteria

- the sprint defines `PORT-010` as a structural production parity rule in
  `sc-lint-portability`
- the sprint states the accepted companion forms for a Unix-only branch:
  Windows sibling or explicit portable fallback
- the sprint defines sibling scope and portable fallback in machine-detectable
  terms instead of leaving companion matching implicit
- the sprint requires the structural scan to cover all three production item
  categories:
  - free functions
  - modules
  - impl methods
- the sprint keeps leaf-pattern path, env, and shell portability findings in
  `C.6`, `C.7`, and `C.8` rather than duplicating those closures here
- the sprint references GitHub issue `#56` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo test -p sc-lint-portability flags_cfg_unix_item_without_windows_companion`
- `cargo test -p sc-lint-portability flags_cfg_unix_item_without_portable_fallback`
- `cargo test -p sc-lint-portability passes_cfg_unix_item_with_windows_companion`
- `cargo test -p sc-lint-portability passes_cfg_unix_item_with_portable_fallback`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
