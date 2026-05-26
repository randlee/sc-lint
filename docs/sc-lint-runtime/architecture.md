# `sc-lint-runtime` Architecture

This document records the crate-local architecture summary for
`sc-lint-runtime`.

## Role

`sc-lint-runtime` owns AST-sensitive std runtime and concurrency analysis and
its backend machine contract.

## Authoritative Architecture Sources

- [../architecture.md](../architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)

## Boundary Rules

- the crate remains separate from `sc-lint-portability` and
  `sc-lint-boundary`
- generic std concurrency rules stay here
- Tokio-specific rules remain reserved for `sc-lint-tokio`

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
