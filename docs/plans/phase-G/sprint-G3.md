---
id: G.3
title: Self-Contained Release And Consumer-Blocking Lint Fixes
status: planned
branch: feature/phase-G3-self-contained-release
target: develop
---

# Sprint G.3 — Self-Contained Release And Consumer-Blocking Lint Fixes

## Goal

- guarantee the released `sc-lint` archive runs every kit recipe with no
  source-tree helper, and fix the lint defects consumers currently work around

## Hard Dependencies

- G.1 merged (kit recipes define what "every recipe" means)
- issue `#84` (full/ci profile runs source-tree Cargo wrappers)
- `scripts/release_artifacts.py`, `.github/workflows/release*.yml`

## Exact Targets

- `crates/sc-lint/src/dispatch.rs`
- `crates/sc-lint/src/python_adapter.rs`
- `crates/sc-lint/src/config.rs`
- `crates/sc-lint-attributes/src/` (identity-literals unicode-escape parser)
- `scripts/release_artifacts.py`
- `.github/workflows/release.yml`
- `docs/sc-lint/cli-contract.md`
- `docs/issues-inventory.md`
- `CHANGELOG.md`

## Deliverables

- A release-archive smoke test: install the built archive into
  `tests/fixtures/adoption/empty-workspace` via the kit and run `just setup
  lint test upgrade`; passes with `SC_LINT_SOURCE_ROOT` unset and no `.just/`
  directory present.
- `full` and `ci` profiles invoke only binaries shipped in the archive
  (`#84`); any profile entry that needs a Python helper ships that helper
  inside the archive under a product-owned path or is removed from the
  profile.
- identity-literals accepts valid Rust unicode escapes (regression test with
  `"\u{1F600}"` and `'\u{7}'`).
- `sc-lint version --json` reports the archive layout (`self_contained: true`).

## Acceptance Criteria

- CI job `release-smoke` green on the three OSes with no `.just/` in the
  workspace.
- `grep -rn "\.just/" crates/sc-lint/src` returns nothing outside tests.
- `#84` closed by the PR.

## Required Validation

- `cargo test --workspace`
- `sc-lint lint --profile ci`
- `just test-adoption`

## Out Of Scope

- new lint rules or profiles
