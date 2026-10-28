# sc-lint-portability

## Purpose and ownership

`sc-lint-portability` is the AST-sensitive analyzer for host portability:
platform-gated code, shell/path literals, environment lookups, and equivalent
Windows or non-Unix behavior. It ships a backend binary and library.

## Intended users

Rust workspace maintainers run it through the product's portability profile;
CI owners use the findings to keep Linux, macOS, and Windows lanes equivalent.

## Configuration and commands

```sh
sc-lint lint sc-portability
sc-lint lint ci
```

The analyzer inspects the configured repository root and reports source path,
rule, and portable remediation. It does not read consumer `sc-lint.toml`
profiles directly; the top-level product orchestrates it.

For equivalent private platform implementations with different module paths,
`platform_adapters` records an explicit Unix/Windows pair and a review reason
in `[tool.sc-lint.analyzers.portability]`. The analyzer validates the configured
pair but does not compare the implementations' interfaces; the review reason
records that human assessment. For each paired module file, the exemption covers
private `fn` and `mod` items and private inherent-impl methods at file or
inline-module scope. Public items, trait-impl methods, impls nested in function
bodies, unrelated items, and separately Unix-gated public items remain subject
to normal parity checks. Keep the pair specific. Common `exclude_paths_by_rule`
entries suppress only the named rule's findings at the listed paths; they do
not hide other findings in the same file. See
[configuration](../configuration.md) for syntax and filter precedence.

Known limitation: a crate-root `main.rs` with a `#[cfg(unix)]` function and a
`#[cfg(not(unix))]` sibling is still reported by PORT-010. This behavior is
tracked for follow-up in issue `lint-phase-l.3`.

## Finding interpretation and CI

Treat portability findings as required unless the source contains an explicit,
reviewed platform fallback or a configured private-adapter exemption applies.
CI should run the complete profile and retain the normalized report for all
supported targets.

## Common failures

Missing companion implementations, ungated platform APIs, and hard-coded shell
paths are common findings. Add a portable fallback or a precise `cfg` boundary,
then rerun the analyzer; do not disable the entire profile.

## Related packages

Pair with [runtime](./sc-lint-runtime.md) and
[sc-lint](./sc-lint.md)'s [CI guide](../ci.md). Shared finding serialization
is documented in [schema](./sc-lint-schema.md).
