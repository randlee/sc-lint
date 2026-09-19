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

- none as start dependencies; this targeted-fix stack roots directly on
  `develop` and has no touch point with Stack A. Its one touch point with
  Stack B is the `.just/` helper reconciliation named in
  `phase-G-plan.md` (G.3a removes `.just/*.py` and ships the fixed modules
  from the `sc_lint` wheel); it is resolved by merge-forward after both
  stacks land and does not gate G.3c.

## Exact Targets

- `.just/lint_common.py` Rust-literal parser and `.just/lint_identity_literals.py`
  identity-literals utility, with focused tests under `.just/tests/`
- `docs/issues-inventory.md`

## Deliverables

- identity-literals accepts valid Rust unicode escapes, including `"\u{1F600}"`
  and `'\u{7}'`, with focused regression tests.
- The recorded consumer-blocking defect is closed without changing release
  packaging, adoption-kit assets, or configuration behavior.

## Acceptance Criteria

- The focused identity-literals regression test covers `"\u{1F600}"` and
  `'\u{7}'` and passes.
- `python3 -m unittest discover -s .just/tests -p 'test_lint*.py'` passes.
- `just lint` runs the identity-literals target successfully.

## Required Validation

- `python3 -m unittest discover -s .just/tests -p 'test_lint*.py'`
- `just lint`

Validation evidence for the implementation is recorded against the two
changed utility files: `test_lint_common.py` ran 15 tests and
`test_lint_identity_literals.py` ran 2 tests (17 focused tests total); the
broader `test_lint*.py` discovery ran 48 tests. The direct
`python3 .just/lint_identity_literals.py` identity-literals target and the
full `just lint` command both passed.

## Out Of Scope

- release packaging and wheel delivery (G.3a–G.3b)
- new lint rules or profiles
