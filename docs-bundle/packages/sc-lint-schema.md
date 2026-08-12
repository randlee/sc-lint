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
sc-lint-schema = "0.4.0"
```

Keep field names and status values stable; add compatible fields rather than
changing the meaning of an existing field.

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
