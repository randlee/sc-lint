# `sc-lint-runtime` Requirements

These requirements define the crate-local rule ownership for `sc-lint-runtime`.

## Purpose

`sc-lint-runtime` is the shared analyzer crate for std
runtime/concurrency-correctness rules.

## Scope Rules

- generic std runtime and liveness rules land here
- Tokio-specific rules remain out of scope and reserve `sc-lint-tokio` as
  their later home
- the crate must keep its machine contract stable through the top-level CLI
  normalization surface

## Requirement Traceability

- `REQ-PRODUCT-004B` assigns shared standard-library runtime and concurrency
  correctness rules to this crate.
- `REQ-PRODUCT-004C` reserves Tokio-specific rules for a separately justified
  analyzer crate.
- `SCB-RUNTIME-001` and `SCB-RUNTIME-002` are the current crate-owned rule
  contracts, documented in the crate README.
- `NFR-003` records the cross-cutting runtime scope.

## Related Docs

- [architecture.md](./architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)
