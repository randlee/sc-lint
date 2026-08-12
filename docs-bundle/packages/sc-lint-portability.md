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

## Finding interpretation and CI

Treat portability findings as required unless the source contains an explicit,
reviewed platform fallback. CI should run the complete profile and retain the
normalized report for all supported targets.

## Common failures

Missing companion implementations, ungated platform APIs, and hard-coded shell
paths are common findings. Add a portable fallback or a precise `cfg` boundary,
then rerun the analyzer; do not disable the entire profile.

## Related packages

Pair with [runtime](./sc-lint-runtime.md) and
[sc-lint](./sc-lint.md)'s [CI guide](../ci.md). Shared finding serialization
is documented in [schema](./sc-lint-schema.md).
