# `sc-lint-directives` Architecture

## Role

This library owns the shared parsed types, syntax validation, and interpretation
of supported `#[sc_lint(...)]` source directives used by the proc macro and
analyzers.

## Governing ADRs

- [ADR-004 — Structured boundary definitions](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)

## Proposed Decision

The shared parse ownership split is recorded for Rand review in
[draft ADR-018](../sc-lint/adr/ADR-018-shared-source-and-report-types.md).

## Boundary Rules

- Parsing is shared; semantic rule ownership stays with the relevant analyzer.
- The crate must not depend on the CLI or analyzer implementations.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-directives/README.md)
