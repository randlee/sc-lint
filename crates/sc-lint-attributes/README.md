# `sc-lint-attributes`

`sc-lint-attributes` is the proc-macro crate that provides the
`#[sc_lint(...)]` attribute namespace used by analyzer-owned source
declarations.

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
- delegate syntax understanding to [`sc-lint-directives`](../sc-lint-directives/README.md)
- preserve stable source-level attribute usage across analyzer evolution

## Usage

In a consumer crate:

```toml
sc-lint-attributes = { version = "0.1.0", path = "../sc-lint-attributes" }
```

Example source:

```rust
#[sc_lint(boundary.internal_only)]
pub(crate) struct Token;
```

## Further Reading

- [`sc-lint-directives`](../sc-lint-directives/README.md)
- [Boundary enforcement model](../../docs/sc-lint/boundary-enforcement-model.md)
- [Crate architecture](../../docs/sc-lint/crate-architecture.md)
