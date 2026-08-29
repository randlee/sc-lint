---
id: G.3b
title: Self-Contained Release And Consumer-Blocking Lint Fixes
status: planned
branch: sprint/G.3b-self-contained-release
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.3b-self-contained-release
stack: B
stack_base: sprint/G.3a-python-bindings
target: develop (via stack B, PR base sprint/G.3a-python-bindings)
owner: cfast
---

# Sprint G.3b — Self-Contained Release And Consumer-Blocking Lint Fixes

## Goal

- guarantee the released `sc-lint` archive runs every kit recipe with no
  source-tree helper, and fix the lint defects consumers currently work around

## Hard Dependencies

- G.3a's **Unblock Milestone** committed on
  `sprint/G.3a-python-bindings` (same stack). G.3b may begin product-only
  work at that commit, without waiting for G.3a CI, QA, review, or merge.
- The G.1 → G.3b bootstrap-copy reconciliation is not a start dependency.
  Before G.3b's release-closure and bootstrap-copy acceptance criteria run,
  G.1 must have merged to `develop` and that `develop` commit must be
  merge-forwarded into this worktree.
- issue `#84` (full/ci profile runs source-tree Cargo wrappers)
- `scripts/release_artifacts.py`, `.github/workflows/release*.yml`

## Exact Targets

- `crates/sc-lint/src/dispatch.rs`
- `crates/sc-lint/src/python_adapter.rs`
- `crates/sc-lint/src/config.rs`
- `crates/sc-lint-attributes/src/` (identity-literals unicode-escape parser)
- `scripts/release_artifacts.py`
- `.github/workflows/release.yml`
- `packages/sc-lint-adoption/.sc-lint/bootstrap`, `bootstrap.ps1` (re-sync verbatim from product after G.3a)
- `docs/sc-lint/cli-contract.md`
- `docs/issues-inventory.md`
- `CHANGELOG.md`

## Governing Contract

This sprint closes REQ-PRODUCT-019, REQ-PRODUCT-020, REQ-PRODUCT-021, and
REQ-PRODUCT-024 for the released artifact. The G.1 bootstrap-copy re-sync is
the one explicit cross-stack reconciliation: Stack B is never based on Stack
A, and G.3b merges `develop` forward only after G.1 has landed.

## Deliverables

- A release-archive smoke test: install the built archive into
  `tests/fixtures/adoption/empty-workspace` via the kit and run `just setup
  lint test upgrade`; passes with `SC_LINT_SOURCE_ROOT` unset and no `.just/`
  directory present.
- `full` and `ci` profiles invoke only binaries shipped in the archive or
  helpers imported from the `sc_lint` wheel (`#84`); no profile entry
  references a source-tree path.
- No Rust added for configuration: changes under `crates/` are limited to
  dispatch paths, the parser fix, and `version --json`; a diff adding any
  module named `configure`, `install`, `setup`, or `template` fails review.
- identity-literals accepts valid Rust unicode escapes (regression test with
  `"\u{1F600}"` and `'\u{7}'`).
- `sc-lint version --json` reports the archive layout (`self_contained: true`).
- Kit copies of `.sc-lint/bootstrap*` are byte-identical to the product files (`cmp` in CI).

## Acceptance Criteria

- CI job `release-smoke` green on the three OSes with no `.just/` in the
  workspace.
- `grep -rn "\.just/" crates/sc-lint/src` returns nothing outside tests.
- `git diff --stat develop -- crates/` touches no new module.
- `#84` closed by the PR.

## Required Validation

- `cargo test --workspace`
- `sc-lint lint --profile ci`
- `just test-adoption`

## Out Of Scope

- new lint rules or profiles
