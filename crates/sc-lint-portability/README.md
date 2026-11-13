# `sc-lint-portability`

`sc-lint-portability` is the Rust analyzer used by the
[`sc-lint` product](https://crates.io/crates/sc-lint) for cross-platform Rust
source checks. The `sc-portability` name is the analyzer target accepted by
`sc-lint lint`.

Current crate release: **0.8.9**. Install the product CLI with
`brew install randlee/tap/sc-lint` or from
[GitHub Releases](https://github.com/randlee/sc-lint/releases). To use this
crate as a Rust library, run `cargo add sc-lint-portability@0.8`. See the
[repository](https://github.com/randlee/sc-lint) and
[API documentation](https://docs.rs/sc-lint-portability/0.8.9).

## Purpose

Use `sc-portability` to catch portability drift such as:

- `PORT-001`
  - hardcoded Unix-only absolute paths in test code
- `PORT-002`
  - direct `dirs::home_dir()` without the configured override check
- `PORT-003`
  - `std::env::set_var()` in test code
- `PORT-004`
  - ungated `std::os::unix` imports in production code
- `PORT-005`
  - `#[cfg_attr(not(unix), allow(dead_code))]` portability suppressors
- `PORT-006`
  - hardcoded Unix-only absolute path literals in production code
- `PORT-007`
  - hardcoded Windows absolute path literals, including drive-letter and UNC
    forms, in production code; `#[cfg(windows)]`-gated items and expression
    blocks are exempt, as are the `\\?\` verbatim and `\\.\` device prefixes
- `PORT-008`
  - direct production lookups of `HOME`, `USER`, and `XDG_*` without a
    platform-neutral abstraction
- `PORT-009`
  - production `Command::new("sh" | "bash")` or hardcoded `/bin/sh` /
    `/bin/bash` shell-invocation assumptions without an explicit Unix-only
    boundary
- `PORT-010`
  - production `#[cfg(unix)]` items without a `#[cfg(windows)]` companion or
    explicit portable fallback in the same scope

## Ownership And Scope

Per [ADR-010](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md),
`sc-lint-portability` is the shared product owner for consumer-neutral:

- Windows-path parity companion rules for the current Unix-only path checks
- broader environment-variable portability rules
- shell-portability rules for OS-specific shell-path and shell-command
  assumptions
- structural `cfg` parity rules for Unix-only production branches

Consumer-specific portability wrappers may still exist in downstream repos, but
they do not replace `sc-lint-portability` as the core shared ownership
boundary for these rule families.

## Invocation

Primary product surface:

```bash
sc-lint lint sc-portability
sc-lint --json lint sc-portability
```

Repository lint gate:

```bash
just lint
```

Backend-local path:

```bash
cargo run -p sc-lint-portability -- analyze --root . --format text
cargo run -p sc-lint-portability -- analyze --root . --format json
```

## Representative Pass

Production code with Unix-only imports correctly gated:

```rust
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn configure() {}
```

Representative text result from the 0.8.5 binary on a one-crate workspace
with no findings:

```text
sc-lint-portability 0.8.5 status=pass scanned_crates=1 findings=0
```

## Representative Fail

Production code with an ungated Unix-only import:

```rust
use std::os::unix::fs::PermissionsExt;

pub fn configure() {}
```

The text summary from the 0.8.5 binary for an ungated Unix import is:

```text
sc-lint-portability 0.8.5 status=fail scanned_crates=1 findings=1
```

Finding details are in JSON. The `message` from this example is:

```text
/private/tmp/sc-lint-docs-085-A-sample/src/lib.rs:1: PORT-004 ungated std::os::unix import in production code; wrap the item with #[cfg(unix)] or move the import behind a Unix-only boundary
```

The backend `sc-lint-portability analyze` returns exit code `0` after a
completed analysis even when the report status is `fail`. The top-level
`sc-lint lint sc-portability` command returns exit code `10` for a configured
lint failure.

The other rules in the inventory above cover production Unix and Windows path
literals, home-directory environment lookups, Unix-shell assumptions, Unix
items without a Windows companion, and Unix-only paths in test code. Finding
messages are included in the JSON report with the source path and line.

## Configuration

`sc-lint-portability` reads `[tool.sc-lint.analyzers.portability]` from
`<root>/sc-lint.toml` by default. If that file is absent, the analyzer uses its
built-in defaults. Pass `--config PATH` to read a different TOML file; an
explicit path must exist and be readable.

`sc-lint-portability analyze` exits `0` when analysis completes, including
when its report status is `fail`; inspect `status` and `findings` in the
report. The top-level `sc-lint lint sc-portability` command applies the
configured lint policy. Invalid arguments or configuration return an error.

The analyzer table accepts common settings and portability-specific settings:

```toml
[tool.sc-lint.analyzers.portability]
enabled = true
reason = "This repository uses the portability analyzer"
exclude_rules = ["PORT-002"]
fail_rules = ["PORT-001", "PORT-006"]
exclude_paths = ["examples/legacy/**"]
config_home_env = "XDG_CONFIG_HOME"
unix_path_prefixes = ["/srv/"]
targets = ["linux", "macos", "windows"]
```

`enabled`, `reason`, `exclude_rules`, `fail_rules`, `exclude_paths`, and
`exclude_paths_by_rule` are common settings. `config_home_env`,
`unix_path_prefixes`, `targets`, and `platform_adapters` are
portability-specific. When both are present, `config_home_env` in the analyzer
table takes precedence over the legacy `[portability].config_home_env` value.
`unix_path_prefixes` adds prefixes to the built-in lists used by both PORT-001
and PORT-006; configured prefixes never replace or disable the built-ins.
`targets` is accepted for adopted configurations but is not interpreted by a
rule yet.

`platform_adapters` records explicit Unix/Windows module pairs and a review
reason for private implementations intended to provide the same interface. The
analyzer validates the configured pair but does not compare the implementations'
interfaces. For each paired module file, the exemption covers private `fn` and
`mod` items and private inherent-impl methods at file or inline-module scope.
Public items, trait-impl methods, impls nested in function bodies, unrelated
items, and separately Unix-gated public items remain subject to parity checks.

PORT-010 recognizes a non-test, same-name, same-kind `#[cfg(not(unix))]`
sibling in the same lexical scope: crate root (including `main.rs`), inline or
file module, or the same inherent impl (S3). Trait impls retain their existing
treatment. It also recognizes `#[cfg(unix)] mod x;` declarations and items in
proven Unix-only out-of-line modules, including public items (S6). Resolution
follows `x.rs`, `x/mod.rs` and `#[path]`. Missing or ambiguous files give no
Unix-only proof for their items; the exact Unix-gated module declaration is
exempt independently of resolution. Other shapes or deliberate Unix-only
items use a reasoned `#[sc_lint(allow(PORT-010), reason = "...")]` site allowance.

Pass `--rule-filter PORT-001,PORT-006` to select rules. When provided, the
filter determines the selected rules instead of `exclude_rules`. Invalid
configuration exits with code 3; an invalid rule filter exits with code 2.

## Output And Logging

Canonical machine mode:

```bash
sc-lint --json lint sc-portability
```

The top-level envelope is the same shared CLI contract:

- success:
  - `ok: true`
  - `command: "lint.sc-portability"`
  - findings payload under `data`
- failure:
  - `ok: false`
  - `command: "lint.sc-portability"`
  - stable `CliError` object under `error`

Structured logs for this tool use:

- service name:
  - `sc-portability`
- command identity:
  - `lint.sc-portability`

Read the command lifecycle through:

- `cli.command.started`
- `cli.command.completed`
  - includes `summary` and `elapsed_ms`
- `cli.command.error`
  - includes stable error code and `CliError.kind`

Delegated/backend normalization records also appear:

- `cli.dispatch.started`
- `cli.dispatch.normalized`
  - includes the delegated tool name and normalized finding count

See:

- [logging.md](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/logging.md)
- [cli-contract.md](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/cli-contract.md)
