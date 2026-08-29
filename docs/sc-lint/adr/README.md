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
- [`ADR-011-interface-versioning-and-published-artifacts.md`](./ADR-011-interface-versioning-and-published-artifacts.md)
  - accepted `sc-lint-version` form-factor, interface-family configuration
    surface, and shared HTML/XHTML/JSON interface-report artifact policy
- [`ADR-012-consumer-adoption-and-just-contract.md`](./ADR-012-consumer-adoption-and-just-contract.md)
  - installed-product consumer orchestration, generated Just integration, and
    explicit source-maintainer versus consumer ownership
- [`ADR-013-analyzer-shared-support.md`](./ADR-013-analyzer-shared-support.md)
  - rule-neutral shared AST scanning/rendering support without analyzer-to-
    analyzer dependencies

## Draft ADRs

- [`ADR-015-standard-repo-tools-adoption-kit.md`](./ADR-015-standard-repo-tools-adoption-kit.md)
  - standard repo-tools adoption kit and rejection of Phase F's consumer-specific engine
- [`ADR-016-python-wheel-runtime-and-no-rust-configuration.md`](./ADR-016-python-wheel-runtime-and-no-rust-configuration.md)
  - version-matched Python wheel runtime and no-Rust configuration boundary

## Index Rules

- add every accepted ADR that affects the release line here
- update this index when ADR status changes
- keep [docs/project-plan.md](../../project-plan.md) and
  [docs/sc-lint/README.md](../README.md) aligned with this list
