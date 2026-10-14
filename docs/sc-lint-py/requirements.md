# `sc-lint-py` Requirements

## Purpose

The wheel provides the Python runtime for consumer-run helpers and a thin
binding to the existing CLI.

## Requirement Traceability

- `REQ-PRODUCT-019` governs consumer setup and the version-matched installed
  runtime.
- `REQ-PRODUCT-020` governs verified release-artifact installation and
  recoverable replacement behavior.
- `REQ-PRODUCT-021` governs the version-matched offline documentation bundle.
- `REQ-PRODUCT-024` governs wheel-delivered helpers and consumer-owned test
  recipes.
- ADR-016 defines the wheel boundary and prohibits configuration logic in
  Rust.

## Scope Rules

- The bridge exposes only existing CLI behavior and adds no lint logic.
- The wheel and CLI version remain matched; helpers are not copied from a
  source archive.

## Related Docs

- [architecture.md](./architecture.md)
- [Python bindings contract](../sc-lint/python-bindings.md)
- [crate README](../../bindings/sc-lint-py/README.md)
