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

The command manages `sc-lint.toml`, `Justfile`, `.sc-lint/bootstrap`, and the
Windows companion `.sc-lint/bootstrap.ps1` only.
Conflicting user-owned files produce a structured error instead of an
overwrite.

## Public recipes

The generated file has exactly these public entry points:

```just
set windows-shell := ["pwsh", "-NoLogo", "-Command"]

default: lint

bootstrap_command := if os_family() == "windows" { "& .\\.sc-lint\\bootstrap.ps1" } else { ".sc-lint/bootstrap" }

setup:
    {{bootstrap_command}} setup --config sc-lint.toml

lint:
    {{bootstrap_command}} lint --config sc-lint.toml

test:
    {{bootstrap_command}} test --config sc-lint.toml

upgrade:
    {{bootstrap_command}} upgrade --config sc-lint.toml
```

The `bootstrap_command` expression dispatches to the product-owned PowerShell
companion on Windows and to the POSIX helper elsewhere.

`setup` installs the configured floor when no product binary is present, then
preflights it. `lint` and `test` use the same resolver before running complete
profiles, and `upgrade` safely moves the managed installation forward. A
configured `SC_LINT_BIN`, managed installation, or `PATH` binary is always
used consistently for every operation. When none is available, the helper
downloads the configured release archive and verifies its SHA-256 manifest
entry before activation; it never runs a profile before that succeeds.

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
helper supports `setup`, `lint`, `test`, and `upgrade`.
