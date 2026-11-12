# `sc-lint-schema`

`sc-lint-schema` is an internal building block of the
[`sc-lint` product](https://crates.io/crates/sc-lint). It defines the shared
finding and report types used across the analyzer crates and top-level CLI; it
is not a standalone lint command. See the
[sc-lint repository](https://github.com/randlee/sc-lint).

Current crate release: **0.8.8**. API documentation is on
[docs.rs](https://docs.rs/sc-lint-schema/0.8.8).

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

- `validate_adapter_payload` and `PayloadDrift` for checking the shape of the
  Python adapter transport payload before normalization
- `validate_analyzer_report` and `AnalyzerReportDrift` for checking Rust
  analyzer output against the published schema selected by `schema_version`:
  [`analyzer-report-v1` for `0.1.0`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-schema/schema/analyzer-report-v1.schema.json) or
  [`analyzer-report-v2` for `0.2.0`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-schema/schema/analyzer-report-v2.schema.json)
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
sc-lint-schema = "0.8"
```

Example consumer path:

```rust
use sc_lint_schema::Finding;
```

## Further Reading

- [Graph schema](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint-boundary/graph-schema.md)
- [CLI contract](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/cli-contract.md)
- [Crate architecture](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/crate-architecture.md)
- [MIT license](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-schema/LICENSE)
