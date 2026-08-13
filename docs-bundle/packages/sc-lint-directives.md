# sc-lint-directives

## Purpose and ownership

`sc-lint-directives` is the shared library-only parser for `sc-lint` policy
directives. It keeps directive names and argument parsing consistent between
the procedural macro and analyzer crates.

## Intended users

Analyzer and macro maintainers depend on it; ordinary consumers normally see it
only transitively through `sc-lint-attributes` and `sc-lint-boundary`.

## Configuration and API

The crate exposes parsed directive data for Rust syntax trees. Use the
version-matched release and do not duplicate directive parsing in a consumer
script:

```toml
[dependencies]
sc-lint-directives = "0.5.0"
```

## Output and CI

Parsing returns structured Rust values or a contextual error. Its correctness
is covered by the workspace tests and by boundary analyzer integration tests;
it emits no standalone report.

## Common failures

Unknown directive names and malformed arguments should be fixed at the source
attribute. Check [sc-lint-attributes](./sc-lint-attributes.md) for the macro
surface and rerun `cargo test --workspace`.

## Related packages

See [sc-lint-boundary](./sc-lint-boundary.md) for the primary consumer and
[sc-lint-schema](./sc-lint-schema.md) for shared output contracts.
