# `sc-lint-schema` Requirements

## Purpose

The crate centralizes machine-visible finding and report transport types shared
by analyzer backends and the top-level CLI.

## Requirement Traceability

- `REQ-PRODUCT-002DA` requires one canonical top-level success/failure
  envelope family, normalized at the CLI boundary; the `sc-lint-python-v1`
  adapter payload is validated by `validate_adapter_payload` before normalization.
  Rust analyzer reports are checked against the published JSON Schema by
  `validate_analyzer_report` before normalization.
- `REQ-PRODUCT-003` and `REQ-PRODUCT-004D` establish the shared-crate and
  analyzer ownership boundaries.

## Scope Rules

- Types describe transport data; they do not perform analysis or decide policy.
- The crate may hold pure shape validation of the transport payloads it defines
  (`validate_adapter_payload` and `validate_analyzer_report`); it still
  performs no analysis and decides no policy.
- Backend-native contracts are adapted by the CLI rather than leaking as the
  stable product envelope.

## Related Docs

- [architecture.md](./architecture.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [ADR-011 — Interface versioning and published artifacts](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)
- [crate README](../../crates/sc-lint-schema/README.md)
- [CLI contract](../sc-lint/cli-contract.md)
