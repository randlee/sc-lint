# `sc-lint-directives`

`sc-lint-directives` is an internal building block of the
[`sc-lint` product](https://crates.io/crates/sc-lint). It provides the shared
parsing and data types for `#[sc_lint(...)]` source directives; it is not a
standalone lint command. See the
[sc-lint repository](https://github.com/randlee/sc-lint).

Current crate release: **0.8.6**. API documentation is on
[docs.rs](https://docs.rs/sc-lint-directives/0.8.6).

## Purpose

Use this crate when analyzer or proc-macro code needs one canonical
interpretation of supported `sc_lint` attributes, including:

- boundary declarations such as `boundary.internal_only`
- boundary declarations such as `boundary.forbid_external_impls`
- approved local allow directives for specific cycle categories

Centralizing directive parsing here prevents drift between proc-macro expansion
and backend analyzer ingestion.

## Key Types

This crate owns the directive parsing surface that both sibling crates consume:

- parsed directive structures
- typed directive variants for approved `boundary.*` forms
- validation helpers for supported attribute syntax

## Usage

It is consumed by both the proc-macro crate and analyzer backends:

```toml
sc-lint-directives = "0.8"
```

Example consumer path:

```rust
use sc_lint_directives::Directive;
```

## Further Reading

- [Boundary enforcement model](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint-boundary/boundary-enforcement-model.md)
- [Crate architecture](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/crate-architecture.md)
- [`sc-lint-attributes`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-attributes/README.md)
- [MIT license](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-directives/LICENSE)
