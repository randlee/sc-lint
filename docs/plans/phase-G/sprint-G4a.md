---
id: G.4a
title: sc-publish Delegation Qualification
status: planned
branch: n/a (external sc-publish consumer PR)
worktree: n/a (target repository worktree)
stack: external-non-branch
stack_base: n/a
target: ../sc-publish develop (external PR)
owner: cfast
---

# Sprint G.4a — sc-publish Delegation Qualification

## Goal

Adopt the released kit in `sc-publish` and remove its independent sc-lint
pin/action so its publishing surface delegates to the same version policy and
verified setup Action as every other consumer.

## Hard Dependencies

- G.2 and G.3b merged to `develop` and a versioned `sc-lint` release containing
  both is published.
- The G.2 adopter skill passes its own empty and established fixture evals.
- Write access to `../sc-publish`; its branch policy governs this external PR.

## Exact Targets

- `../sc-publish/plugins/sc-lint/` (kit installed verbatim by the adopter)
- `../sc-publish/sc-lint.toml`, `.sc-lint/`, `Justfile`, and
  `README.sc-lint.md` (kit-managed adoption end state)
- `../sc-publish/plugins/sc-publish/.github/actions/setup-lint-toolchain/action.yml`
- `../sc-publish/plugins/sc-publish/.github/actions/setup-sc-lint/` (delete)
- `../sc-publish/plugins/sc-publish/.github/workflows/release-preflight.yml`
- the external PR body and its attached dry-run output

## Deliverables

- Run the G.2 adopter verbatim against `../sc-publish`; commit only the
  reviewed kit end state and any explicitly listed, now-redundant local
  sc-lint scaffolding. The kit itself never deletes files.
- Make `setup-lint-toolchain` invoke the kit-provided local Action at
  `./plugins/sc-lint/.github/actions/setup-sc-lint`; remove the independent
  `setup-sc-lint` action and its `0.4.0` default pin.
- Keep release-preflight behavior intact while its sc-lint setup delegates to
  the kit. The PR body records every removal and the clean `--dry-run` output.

## Acceptance Criteria

- `python3 plugins/sc-lint/install.py --dry-run --input install.json .` exits
  0 after the adoption commit.
- `rg -n '0\\.4\\.0|setup-sc-lint' plugins/sc-publish/.github` returns no
  independent sc-lint pin/action reference; the only setup reference is the
  kit local Action path.
- `just setup`, `just lint`, and `just test` pass under the released kit
  contract, and every changed `sc-publish` CI job is green.
- The PR body contains the adopter's dry-run exit-0 output and removal list.

## Required Validation

- G.2 skill commands run verbatim in the target worktree.
- Target-repository CI and the release-preflight workflow are green.

## Out Of Scope

- consumer migration behavior specific to `sc-compose` (G.4c)
- any product or kit correction; file an issue and route a release fix first
