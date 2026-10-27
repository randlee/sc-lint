# `sc-lint-schema` Architecture

## Role

This crate defines the shared machine-visible findings and report transport
types used by analyzer backends and normalized by the top-level CLI.

## Governing ADRs

- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [ADR-011 — Interface versioning and published artifacts](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)

## Boundary Rules

- Shared types carry data and do not implement analysis or policy.
- The crate may hold pure shape validation of the transport payloads it defines (`validate_adapter_payload`); it still performs no analysis and decides no policy.
- Backend-native machine contracts are normalized by the CLI.
- This crate remains below the analyzer and CLI layers in the dependency graph.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-schema/README.md)
- [CLI contract](../sc-lint/cli-contract.md)
