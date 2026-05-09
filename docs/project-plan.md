# sc-lint Project Plan

This document is the high-level planning index for the `sc-lint` project.

## Current Planning Scope

The current project focus is:

- extracting `sc-lint` into a standalone repository
- stabilizing the initial crate set
- defining canonical crate boundaries for `sc-lint` itself
- establishing a self-hosting `just lint` development gate for this repo
- planning and introducing a top-level `sc-lint` CLI
- defining the top-level CLI as an AI-first machine contract rather than only
  a human wrapper over backend tools
- migrating boundary-definition handling to structured TOML sources
- migrating generic lint/view tooling into `sc-lint`
- moving boundary inventory and manifest-policy enforcement from Python into
  `sc-lint-boundary`
- backporting reusable lint families that were first proven on `atm-core`
- keeping consumer-specific policy lints local unless only their framework is
  worth extracting
- improving pre-CI developer confidence with cross-target compile checks where
  those checks can surface platform drift before a push
- preserving CI and lint-runner parity during extraction

## Current Detailed Planning References

- project roadmap
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- boundary TOML migration plan
  - see [docs/sc-lint/boundary-toml-migration.md](./sc-lint/boundary-toml-migration.md)
- boundary enforcement model rollout
  - see [docs/sc-lint/boundary-enforcement-model.md](./sc-lint/boundary-enforcement-model.md)
- CLI requirements and contract
  - see [docs/sc-lint/cli-requirements.md](./sc-lint/cli-requirements.md)
  - see [docs/sc-lint/cli-architecture.md](./sc-lint/cli-architecture.md)
  - see [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)
- extraction and migration plan
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
- current phase execution plan
  - see [docs/sc-lint/foundation-phase-plan.md](./sc-lint/foundation-phase-plan.md)
  - see [docs/sc-lint/sprint-S1.md](./sc-lint/sprint-S1.md)
  - see [docs/sc-lint/sprint-S2.md](./sc-lint/sprint-S2.md)
  - see [docs/sc-lint/sprint-S3.md](./sc-lint/sprint-S3.md)
  - see [docs/sc-lint/sprint-S4.md](./sc-lint/sprint-S4.md)
- initial analyzer MVP
  - see [docs/sc-lint/mvp.md](./sc-lint/mvp.md)

## Current Phase Priorities

This phase should execute in the following order:

1. define canonical `sc-lint` boundaries in TOML
2. make `just lint` self-host the repo's own analyzer checks by default
3. add the top-level `sc-lint` CLI crate and define its canonical machine
   contract
4. extract generic Python utilities
5. backport reusable `atm-core`-proven analyzer families into `sc-lint`
6. define the cross-target preflight strategy for local and CI lint flows
7. migrate boundary inventory + manifest policy from Python into
   `sc-lint-boundary`
8. run parity validation before deprecating Python boundary logic

## Scheduled Sprint Plans

The currently scheduled foundation sprints are:

- `S.1`
  - CLI bootstrap
  - `docs/sc-lint/sprint-S1.md`
- `S.2`
  - profiles and Windows preflight
  - `docs/sc-lint/sprint-S2.md`
- `S.3`
  - generic utility extraction
  - `docs/sc-lint/sprint-S3.md`
- `S.4`
  - Rust boundary inventory migration and reusable analyzer backports
  - `docs/sc-lint/sprint-S4.md`

## Release 1 Target

Release `0.1.x` should establish:

- a stable repo-local development and CI gate
- canonical TOML boundaries for the repo's own tool surfaces
- canonical TOML boundaries for the planned top-level CLI contract items
- a documented and approved top-level `sc-lint` CLI contract
- explicit machine-contract decisions for:
  - canonical `--json` mode
  - stable machine-readable failures
  - reusable request/response seams
  - secondary interactive graph surfaces only
- a detailed extraction and migration plan for remaining generic tooling
- an explicit release-1 scope statement for which product surfaces are and are
  not boundary-inventory enforced
- a documented partition for:
  - reusable analyzer families that migrate into `sc-lint`
  - consumer-local policy lints that stay in their proving repo
- a documented strategy for surfacing likely Windows/Linux compile failures
  before CI without pretending that cross-target checks replace real
  multi-platform runners
- an initial Windows preflight path based on `cargo xwin check`, with a clear
  stance on whether and when `cargo xwin clippy` should be promoted
- documented `fast/full/ci` profile semantics and the distinction between:
  - `sc-lint lint ci`
  - `sc-lint ci`
- an implementation path for moving boundary inventory and manifest-policy
  logic into Rust with Python parity validation retained during migration

The current phase is the release-1 foundation phase. It does not imply that
every release-1 implementation item is already complete.

## Planning Conventions

- This file tracks project-level phases and priorities.
- Detailed sprint, phase, or feature plans should live under `docs/sc-lint/`
  unless they are repo-wide concerns.
- New crate introductions and major lint families should be added here as
  top-level planning items and then linked to their detailed plan documents.
