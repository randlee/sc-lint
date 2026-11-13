# `sc-lint-analyzer-support`

`sc-lint-analyzer-support` is an internal building block of the
[`sc-lint` product](https://crates.io/crates/sc-lint), not a user-facing
analyzer or command. See the
[sc-lint repository](https://github.com/randlee/sc-lint).

Current crate release: **0.8.9**. API documentation is on
[docs.rs](https://docs.rs/sc-lint-analyzer-support/0.8.9).

It provides shared AST source discovery, scope
classification, stable text-report rendering, and a rule-neutral parser for
analyzers' `[tool.sc-lint.analyzers.<name>]` configuration tables. The parser
validates common settings and `--rule-filter` selections using the rule data
each independently-owned analyzer crate supplies.

It is a support library, not an analyzer or command surface. It has no rule
ids, no CLI, and no analyzer-to-analyzer dependency. `sc-lint-portability` and
`sc-lint-runtime` both depend on it so the scanner contract has one source of
truth.

Add this crate as a library dependency with `cargo add sc-lint-analyzer-support@0.8`.

See the [package guide](https://github.com/randlee/sc-lint/blob/main/docs-bundle/packages/sc-lint-analyzer-support.md)
and [ADR-013](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/adr/ADR-013-analyzer-shared-support.md).
[MIT license](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-analyzer-support/LICENSE).
