# `sc-lint-attributes` Requirements

## Purpose

The crate supplies the source-level `#[sc_lint(...)]` attribute namespace
consumed by analyzer tooling.

## Requirement Traceability

- `REQ-PRODUCT-003` keeps analyzers independent while allowing shared source
  declaration surfaces.
- `REQ-SCB-001` through `REQ-SCB-007` define the boundary declaration and
  suppression contracts that use this attribute surface.

## Scope Rules

- Accepted syntax is parsed and represented by `sc-lint-directives`.
- Rule meaning and enforcement stay in the owning analyzer.

## Related Docs

- [architecture.md](./architecture.md)
- [crate README](../../crates/sc-lint-attributes/README.md)
- [boundary requirements](../sc-lint-boundary/requirements.md)
