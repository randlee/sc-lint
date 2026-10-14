# `sc-lint-directives` Requirements

## Purpose

The crate provides one canonical parse model for supported source directives.

## Requirement Traceability

- `REQ-PRODUCT-003` requires shared analyzer boundaries without direct
  analyzer-to-analyzer dependencies.
- `REQ-SCB-001` through `REQ-SCB-007` define the boundary directives and
  validation behavior consumed by this crate.

## Scope Rules

- The proc macro and analyzers use the same typed directive representation.
- Parsing and syntax validation do not decide analyzer findings.

## Related Docs

- [architecture.md](./architecture.md)
- [crate README](../../crates/sc-lint-directives/README.md)
- [boundary requirements](../sc-lint-boundary/requirements.md)
