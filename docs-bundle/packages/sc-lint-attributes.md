# sc-lint-attributes

## Purpose and ownership

`sc-lint-attributes` is the published procedural-macro crate that expands
source-level `#[sc_lint(...)]` declarations into analyzer metadata. It is a
library-only package owned by the Rust source authoring layer; it is not a
consumer CLI.

## Intended users

Rust crates that annotate boundary policy use it as a development dependency.
Analyzer and workspace maintainers own the declarations and review expanded
policy alongside the source.

## Configuration and API

Add the version-matched crate and use the supported attribute forms documented
by [sc-lint-directives](./sc-lint-directives.md). Keep policy declarations
near the item they govern and let the compiler report malformed arguments.

```toml
[dependencies]
sc-lint-attributes = "0.5.0"
```

## Output and CI

The macro emits Rust items and metadata consumed by boundary analysis; it does
not produce a standalone report. CI should compile the annotated workspace and
run the configured `sc-lint lint sc-boundary` profile.

## Common failures

Unknown directives or malformed arguments are compile-time errors. If a rule is
not recognized, check the version pairing with
[sc-lint-directives](./sc-lint-directives.md) and rerun the workspace build.

## Related packages

See [sc-lint-boundary](./sc-lint-boundary.md) for enforcement and
[sc-lint-schema](./sc-lint-schema.md) for report data types.
