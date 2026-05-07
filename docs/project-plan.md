# sc-lint Project Plan

This document is the high-level planning index for the `sc-lint` project.

## Current Planning Scope

The current project focus is:

- extracting `sc-lint` into a standalone repository
- stabilizing the initial crate set
- migrating boundary-definition handling to structured TOML sources
- preserving CI and lint-runner parity during extraction

## Current Detailed Planning References

- project roadmap
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- boundary TOML migration plan
  - see [docs/sc-lint/boundary-toml-migration.md](./sc-lint/boundary-toml-migration.md)
- boundary enforcement model rollout
  - see [docs/sc-lint/boundary-enforcement-model.md](./sc-lint/boundary-enforcement-model.md)
- initial analyzer MVP
  - see [docs/sc-lint/mvp.md](./sc-lint/mvp.md)

## Planning Conventions

- This file tracks project-level phases and priorities.
- Detailed sprint, phase, or feature plans should live under `docs/sc-lint/`
  unless they are repo-wide concerns.
- New crate introductions and major lint families should be added here as
  top-level planning items and then linked to their detailed plan documents.
