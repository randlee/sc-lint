---
id: G.3c
title: Identity-Literals Unicode-Escape Parser Fix
status: planned
branch: sprint/G.3c-identity-literals-unicode-fix
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.3c-identity-literals-unicode-fix
stack: B
stack_base: sprint/G.3b-self-contained-release
target: develop (via stack B, PR base sprint/G.3b-self-contained-release)
owner: flint
---

# Sprint G.3c — Identity-Literals Unicode-Escape Parser Fix

## Goal

Fix the isolated identity-literals parser defect so valid Rust unicode escapes
are accepted without widening the release or adoption-kit scope.

## Hard Dependencies

- G.3b's **Unblock Milestone** committed on
  `sprint/G.3b-self-contained-release`; this sprint begins without waiting for
  G.3b CI, QA, review, or merge.

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
