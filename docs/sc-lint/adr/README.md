# `sc-lint` ADR Index

This index lists the architecture decision records that currently govern the
Phase A planning surface.

## Accepted ADRs

- [`ADR-004-structured-boundary-definitions.md`](./ADR-004-structured-boundary-definitions.md)
  - canonical TOML boundary definitions and planning-aware inventory parity
- [`ADR-005-cli-profiles-and-xwin-preflight.md`](./ADR-005-cli-profiles-and-xwin-preflight.md)
  - top-level CLI profile semantics and `xwin` preflight policy
- [`ADR-006-ai-first-cli-contract.md`](./ADR-006-ai-first-cli-contract.md)
  - canonical AI-first top-level CLI machine contract
- [`ADR-007-analyzer-crate-partition.md`](./ADR-007-analyzer-crate-partition.md)
  - analyzer-crate partitioning and primary lint-target mapping

## Index Rules

- add every accepted ADR that affects the release line here
- update this index when ADR status changes
- keep [docs/project-plan.md](../../project-plan.md) and
  [docs/sc-lint/README.md](../README.md) aligned with this list
