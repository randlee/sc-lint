# `sc-lint-schema` Requirements

## Purpose

The crate centralizes machine-visible finding and report transport types shared
by analyzer backends and the top-level CLI.

## Requirement Traceability

- `REQ-PRODUCT-002DA` requires one canonical top-level success/failure
  envelope family, normalized at the CLI boundary.
- `REQ-PRODUCT-003` and `REQ-PRODUCT-004D` establish the shared-crate and
  analyzer ownership boundaries.

## Scope Rules

- Types describe transport data; they do not perform analysis or decide policy.
- Backend-native contracts are adapted by the CLI rather than leaking as the
  stable product envelope.

## Related Docs

- [architecture.md](./architecture.md)
- [crate README](../../crates/sc-lint-schema/README.md)
- [CLI contract](../sc-lint/cli-contract.md)
