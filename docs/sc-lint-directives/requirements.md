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
- The `#[sc_lint(...)]` grammar is backward compatible; additions preserve
  existing valid forms and their interpretation.

## Related Docs

- [architecture.md](./architecture.md)
- [ADR-004 — Structured boundary definitions](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [crate README](../../crates/sc-lint-directives/README.md)
- [boundary requirements](../sc-lint-boundary/requirements.md)
