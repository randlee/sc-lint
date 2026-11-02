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
`--profile` selects a step or named profile. Omitting it uses
`[tool.sc-lint] default_lint_profile` when configured; otherwise it selects
every step. Explicit `--profile all` always selects every step. A profile
cannot contain another profile. `--platform-scope host` is the default: it includes untagged steps and
steps for the host OS. `all` also reports unsupported cross-platform commands
as skips; `linux`, `macos`, or `windows` includes that platform's steps.

Only literal `cargo check` and `cargo clippy` commands can declare a target.
Native commands run unchanged, same-host targets add Cargo's `--target`, and
Linux/macOS to `x86_64-pc-windows-msvc` runs through `cargo xwin`. Other
cross-target combinations produce an ordered `skipped` workflow row; missing
tools and failures for supported combinations fail the run.

Consumer lint and test steps run in parallel by default and fail slow: every
selected step runs, the report keeps declaration order, and a failed run names
every failed step. A step whose `argv[0]` is `cargo` runs on one serialized
cargo lane in declaration order; any other program runs on its own worker
(there is no worker cap, and cargo still serializes itself on its build
lock). A step marked `after_all = true` starts only after every other step has
finished, and several such steps run one at a time in declaration order. Pass
`--fail-fast` (`sc-lint lint ci --consumer --fail-fast`, `sc-lint test
--fail-fast`) to stop starting cargo-lane and `after_all` steps after the
first failure; steps already running finish, and the skipped steps are listed
separately from the failed ones. Test selectors, forwarded test arguments and
`sc-lint test --list` are described under [Test selectors](#test-selectors).

## Profiles, platform scope, and declared tools

Each lint step has a unique `name` and argv `command`. Optional `platform`
and `target` keys apply only to lint steps. `platform` is `linux`, `macos`, or
`windows`; a declared `target` must be a Cargo target triple for that platform
and is supported only for literal `cargo check` or `cargo clippy` commands.
Do not put `--target` in the command when using the `target` key. Profiles are
ordered lists of lint step names; they cannot contain other profiles, and a
profile name cannot shadow a step name or `all`.

Lint steps also accept `env`, `env_paths`, and `cwd`, with the same meaning as
those keys on test command selectors. `env` values are passed verbatim;
relative `env_paths` values resolve from the loaded config file's directory,
independent of step cwd, without normalizing separators/`..` or resolving
symlinks. Native absolute values pass through; paths need not exist.
`cwd` is a non-empty path relative to the repository root, without `..`
components. A key cannot be declared in both environment tables.
Other undeclared lint-step keys remain configuration errors.

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

Lint rows may also declare `host = "linux" | "macos" | "windows"` independently
of target `platform`. A no-target row's declared `host` and `platform` must
agree. Selected rows validate before scope filtering; scope runs before host
eligibility, then the actual host determines native cargo versus xwin.
`host_mismatch` skips neither fail the run nor block an `after_all` barrier.
Only rows surviving scope appear in output.

`default_lint_profile` must name a profile in `[tool.sc-lint.lint_profiles]`;
it is resolved only for a bare consumer lint run. Scope-only selection still
uses that default. Explicit selectors override it, including `all`;
`sc-lint test` is unaffected. With the managed Justfile:

```sh
just lint          # configured default, or every row if no default
just lint "" all   # configured default with all platform scopes
just lint ci all   # explicit ci profile with all platform scopes
```

`host`, `env_paths`, and `default_lint_profile` require sc-lint 0.8.4 or newer;
existing configs keep their behavior. See [Configuration](./configuration.md)
for syntax and [Troubleshooting](./troubleshooting.md#consumer-lint-configuration)
for the new validation codes.

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

The supported Python invocations for direct manifest, cargo-shear, and spelling
checks use the managed environment:

```sh
.sc-lint/venv/bin/python -m sc_lint.lint_manifests
.sc-lint/venv/bin/python -m sc_lint.lint_cargo_shear
.sc-lint/venv/bin/python -m sc_lint.lint_codespell
```

The kit does not add `setup.pre` or a `runner.python` configuration hook.

## Test selectors

`[tool.sc-lint.test]` is either the `[[tool.sc-lint.test]]` row list (every row
runs when no name, or `all`, is given; a name runs that row) or a table of
named selectors. A selector is a `command` argv, or a `group` of other selector
names expanded in listed order (nested groups expand in place; a command
reached twice runs once). `default` names the selector a bare `sc-lint test`
runs; any other selector runs only when named.

```toml
[tool.sc-lint.test]
default = "unit"

[tool.sc-lint.test.selectors.unit]
command = ["cargo", "test"]
env = { RUST_LOG = "debug" }
cwd = "crates/app"
platforms = ["linux", "macos"]

[tool.sc-lint.test.selectors.integration]
command = ["cargo", "test", "--test", "integration"]

[tool.sc-lint.test.selectors.full]
group = ["unit", "integration"]
```

`env` adds literal string variables to the inherited environment; `env_paths`
adds path values resolved from the config file's directory as described above.
Only command selectors accept these fields; groups reject them, and no test
selector accepts `host`. `cwd` is relative to
the repository root, and `platforms` (absent or empty means every platform)
skips the selector on other hosts with a `platform_mismatch` row. `after_all`
is accepted on command selectors only. Selector names are listed sorted by
name.

```sh
sc-lint test --config sc-lint.toml --list
sc-lint test --config sc-lint.toml integration --nocapture
just test --fail-fast full
```

Flags (`--fail-fast`, `--list`, `--config`, `--json`) precede the selector;
every word after the selector is appended verbatim to its command, so
`integration --nocapture` runs `cargo test --test integration --nocapture`.
A group selector does not accept forwarded arguments. `--list` prints the
declared selectors and runs nothing; it cannot be combined with a selector or
`--fail-fast`. An unknown name fails with `test.unknown_selector`, a group
cycle with `test.group_cycle`, and `after_all` on a group with
`profile.invalid_after_all`.

## Inspecting output

Lint rows declaring `host` include actual-host `host` and configured
`host_selector` on Run and skip rows. Untagged Run rows keep their existing
shape; untagged cross skips retain their actual-host `host`. A lint row adds
`env_path_keys` only when `env_paths` was declared (including an empty table),
and never adds `env_keys`. Test-selector listings retain literal `env_keys`
and add a separate `env_path_keys`. Both contain configured key names only;
resolved path values never appear in reports.

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
