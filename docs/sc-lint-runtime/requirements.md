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

## Stable consumer interfaces

Rule IDs and this analyzer's JSON output are stable consumer contracts
(`REQ-PRODUCT-002`, [ADR-011](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)).

## Related Docs

- [architecture.md](./architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)
