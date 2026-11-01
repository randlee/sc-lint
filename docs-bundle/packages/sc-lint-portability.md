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

PORT-010 recognizes a non-test, same-name, same-kind `#[cfg(not(unix))]`
sibling in the same lexical scope: crate root (including `main.rs`), inline or
file module, or the same inherent impl (S3). Trait impls retain their existing
treatment. It also recognizes `#[cfg(unix)] mod x;` declarations and items in
proven Unix-only out-of-line modules, including public items (S6). Resolution
follows `x.rs`, `x/mod.rs` and `#[path]`. Missing or ambiguous files give no
Unix-only proof for their items; the exact Unix-gated module declaration is
exempt independently of resolution. Other shapes or deliberate Unix-only
items use a reasoned `#[sc_lint(allow(PORT-010), reason = "...")]` site allowance.

## Finding interpretation and CI

Treat portability findings as required unless the source contains an explicit,
reviewed platform fallback or a configured private-adapter exemption applies.
CI should run the complete profile and retain the normalized report for all
supported targets.

### Reviewed PORT-010 site allowance

Use an item-level allowance only when the public API is deliberately
Unix-specific and its reason names that cause:

```rust
#[cfg(unix)]
#[sc_lint(allow(PORT-010), reason = "This helper intentionally exposes the POSIX uid API.")]
pub fn current_uid() -> u32 { /* ... */ }
```

The exception covers only the annotated item. It does not cover an impl's
methods, siblings, a module, or a file. Consumers using the attribute add the
version-matched macro crate and admit it in each annotated crate's boundary:

```toml
[dependencies]
sc-lint-attributes = "0.8.4"
```

Add `sc-lint-attributes` to the `dependencies.allowed_dependencies` array in
the existing boundary record for every annotated consumer crate. For example,
the complete top-level record shape is:

```toml
boundary_id = "BOUNDARY-Example"
owner_package = "example"
owner_crate_path = "example"
name = "Example"

[public]
facade = "example::api"

[dependencies]
allowed_dependencies = ["sc-lint-attributes", "existing-package"]
```

Reports containing suppressed findings use schema version `0.2.0` and include
the `allowed` array with the rule, reason, path, line, and item identity. Empty
allowance reports remain schema version `0.1.0` and omit `allowed`. Consumers
must upgrade strict v1 validators before enabling the new report form.

## Common failures

Missing companion implementations, ungated platform APIs, and hard-coded shell
paths are common findings. Add a portable fallback or a precise `cfg` boundary,
then rerun the analyzer; do not disable the entire profile.

## Related packages

Pair with [runtime](./sc-lint-runtime.md) and
[sc-lint](./sc-lint.md)'s [CI guide](../ci.md). Shared finding serialization
is documented in [schema](./sc-lint-schema.md).
