# Canonical Just setup

This document is the source model for the generated consumer `Justfile`.
`sc-lint init --just` creates the initial consumer integration files. It does
not copy Python runner scripts or overwrite a consumer README.

## Generate the integration

```sh
sc-lint init --just --check   # inspect drift without writing
sc-lint init --just --dry-run # validate without writing; --json lists paths
sc-lint init --just           # create missing files; refresh a recognized older bootstrap
```

Human output starts with `init: <status>`:

| Status | Meaning |
| --- | --- |
| `current` | A non-mutating check found no drift. |
| `created` | Only missing files were written. |
| `updated` | A recognized prior-release bootstrap was refreshed, with or without missing files created. |
| `would_create` | `--dry-run`: only missing files would be written. |
| `would_update` | `--dry-run`: a recognized prior-release bootstrap would be refreshed. |

Non-empty file lists follow, one line each, in this order: `Created:` (or
`Would create:` for `--dry-run`), `Refreshed:` (or `Would refresh:` for
`--dry-run`) and `Preserved:`. Only a bootstrap recognized as coming from a
prior release can be refreshed. Existing `sc-lint.toml` and `Justfile` files
are preserved exactly as they are. A different existing bootstrap is not
overwritten; initialization reports a conflict. For example:

```text
init: created
Created: /work/repo/Justfile, /work/repo/.sc-lint/bootstrap.py
Preserved: /work/repo/sc-lint.toml
```

Paths are printed as the repository root joined with each file name.
Refreshing a recognized prior-release bootstrap can print, for example:

```text
init: updated
Refreshed: /work/repo/.sc-lint/bootstrap.py
Preserved: /work/repo/sc-lint.toml, /work/repo/Justfile
```

`sc-lint --json init --just` reports these eight data fields: `status`,
`managed_files`, `preserved_files`, `created_files`, `refreshed_files`,
`check`, `dry_run` and `summary`. The file-list
fields are always arrays. `refreshed_files` contains files that were refreshed
on a real run or would be refreshed on `--dry-run`; `dry_run` and `status`
distinguish the two cases.

`--check` reports missing files and a recognized prior-release bootstrap as
outdated. `--dry-run` shows which files would be created or refreshed. A
bootstrap with an unrecognized content change is not overwritten; move it
aside or restore the expected content before retrying initialization.

## Public recipes

The generated file has exactly these public entry points:

```just
set windows-shell := ["cmd.exe", "/c"]

python := if os_family() == "windows" { "python" } else { "python3" }

default: lint

setup:
    {{python}} .sc-lint/bootstrap.py setup --config sc-lint.toml

lint selector='' *scope:
    {{python}} .sc-lint/bootstrap.py lint --config sc-lint.toml "{{selector}}" {{scope}}

test *layer:
    {{python}} .sc-lint/bootstrap.py test --config sc-lint.toml {{layer}}

upgrade:
    {{python}} .sc-lint/bootstrap.py upgrade --config sc-lint.toml
```

Every platform runs the same Python bootstrap; Python 3.9 or newer is required.

## Select lint steps and run managed commands

`just lint` accepts a step name, named profile, or `all` as its first
argument. An optional second argument selects platform scope. For example,
`just lint ci windows` selects the `ci` profile's Windows steps. The supported
scopes are `host` (the default), `all`, `linux`, `macos`, and `windows`. To use
a scope with the configured default profile, leave the selector slot empty:
`just lint "" all`. The equivalent direct helper form is:

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
stay behind the product command boundary. The consumer owns the generated
Justfile and may change its recipes; the product owns the bootstrap. The root
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

Lint, test and exec child commands inherit PATH in this order: the installed
archive's bin directory, the managed `.sc-lint/venv/bin` directory (or
`.sc-lint/venv/Scripts` on Windows), then the host PATH. Thus consumer steps
using `python3 -m sc_lint.<helper>` on Unix or `python -m sc_lint.<helper>` on
Windows use the managed interpreter. The venv is relative to the selected
config file's directory. Exec uses the existing environment without provisioning it.
