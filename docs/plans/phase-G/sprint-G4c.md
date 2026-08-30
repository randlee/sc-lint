---
id: G.4c
title: sc-compose Migration Qualification
status: planned
branch: n/a (external sc-compose consumer PR)
worktree: n/a (target repository worktree)
stack: external-non-branch
stack_base: n/a
target: ../sc-compose develop (external PR)
owner: clint
---

# Sprint G.4c — sc-compose Migration Qualification

## Goal

Adopt the released kit in `sc-compose` only after G.4b's wyvern greenfield PR
and atm-core migration PR both merge, and remove the now-redundant
source-coupled sc-lint scaffolding without disturbing `sc-compose`'s native
`template-contracts` surface.

## Hard Dependencies

- Both G.4b PRs merged with required CI and dry-run checks green.
- The G.2/G.3b release used by G.4b remains the selected release for this PR.
- Write access to `../sc-compose`; its branch policy governs this external PR.

## Exact Targets

- `../sc-compose` kit-managed end-state files and one external consumer PR
- `../sc-compose/scripts/materialize_sc_lint_runtime.py` (delete)
- `../sc-compose/.just/*.py` sc-lint helpers (delete)
- `../sc-compose/.sc/sc-lint/targets/*.toml` forwarding targets (delete)
- `../sc-compose` `lint-ci-consumer` workaround recipe (delete)
- `../sc-compose` native `template-contracts` target (retain)

## Deliverables

- Run the G.2 adopter verbatim, then remove only the listed redundant
  sc-lint-forwarding scaffolding. The PR body distinguishes deleted forwarding
  code from retained `template-contracts` behavior.
- Do not add an adapter, copied Python file, special profile, or consumer-only
  exception. Any discovered gap is filed and fixed in a separate product/kit
  release before this PR can merge.
- Retain repository, kit version, PR URL, merge commit, date, and the
  removal/retention inventory as required G.5 rollout-table inputs.

## Acceptance Criteria

- `python3 plugins/sc-lint/install.py --dry-run --input install.json .` exits
  0 after the migration commit.
- `just setup`, `just lint`, and `just test` pass locally and on sc-compose CI
  through the kit; native `template-contracts` remains available and green.
- `test ! -e scripts/materialize_sc_lint_runtime.py`; `find .just -name '*.py'
  -print -quit` and `find .sc/sc-lint/targets -name '*.toml' -print -quit`
  produce no former sc-lint-forwarding helper/target path; the explicit PR
  diff verifies any remaining native `.just` assets are unrelated.
- No consumer-local workaround is introduced, and the PR body contains the
  clean dry-run output plus the removal/retention inventory.

## Required Validation

- G.2 skill commands run verbatim in the target worktree.
- sc-compose CI and `template-contracts` validation are green before merge.

## Out Of Scope

- rollout beyond first wave (G.5)
- product or kit changes in this consumer PR
