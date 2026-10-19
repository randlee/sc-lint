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

This creates only `sc-lint.toml`, `Justfile`, and `.sc-lint/bootstrap.py`.

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

## Profiles, platform scope, and declared tools

Each lint step has a unique `name` and argv `command`. Optional `platform`
and `target` keys apply only to lint steps. `platform` is `linux`, `macos`, or
`windows`; a declared `target` must be a Cargo target triple for that platform
and is supported only for literal `cargo check` or `cargo clippy` commands.
Do not put `--target` in the command when using the `target` key. Profiles are
ordered lists of lint step names; they cannot contain other profiles, and a
profile name cannot shadow a step name or `all`.

```toml
[[tool.sc-lint.lint]]
name = "format"
command = ["cargo", "fmt", "--all", "--", "--check"]

[[tool.sc-lint.lint]]
name = "workspace-check"
command = ["cargo", "check", "--workspace"]

[[tool.sc-lint.lint]]
name = "windows-check"
command = ["cargo", "check", "--workspace"]
platform = "windows"
target = "x86_64-pc-windows-msvc"

[tool.sc-lint.lint_profiles]
ci = ["format", "workspace-check", "windows-check"]
```

The default platform scope, `host`, selects untagged steps and steps for the
current operating system. `all` also reports unsupported cross-platform
commands as skips; `linux`, `macos`, or `windows` selects that platform's
steps. Linux and macOS use `cargo xwin` for the Windows MSVC target shown
above. See [Just setup](./just-setup.md#select-lint-steps-and-run-managed-commands)
for the consumer command forms.

When a consumer adopts the kit, ordinary setup can provision only tools
declared in `.sc-lint/install.json`, next to `sc-lint.toml`. The persisted JSON
has a top-level `tools` object with `cargo` and `rustup_targets` collections:

```json
{
  "tools": {
    "cargo": {"cargo-xwin": "0.18.4"},
    "rustup_targets": ["x86_64-pc-windows-msvc"]
  }
}
```

Cargo packages require exact stable versions. During declared-tool setup, the
effective `CARGO_HOME` is the environment value when set, otherwise the
platform user's `.cargo` directory; setup adds its `bin` directory to the
child `PATH`. Setup checks the installed package version and executable,
installing a missing version with
`cargo install --locked --version =<version> <package>`. Rustup targets are
inspected, added if missing, and verified without selecting a toolchain. No
product default chooses a consumer's tools or versions, and lint execution
never installs tools.

`setup --dry-run` does not install tools, create the helper environment,
download files, or write. If the tools object declares tools but the managed
`sc_lint.setup_tools` helper is unavailable, dry-run reports
`SETUP_TOOLS.INSPECTION_UNAVAILABLE` instead of bootstrapping that helper.
Run ordinary `just setup` to provision the managed helper and declared tools.
See [troubleshooting](./troubleshooting.md) for recovery.

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
For the source-only identity-literal check and Windows console handling, see
[the detailed behavior notes](./best-practices.md#identity-literals-and-captured-output).
