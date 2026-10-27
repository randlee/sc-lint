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
- `sleep.rs` owns the `SCB-RUNTIME-003` AST pass and its production-target and
  Tokio-package policy. ADR-020 decision 6 narrows ADR-007: rules requiring no
  Tokio dependency or runtime semantics may live here; `sc-lint-tokio` remains
  reserved for rules that do.
- `SleepScope` (in `runtime.rs`) decides which files the sleep pass visits. It
  keeps workspace packages that declare a normal or build `tokio` dependency and
  have a lib or bin target. Source files of the package's other targets (tests,
  examples, benches, build scripts) are excluded, and test files are skipped.
  The runtime analyzer table's `exclude_paths_by_rule` entries then filter
  `SCB-RUNTIME-003` findings by path.

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
