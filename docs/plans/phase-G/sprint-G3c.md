---
id: G.3c
title: Identity-Literals Unicode-Escape Parser Fix
status: planned
branch: sprint/G.3c-identity-literals-unicode-fix
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.3c-identity-literals-unicode-fix
stack: C
stack_base: develop
target: develop (via stack C, PR base develop)
owner: cfast
---

# Sprint G.3c — Identity-Literals Unicode-Escape Parser Fix

## Goal

Fix the isolated identity-literals parser defect so valid Rust unicode escapes
are accepted without widening the release or adoption-kit scope.

## Hard Dependencies

- none; this disjoint targeted-fix stack roots directly on `develop` and has
  no touch point with Stack A or Stack B

## Exact Targets

- `crates/sc-lint-attributes/src/` identity-literals parser and its regression tests
- `docs/issues-inventory.md`

## Deliverables

- identity-literals accepts valid Rust unicode escapes, including `"\u{1F600}"`
  and `'\u{7}'`, with focused regression tests.
- The recorded consumer-blocking defect is closed without changing release
  packaging, adoption-kit assets, or configuration behavior.

## Acceptance Criteria

- The focused identity-literals regression test covers `"\u{1F600}"` and
  `'\u{7}'` and passes.
- `cargo test -p sc-lint-attributes` passes.

## Required Validation

- `cargo test -p sc-lint-attributes`

## Out Of Scope

- release packaging and wheel delivery (G.3a–G.3b)
- new lint rules or profiles
