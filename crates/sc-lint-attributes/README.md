# `sc-lint-attributes`

`sc-lint-attributes` is an internal building block of the
[`sc-lint` product](https://crates.io/crates/sc-lint). This proc-macro crate
provides the `#[sc_lint(...)]` attribute namespace used by analyzer-owned
source declarations; it is not a standalone lint command. See the
[sc-lint repository](https://github.com/randlee/sc-lint).

Current crate release: **0.8.6**. API documentation is on
[docs.rs](https://docs.rs/sc-lint-attributes/0.8.6).

## Purpose

Use this crate when Rust source needs compile-valid `sc_lint` attributes for
policy declarations that analyzers later inspect, such as:

- `#[sc_lint(boundary.internal_only)]`
- `#[sc_lint(boundary.forbid_external_impls)]`
- approved `boundary.allow(...)` cycle suppressors

The crate stays intentionally thin: it provides the attribute namespace and
syntax acceptance while the analyzers own semantic enforcement.

## Key Types

This crate is a proc-macro surface rather than a deep domain model. Its role
is to:

- expose the `#[sc_lint(...)]` attribute entry point
- delegate syntax understanding to [`sc-lint-directives`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-directives/README.md)
- preserve stable source-level attribute usage across analyzer evolution

## Usage

In a consumer crate:

```toml
sc-lint-attributes = "0.8"
```

Example source:

```rust
use sc_lint_attributes::sc_lint;

#[sc_lint(boundary.internal_only)]
pub(crate) struct Token;
```

## Further Reading

- [`sc-lint-directives`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-directives/README.md)
- [Boundary enforcement model](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint-boundary/boundary-enforcement-model.md)
- [Crate architecture](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/crate-architecture.md)
- [MIT license](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-attributes/LICENSE)
