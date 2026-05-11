# `sc-boundary` User Guide

`sc-boundary` is the shipped boundary-analysis tool for AST-sensitive Rust
ownership, boundary, and manifest-policy checks.

## Purpose

Use `sc-boundary` to check:

- architectural cycles
- source-level boundary declarations such as `boundary.internal_only`
- external impl restrictions such as `boundary.forbid_external_impls`
- Rust-native manifest-policy checks added in A.7

Current rule families include:

- `SCB-CYCLE-001`
- `SCB-CYCLE-002`
- `SCB-CYCLE-003`
- `SCB-BOUNDARY-001`
- `SCB-BOUNDARY-002`
- `SCB-BOUNDARY-003`
- `SCB-MANIFEST-001`
- `SCB-MANIFEST-002`

## Invocation

Primary product surface:

```bash
sc-lint lint sc-boundary
sc-lint --json lint sc-boundary
```

Repo-local convenience path:

```bash
just lint sc-boundary
```

Backend-local path:

```bash
cargo run -p sc-lint-boundary -- analyze --root . --format text
cargo run -p sc-lint-boundary -- analyze --root . --format json
```

Backend-local filtered analysis:

```bash
cargo run -p sc-lint-boundary -- analyze --root . --rule cycles --format text
cargo run -p sc-lint-boundary -- analyze --root . --rule boundaries --format text
```

Graph export stays backend-local and is adjacent to, but not part of, the
top-level `lint sc-boundary` contract:

```bash
cargo run -p sc-lint-boundary -- export-graph --root . --format json
cargo run -p sc-lint-boundary -- export-graph --root . --format turtle
```

## Representative Pass

Source with a local-only API and no cross-module violations:

```rust
mod parser {
    #[sc_lint(boundary.internal_only)]
    pub(crate) struct Token;

    pub(crate) fn parse() -> Token {
        Token
    }
}
```

Representative text result:

```text
sc-lint-boundary: PASS
scanned crates: 1
findings: 0
```

## Representative Fail

Trait marked `forbid_external_impls` but implemented outside its declaring
module:

```rust
mod api {
    #[sc_lint(boundary.forbid_external_impls)]
    pub trait Encode {}
}

mod adapters {
    pub struct Json;

    impl crate::api::Encode for Json {}
}
```

Representative finding:

```text
SCB-BOUNDARY-003 forbid_external_impls violation:
trait crate::api::Encode implemented outside declaring module by crate::adapters::Json
```

Manifest-policy failures also surface through the same tool:

```text
SCB-MANIFEST-001:
workspace.package.version missing from Cargo.toml
```

## Disable Model

Top-level `sc-lint` does not add rule-disable flags for `sc-boundary`.

Supported source-level suppression is limited to the current approved
`#[sc_lint(...)]` boundary directives:

- `#[sc_lint(boundary.allow("cycle.type_method_self_loop"))]`
- `#[sc_lint(boundary.allow("cycle.recursive_value_container"))]`

Supported boundary declarations are:

- `#[sc_lint(boundary.internal_only)]`
- `#[sc_lint(boundary.forbid_external_impls)]`

Important limits:

- these directives are backend-owned, not CLI-owned
- the cycle allowances only apply to the named cycle categories
- there is no top-level `--allow`, `--deny`, or profile-specific rule override
  flag in release `0.1.x`

## Output And Logging

Human output is a presentation layer. The canonical machine surface is:

```bash
sc-lint --json lint sc-boundary
```

That command returns the standard top-level envelope from
[`cli-contract.md`](../cli-contract.md):

- success:
  - `ok: true`
  - `command: "lint.sc-boundary"`
  - backend findings payload under `data`
- failure:
  - `ok: false`
  - `command: "lint.sc-boundary"`
  - stable `CliError` object under `error`

Structured logs for this tool use:

- service name:
  - `sc-boundary`
- command identity:
  - `lint.sc-boundary`

Read the shared command lifecycle like this:

- `cli.command.started`
  - command entry record for `lint.sc-boundary`
- `cli.command.completed`
  - final verdict plus `summary` and `elapsed_ms`
- `cli.command.error`
  - stable error code plus `CliError.kind` and summary message

Delegated/backend normalization records also appear for this tool:

- `cli.dispatch.started`
- `cli.dispatch.normalized`

For A.7 manifest-policy coverage, the same command path also carries:

- `manifest_policy_mode = "rust-native"`
- `manifest_policy_parity = "python-oracle"`

See:

- [logging.md](../logging.md)
- [ADR-006](../adr/ADR-006-ai-first-cli-contract.md)

