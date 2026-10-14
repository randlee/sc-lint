# `sc-lint-schema` Architecture

## Role

This library owns shared transport types for findings and analyzer reports.
Analyzer execution policy and top-level CLI normalization live outside it.

## Governing ADRs

- [ADR-006 — AI-first CLI contract](../sc-lint/adr/ADR-006-ai-first-cli-contract.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)

## Proposed Decision

The transport-only schema boundary is recorded for Rand review in
[draft ADR-018](../sc-lint/adr/ADR-018-shared-source-and-report-types.md).

## Boundary Rules

- Keep shared types neutral to analyzer rules and execution.
- The CLI normalizes backend payloads at its boundary; schema types do not
  become a second command or policy layer.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-schema/README.md)
