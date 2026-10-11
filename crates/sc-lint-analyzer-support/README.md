# `sc-lint-analyzer-support`

`sc-lint-analyzer-support` provides shared AST source discovery, scope
classification, stable text-report rendering, and a rule-neutral parser for
analyzers' `[tool.sc-lint.analyzers.<name>]` configuration tables. The parser
validates common settings and `--rule-filter` selections using the rule data
each independently-owned analyzer crate supplies.

It is a support library, not an analyzer or command surface. It has no rule
ids, no CLI, and no analyzer-to-analyzer dependency. `sc-lint-portability` and
`sc-lint-runtime` both depend on it so the scanner contract has one source of
truth.

See the installed [package guide](../../docs-bundle/packages/sc-lint-analyzer-support.md)
and [ADR-013](../../docs/sc-lint/adr/ADR-013-analyzer-shared-support.md).
