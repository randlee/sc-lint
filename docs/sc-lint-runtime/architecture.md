# `sc-lint-runtime` Architecture

This document records the crate-local architecture summary for
`sc-lint-runtime`.

## Role

`sc-lint-runtime` owns AST-sensitive std runtime and concurrency analysis and
its backend machine contract.

## Authoritative Architecture Sources

- [../architecture.md](../architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)

## Governing ADRs

- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [ADR-013 — Analyzer shared support](../sc-lint/adr/ADR-013-analyzer-shared-support.md)

## Boundary Rules

- the crate remains separate from `sc-lint-portability` and
  `sc-lint-boundary`
- generic std concurrency rules stay here
- Tokio-specific rules remain reserved for `sc-lint-tokio`

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
