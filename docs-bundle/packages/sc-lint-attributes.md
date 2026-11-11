# sc-lint-attributes

## Purpose and ownership

`sc-lint-attributes` is the published procedural-macro crate that provides the
source-level `#[sc_lint(...)]` attribute. The macro checks the attribute's
arguments with [sc-lint-directives](./sc-lint-directives.md) and returns the
annotated item unchanged; it generates no code. The analyzers read the
attribute from source. It is a library-only package, not a consumer CLI.

## Intended users

Rust crates that annotate boundary or portability policy on production items
add it under `[dependencies]`, because the attribute must compile wherever the
annotated item does. Analyzer and workspace maintainers own the declarations
and review them alongside the source.

## Configuration and API

Add the version-matched crate and use the supported attribute forms documented
by [sc-lint-directives](./sc-lint-directives.md). Keep policy declarations
near the item they govern and let the compiler report malformed arguments.

```toml
[dependencies]
sc-lint-attributes = "0.8.8"
```

## Output and CI

The macro emits only the original item; it does not produce a report. CI
should compile the annotated workspace and run `sc-lint lint sc-boundary` or
`sc-lint lint sc-portability`, which read the attributes from source.

## Common failures

Unknown directives or malformed arguments are compile-time errors. If a rule is
not recognized, check the version pairing with
[sc-lint-directives](./sc-lint-directives.md) and rerun the workspace build.

## Related packages

See [sc-lint-boundary](./sc-lint-boundary.md) for enforcement and
[sc-lint-schema](./sc-lint-schema.md) for report data types.
