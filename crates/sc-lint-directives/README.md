# `sc-lint-directives`

`sc-lint-directives` provides the shared parsing and data types for
`#[sc_lint(...)]` source directives.

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
sc-lint-directives = { version = "0.1.0", path = "../sc-lint-directives" }
```

Example consumer path:

```rust
use sc_lint_directives::Directive;
```

## Further Reading

- [Boundary enforcement model](../../docs/sc-lint/boundary-enforcement-model.md)
- [Crate architecture](../../docs/sc-lint/crate-architecture.md)
- [`sc-lint-attributes`](../sc-lint-attributes/README.md)
