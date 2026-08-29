---
id: G.4b
title: Greenfield Consumer Qualification
status: planned
branch: n/a (external wyvern and atm-core consumer PRs)
worktree: n/a (target repository worktrees)
stack: external-non-branch
stack_base: n/a
target: ../wyvern develop and ../atm-core develop (external PRs)
owner: clint
---

# Sprint G.4b — Greenfield Consumer Qualification

## Goal

Prove the released adoption kit independently in the two greenfield first-wave
repositories, `wyvern` and `atm-core`, before attempting the established
`sc-compose` migration.

## Hard Dependencies

- G.2 and G.3b merged to `develop` and a versioned release containing both is
  published.
- The G.2 adopter skill passes its durable fixture evals.
- Write access to `../wyvern` and `../atm-core`; their branch policies govern
  their independent external PRs.

## Exact Targets

- `../wyvern` kit-managed end-state files and one external consumer PR
- `../atm-core` kit-managed end-state files and one external consumer PR
- each PR body and attached `--dry-run` evidence

## Deliverables

- Run the G.2 adopter verbatim in disposable worktree copies of both target
  repositories. Each consumer receives the identical kit end state: sole
  `minimum_version` pin, product-owned bootstrap/Just import, kit Action and
  workflow, vendored `plugins/sc-lint`, and `README.sc-lint.md`.
- Open the two PRs in parallel. Each records the exact kit release, clean
  drift check, and any consumer-local scaffolding removal. A kit/product gap is
  an issue and release fix, never a consumer-local wrapper.

## Acceptance Criteria

- In each PR after install, `python3 plugins/sc-lint/install.py --dry-run
  --input install.json .` exits 0 without a write.
- In each PR, `just setup`, `just lint`, and `just test` pass locally and on
  that repository's CI matrix through the kit.
- Both PRs merge with green CI before G.4c starts; their merged commit IDs,
  kit version, and PR URLs are retained as required G.5 rollout-table inputs.

## Required Validation

- G.2 skill commands run verbatim in both target worktrees.
- Each target repository's CI is green before its PR merge.

## Out Of Scope

- modifications to the adoption kit or `sc-lint` product
- `sc-compose` legacy-scaffolding removal (G.4c)
