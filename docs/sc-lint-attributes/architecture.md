# `sc-lint-attributes` Architecture

## Role

This proc-macro crate exposes the compile-valid `#[sc_lint(...)]` attribute
namespace. It delegates directive syntax parsing to `sc-lint-directives`;
analyzers retain semantic enforcement.

## Governing ADRs

- [ADR-004 — Structured boundary definitions](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)

## Boundary Rules

- Keep the macro surface thin and syntax acceptance aligned with the shared
  directive model.
- Do not move analyzer policy or findings into the proc-macro crate.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-attributes/README.md)
