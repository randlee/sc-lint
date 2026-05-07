# sc-lint Requirements

This document is the high-level requirements index for the `sc-lint` project.

## Purpose

`sc-lint` provides reusable linting tools for Rust workspaces, starting with:

- boundary enforcement
- portability checks
- source-level lint attributes
- repo-local lint runner integration

## Current Requirement Areas

- Boundary definition and enforcement requirements
  - see [docs/sc-lint/requirements.md](./sc-lint/requirements.md)
- Structured boundary source migration requirements
  - see [docs/sc-lint/boundary-toml-migration.md](./sc-lint/boundary-toml-migration.md)
- Boundary enforcement model requirements
  - see [docs/sc-lint/boundary-enforcement-model.md](./sc-lint/boundary-enforcement-model.md)

## Requirement Management

- This file owns project-level requirement framing only.
- Detailed crate, rule-family, and migration requirements should live under
  `docs/sc-lint/`.
- As new `sc-lint` crates are added, crate-specific requirements should be
  linked here.
