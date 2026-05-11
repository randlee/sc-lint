# `sc-lint-schema`

`sc-lint-schema` defines the shared findings and report schema used across the
`sc-lint` analyzer crates and top-level CLI.

## Purpose

Use this crate to share stable machine-visible data structures between:

- analyzer backends such as `sc-lint-boundary`, `sc-lint-portability`, and
  `sc-lint-runtime`
- the top-level `sc-lint` CLI normalization layer
- JSON rendering and integration tests that need a consistent payload shape

This crate exists so schema ownership is centralized instead of duplicated
across analyzer crates.

## Key Types

Representative shared schema types include:

- finding records
  - stable rule id, location, message, and severity fields
- analyzer report payloads
  - normalized `PASS`/`FAIL` style report content
- small shared identifier/value newtypes used across backends

The exact Rust surface is intentionally schema-oriented: it carries transport
types, not analyzer execution policy.

## Usage

Backends depend on this crate for shared output types:

```toml
sc-lint-schema = { version = "0.1.0", path = "../sc-lint-schema" }
```

Example consumer path:

```rust
use sc_lint_schema::Finding;
```

## Further Reading

- [Graph schema](../../docs/sc-lint/graph-schema.md)
- [CLI contract](../../docs/sc-lint/cli-contract.md)
- [Crate architecture](../../docs/sc-lint/crate-architecture.md)
