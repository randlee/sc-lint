# `sc-lint` ADR Index

This index lists the architecture decision records that currently govern the
current planning surface.

## Accepted ADRs

- [`ADR-004-structured-boundary-definitions.md`](./ADR-004-structured-boundary-definitions.md)
  - canonical TOML boundary definitions and planning-aware inventory parity
- [`ADR-005-cli-profiles-and-xwin-preflight.md`](./ADR-005-cli-profiles-and-xwin-preflight.md)
  - top-level CLI profile semantics and `xwin` preflight policy
- [`ADR-006-ai-first-cli-contract.md`](./ADR-006-ai-first-cli-contract.md)
  - canonical AI-first top-level CLI machine contract
- [`ADR-007-analyzer-crate-partition.md`](./ADR-007-analyzer-crate-partition.md)
  - analyzer-crate partitioning and primary lint-target mapping
- [`ADR-008-sc-observability-logging.md`](./ADR-008-sc-observability-logging.md)
  - `sc-observability` selection and CLI-owned structured logging policy
- [`ADR-009-observability-boundary-policy.md`](./ADR-009-observability-boundary-policy.md)
  - accepted observability boundary seams, permitted type crossings, and
    direct-link constraints
- [`ADR-010-portability-scope-and-parity.md`](./ADR-010-portability-scope-and-parity.md)
  - shared portability ownership and Unix/Windows parity scope policy

## Index Rules

- add every accepted ADR that affects the release line here
- update this index when ADR status changes
- keep [docs/project-plan.md](../../project-plan.md) and
  [docs/sc-lint/README.md](../README.md) aligned with this list
