# Using sc-lint

The installed product owns consumer lint, test, setup, upgrade, and diagnostic
behavior. `just` is a thin memorable interface over these commands.

## First use

For the first consumer repository, install the product from the supported
package or verified release archive, then generate and commit the managed
integration:

```sh
sc-lint init --just
just setup
```

This creates only `sc-lint.toml`, `Justfile`, `.sc-lint/bootstrap`, and the
Windows companion `.sc-lint/bootstrap.ps1`.

On every later clean checkout, including a machine with no `sc-lint` binary on
`PATH`, run `just setup`. The bootstrap helper downloads the release selected
by `minimum_version`, verifies its SHA-256 checksum from the matching release
manifest, installs the complete product bundle, and checks companion identity
and health before activation. Set `SC_LINT_INSTALL_DIR` to choose the managed
location. `SC_LINT_BIN` selects an explicit executable for direct bootstrap
execution; ordinary setup/lint/test/upgrade use compatible-product resolution
and preflight.

## Daily commands

```sh
just lint
just test
```

`just lint` runs the complete configured lint profile and `just test` runs the
complete configured test profile. Both perform the same minimum-version
preflight and, when necessary, the same verified bootstrap before starting
work. A bootstrap or preflight failure stops before any backend command runs.

Direct product equivalents are useful in automation:

```sh
sc-lint lint ci --consumer --config sc-lint.toml --profile ci
sc-lint test --config sc-lint.toml
```

Consumer lint steps are declared in `[[tool.sc-lint.lint]]`; named profiles
are ordered lists of those step names in `[tool.sc-lint.lint_profiles]`.
`--profile` selects a step or named profile, and omitting it or using `all`
runs every step in declaration order. A profile cannot contain another
profile. `--platform-scope host` is the default: it includes untagged steps and
steps for the host OS. `all` also reports unsupported cross-platform commands
as skips; `linux`, `macos`, or `windows` includes that platform's steps.

Only literal `cargo check` and `cargo clippy` commands can declare a target.
Native commands run unchanged, same-host targets add Cargo's `--target`, and
Linux/macOS to `x86_64-pc-windows-msvc` runs through `cargo xwin`. Other
cross-target combinations produce an ordered `skipped` workflow row; missing
tools and failures for supported combinations fail the run. Selection is
sequential and fail-fast. Generic test groups, parallel or keep-going modes,
setup hooks, and forwarding extra test arguments are not part of this contract.

When a consumer uses the adoption kit, setup can provision only tools
explicitly declared in that kit's persisted `.sc-lint/install.json` `tools`
object. Cargo packages require exact
stable versions; rustup targets are inspected, added if missing, and verified.
`sc-lint setup --dry-run --config sc-lint.toml` inspects the managed product
release and install location without changing either. The generated
`.sc-lint/bootstrap setup --dry-run --config sc-lint.toml` also checks declared
tools and targets without installing or writing. No product default chooses a
consumer's tools or versions. Lint
execution never installs tools. See [Just setup](./just-setup.md) and
[troubleshooting](./troubleshooting.md) for recovery.

## Inspecting output

Add `--json` to receive the stable success/error envelope. Human output names
the command, stable code, cause, suggested recovery, and documentation guide.
Use `sc-lint docs troubleshooting` to map a code to the next action.
Analyzer findings preserve report output when the verdict fails: exit 10 means
analysis completed with failing findings; exit 3 means invalid analyzer
configuration; exit 2 means a command-line selector or usage error. Text
verdicts and JSON reports agree with the process status.

## Source maintainers versus consumers

Source maintainers use the repository's `just lint` and `just test` aggregate
recipes, which include source-only checks. Consumers use the generated thin
Justfile and never need to know analyzer package names or the source checkout.
