# sc-lint Project Plan

This document is the high-level planning index for the `sc-lint` project.

## Current Planning Scope

The current project focus is:

- extracting `sc-lint` into a standalone repository
- stabilizing the initial crate set
- defining canonical crate boundaries for `sc-lint` itself
- establishing a self-hosting `just lint` development gate for this repo
- planning and introducing a top-level `sc-lint` CLI
- migrating boundary-definition handling to structured TOML sources
- migrating generic lint/view tooling into `sc-lint`
- moving boundary inventory and manifest-policy enforcement from Python into
  `sc-lint-boundary`
- preserving CI and lint-runner parity during extraction

## Current Detailed Planning References

- project roadmap
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- boundary TOML migration plan
  - see [docs/sc-lint/boundary-toml-migration.md](./sc-lint/boundary-toml-migration.md)
- boundary enforcement model rollout
  - see [docs/sc-lint/boundary-enforcement-model.md](./sc-lint/boundary-enforcement-model.md)
- extraction and migration plan
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
- current phase execution plan
  - see [docs/sc-lint/foundation-phase-plan.md](./sc-lint/foundation-phase-plan.md)
- initial analyzer MVP
  - see [docs/sc-lint/mvp.md](./sc-lint/mvp.md)

## Current Phase Priorities

This phase should execute in the following order:

1. define canonical `sc-lint` boundaries in TOML
2. make `just lint` self-host the repo's own analyzer checks by default
3. add the top-level `sc-lint` CLI crate
4. extract generic Python utilities
5. migrate boundary inventory + manifest policy from Python into
   `sc-lint-boundary`
6. run parity validation before deprecating Python boundary logic

## Release 1 Target

Release `0.1.x` should establish:

- a stable repo-local development and CI gate
- canonical TOML boundaries for the repo's own tool surfaces
- a documented and approved top-level `sc-lint` CLI contract
- a detailed extraction and migration plan for remaining generic tooling
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
