# sc-lint-runtime

## Purpose and ownership

`sc-lint-runtime` is the AST-sensitive analyzer for runtime safety patterns,
including blocking waits, detached work, and supervision-sensitive execution.
It ships a backend binary and library.

## Intended users

Rust service and library maintainers use it through `sc-lint lint sc-runtime`;
runtime owners review each finding against the service's shutdown and
concurrency model.

## Configuration and commands

```sh
sc-lint lint sc-runtime
sc-lint lint ci
```

The analyzer reads Rust source under the configured root and emits normalized
findings. The top-level product owns command dispatch and CI aggregation.

## Finding interpretation and CI

A finding marks a discarded or unbounded runtime operation and includes the
source location and rule context. Prefer supervised regions, explicit timeout
handling, and observed results; document any intentional exception.

## Common failures

Bare waits, discarded timeout results, and detached tasks commonly fail the
profile. Inspect the exact operation and preserve error context while fixing
the ownership or timeout boundary.

## Related packages

Use [sc-lint-portability](./sc-lint-portability.md) for cross-platform concerns,
[sc-lint-boundary](./sc-lint-boundary.md) for architecture, and
[sc-lint-schema](./sc-lint-schema.md) for output types.
