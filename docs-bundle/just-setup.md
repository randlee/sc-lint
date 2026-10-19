# Canonical Just setup

This document is the source model for the generated consumer `Justfile`.
`sc-lint init --just` materializes the same contract; it does not copy Python
runner scripts or overwrite a consumer README.

## Generate the integration

```sh
sc-lint init --just --check   # inspect drift without writing
sc-lint init --just --dry-run # show managed paths without writing
sc-lint init --just           # create or update managed files
```

The command manages `sc-lint.toml`, `Justfile`, and `.sc-lint/bootstrap.py` only.
Conflicting user-owned files produce a structured error instead of an
overwrite.
The installer also rejects unresolved template variables before writing;
details are in [Best practices](./best-practices.md#template-rendering).

## Public recipes

The generated file has exactly these public entry points:

```just
set windows-shell := ["cmd.exe", "/c"]

python := if os_family() == "windows" { "python" } else { "python3" }

default: lint

setup:
    {{python}} .sc-lint/bootstrap.py setup --config sc-lint.toml

lint *profile:
    {{python}} .sc-lint/bootstrap.py lint --config sc-lint.toml {{profile}}

test *layer:
    {{python}} .sc-lint/bootstrap.py test --config sc-lint.toml {{layer}}

upgrade:
    {{python}} .sc-lint/bootstrap.py upgrade --config sc-lint.toml
```

Every platform runs the same Python bootstrap; Python 3.11 or newer is required.

## Select lint steps and run managed commands

`just lint` accepts a step name, named profile, or `all` as its first
argument. An optional second argument selects platform scope. For example,
`just lint ci windows` selects the `ci` profile's Windows steps. The supported
scopes are `host` (the default), `all`, `linux`, `macos`, and `windows`; a scope
needs a selector before it. The equivalent direct helper form is:

```sh
python .sc-lint/bootstrap.py lint ci windows --config sc-lint.toml
```

Use `exec` to run a command with the managed product environment while
preserving its arguments and exit status:

```sh
python .sc-lint/bootstrap.py exec -- cargo --version
```

An alternate configuration can be named before the `--` separator, as in
`python .sc-lint/bootstrap.py exec --config config/sc-lint.toml -- cargo --version`.
`exec` does not run setup or select a lint profile. See
[profiles, platform scope, and declared tools](./using-sc-lint.md#profiles-platform-scope-and-declared-tools)
for step, target, profile, and tool declaration syntax.

`setup` installs the configured floor when no compatible product is present,
then preflights it. The installer stages the complete product bundle, checks
launcher and companion identities/health, activates atomically, and retains
the prior working bundle if post-install verification fails. `lint` and `test`
run declared profiles/layers after compatible-product preflight; `upgrade`
uses the same verified install and rollback path. When no compatible product
is available, bootstrap downloads the configured release archive and verifies
its checksum before activation; it does not start a profile before setup and
preflight succeed. Configure `SC_LINT_INSTALL_DIR` for a custom managed
location. Use `SC_LINT_BIN` with `exec` when a particular binary is needed.

## Consumer and source-maintainer modes

Consumers run the generated file from their repository root. Source maintainers
use this repository's root `Justfile` as the executable reference model: its
public `setup`, `lint`, `test`, and `upgrade` recipes use the same private
product compatibility preflight, while its complete source-maintainer profiles
stay behind the product command boundary. The consumer template remains
product-owned and must not invoke Cargo analyzer packages directly. The root
model adds `--dry-run` to `just setup` and `just upgrade` so source maintenance
can inspect product installation behavior without mutating a local managed
installation; generated consumer recipes perform the real operation.

## One-command path

For a new consumer repository, install the product once using the supported
package or release archive, run `sc-lint init --just`, and commit the generated
files. On every later clean checkout, `just setup` is the no-prior-binary path:
it downloads and verifies the configured release automatically. The bootstrap
helper supports `setup`, `lint`, `test`, `upgrade`, and `exec`. `exec` uses the
selected managed product and environment for a child command while preserving
its arguments and exit status; it does not recurse into a profile. On Windows,
`.cmd` and `.bat` commands run through the command processor. Provision declared
cargo tools and rustup targets with ordinary `just setup`; profile execution
never installs them. See [using sc-lint](./using-sc-lint.md) for profile and
platform selection.
