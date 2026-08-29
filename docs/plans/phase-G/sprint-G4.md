---
id: G.4
title: First-Wave Consumer Adoption And sc-publish Delegation
status: planned
branch: n/a (consumer repositories and ../sc-publish)
target: each consumer's develop
owner: clint
---

# Sprint G.4 — First-Wave Consumer Adoption And sc-publish Delegation

## Goal

- prove the kit on the three first-wave repositories using only the G.2
  skill, and remove the duplicate sc-lint pin from `sc-publish`

## Hard Dependencies

- G.2 and G.3 merged and a `sc-lint` release published containing them
- `../sc-publish` write access; consumer repos `../wyvern`, `../atm-core`,
  `../sc-compose`

## Exact Targets

- `../sc-publish/plugins/sc-publish/.github/actions/setup-lint-toolchain/action.yml`
- `../sc-publish/plugins/sc-publish/.github/actions/setup-sc-lint/` (delete)
- `../sc-publish/plugins/sc-publish/.github/workflows/release-preflight.yml`
- `../wyvern` — consumer PR
- `../atm-core` — consumer PR
- `../sc-compose` — consumer PR (also removes `scripts/materialize_sc_lint_runtime.py`,
  `.just/*.py` sc-lint helpers, `.sc/sc-lint/targets/*.toml` forwarding targets,
  the `lint-ci-consumer` workaround recipe; keeps the native `template-contracts` target per D1)

## Deliverables

- sc-publish PR: `setup-lint-toolchain` uses `./.github/actions/setup-sc-lint`
  supplied by the sc-lint kit; its own `setup-sc-lint` and the `0.4.0` default
  are removed; consumer bump instructions in its README.
- One PR per consumer, opened by the adopter agent following `SKILL.md`
  verbatim, in order `wyvern` → `atm-core` → `sc-compose`. Each PR body
  contains the dry-run exit-0 output and the list of removed consumer-local
  scaffolding.
- Any change the adopter needs in `sc-lint` or the kit is filed as an issue and
  fixed under G.3's rules before that consumer PR merges; no consumer-local
  workaround is accepted.

## Acceptance Criteria

- In each consumer after merge: `just lint` and `just test` green on the
  consumer's CI matrix; `python3 plugins/sc-lint/install.py --dry-run --input
  install.json .` exits 0.
- `grep -rn "0\.4\.0\|\.just/" ../sc-publish/plugins/sc-publish/.github` returns nothing.
- `../sc-compose`: `ls .just scripts/materialize_sc_lint_runtime.py .sc/sc-lint/targets` fails (paths gone).

## Required Validation

- consumer CI per PR; user merges each consumer PR explicitly.

## Out Of Scope

- repositories beyond the first wave (G.5)
