# sc-lint-schema

## Purpose and ownership

`sc-lint-schema` is the library-only package defining shared machine-readable
finding and report structures. It is the compatibility boundary between
analyzers, the top-level CLI, and downstream report consumers.

## Intended users

Analyzer authors, report renderers, and CI integrations use its serialized
types. Consumer repositories generally receive these values through the
`sc-lint --json` envelope rather than importing the crate directly.

## Configuration and API

Depend on the version-matched crate when implementing an analyzer or report
adapter:

```toml
[dependencies]
sc-lint-schema = "0.8.2"
```

Keep field names and status values stable; add compatible fields rather than
changing the meaning of an existing field.

Rust analyzer reports are checked against the schema selected by
`schema_version` before the CLI normalizes them: [`analyzer-report-v1` for
`0.1.0`](https://github.com/randlee/sc-lint/blob/develop/crates/sc-lint-schema/schema/analyzer-report-v1.schema.json)
or [`analyzer-report-v2` for
`0.2.0`](https://github.com/randlee/sc-lint/blob/develop/crates/sc-lint-schema/schema/analyzer-report-v2.schema.json).
The `validate_analyzer_report` API returns the first schema mismatch as a
path and problem. The schema crate also validates the stable
`sc-lint-python-v1` adapter envelope; unknown keys remain allowed so the
transport can grow additively. JSON values use `serde_json`.

## Output and CI

Schema values serialize to JSON for reports and normalized CLI output. Test
serializations with the workspace suite and validate downstream consumers in
CI before publishing a breaking change.

## Common failures

Malformed JSON or an incompatible field shape indicates a version mismatch
between an analyzer and its renderer. Upgrade the complete sc-lint release,
then inspect the [troubleshooting guide](../troubleshooting.md).

## Related packages

All analyzer packages consume this contract; see
[sc-lint-boundary](./sc-lint-boundary.md),
[sc-lint-portability](./sc-lint-portability.md), and
[sc-lint-runtime](./sc-lint-runtime.md).
