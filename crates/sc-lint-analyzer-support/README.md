# `sc-lint-analyzer-support`

`sc-lint-analyzer-support` provides shared AST source discovery, scope
classification, and stable text-report rendering for the independently-owned
`sc-lint` analyzer crates.

It is a support library, not an analyzer or command surface. It has no rule
ids, no CLI, and no analyzer-to-analyzer dependency. `sc-lint-portability` and
`sc-lint-runtime` both depend on it so the scanner contract has one source of
truth.

See the installed [package guide](../../docs-bundle/packages/sc-lint-analyzer-support.md)
and [ADR-013](../../docs/sc-lint/adr/ADR-013-analyzer-shared-support.md).
