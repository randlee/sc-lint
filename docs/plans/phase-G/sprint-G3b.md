---
id: G.3b
title: Self-Contained Release
status: planned
branch: sprint/G.3b-self-contained-release
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.3b-self-contained-release
stack: B
stack_base: sprint/G.3a-python-bindings
target: develop (via stack B, PR base sprint/G.3a-python-bindings)
owner: flint
# Owner assignment: clint owns most sprints; cfast takes easy closure/fix work; flint takes the harder parallel Stack B track.
---

# Sprint G.3b — Self-Contained Release

## Goal

- guarantee the released `sc-lint` archive runs every kit recipe with no
  source-tree helper

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

As implemented (PR #142):

- `crates/sc-lint/src/dispatch.rs`, `workflow.rs`, `command.rs`, `consts.rs`,
  `config.rs` (self-invoking `product_step`, `self_contained`, backend lookup)
- `crates/sc-lint-portability/src/portability.rs` (`sc-lint.toml` is the only
  repo config; legacy `.just/lint-config.toml` fallback removed)
- `bindings/sc-lint-py/python/sc_lint/run_lint.py` and tests (source-tree
  `lint_sc_boundary.py` / `lint_sc_portability.py` removed)
- `Justfile`, `sc-lint.toml`, `scripts/release_smoke.py`,
  `.github/workflows/ci.yml` (`release-smoke` job),
  `tests/fixtures/adoption/empty-workspace`
- `docs/sc-lint/cli-contract.md`, `docs/issues-inventory.md`, `CHANGELOG.md`

Closure notes: `python_adapter.rs`, `scripts/release_artifacts.py`, and
`.github/workflows/release.yml` needed no change — G.3a already made the
Python helpers wheel-resident and the release archive already carries every
backend binary. `packages/sc-lint-adoption/.sc-lint/bootstrap*` re-sync is
deferred to the G.1 → G.3b reconciliation (see below).

## Governing Contract

This sprint closes REQ-PRODUCT-020 and REQ-PRODUCT-021 for the released
artifact and delivers the G.3b portion of REQ-PRODUCT-024 (remaining
REQ-PRODUCT-024 ownership stays with G.4b/G.4c per the phase-G traceability
table). The G.1 bootstrap-copy re-sync is
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
  dispatch paths and `version --json`; a diff adding any
  module named `configure`, `install`, `setup`, or `template` fails review.
- `sc-lint version --json` reports the archive layout (`self_contained: true`).
- Kit copies of `.sc-lint/bootstrap*` are byte-identical to the product files (`cmp` in CI).
  **Deferred** to the G.1 → G.3b reconciliation: the kit does not exist until
  `sprint/G.1-adoption-kit` merges to `develop`.

## Acceptance Criteria

- CI job `release-smoke` green on the three OSes with no `.just/` in the
  workspace.
- `grep -rn "\.just/" crates/sc-lint/src` returns nothing outside tests.
- `git diff --stat develop -- crates/` touches no new module.
- `#84` closed by the PR.

## Unblock Milestone

Commit the self-contained archive smoke path, including the release artifact
layout and wheel-only helper invocation. Stack B ends at G.3b; G.3c is an
independent `develop`-rooted Stack C and has no dependency on this commit.

## Required Validation

- `cargo test --workspace`
- `sc-lint lint --profile ci`
- `just test-adoption` — **deferred** with the kit `cmp` check until G.1 has
  landed; `release-smoke` CI covers the archive path in the meantime.

## Out Of Scope

- new lint rules or profiles
