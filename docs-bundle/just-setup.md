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
default: lint

bootstrap_command := if os_family() == "windows" { "& .\\.sc-lint\\bootstrap.ps1" } else { ".sc-lint/bootstrap" }

[private]
_ensure-sc-lint:
    {{bootstrap_command}} ensure --config sc-lint.toml

setup: _ensure-sc-lint
    {{bootstrap_command}} setup --config sc-lint.toml

lint: _ensure-sc-lint
    sc-lint lint --consumer --config sc-lint.toml ci

test: _ensure-sc-lint
    sc-lint test --config sc-lint.toml

upgrade: _ensure-sc-lint
    {{bootstrap_command}} upgrade --config sc-lint.toml
```

The `bootstrap_command` expression dispatches to the product-owned PowerShell
companion on Windows and to the POSIX helper elsewhere.

`setup` installs the configured floor, `lint` and `test` preflight before
running complete profiles, and `upgrade` safely moves the managed installation
forward. The private preflight is shared by every work recipe. When a binary is
too old, `ensure` delegates to `sc-lint setup`; if no compatible release is
available it preserves the structured installer recovery instead of running a
profile.

## Consumer and source-maintainer modes

Consumers run the generated file from their repository root. Source maintainers
use this repository's root `Justfile` as the executable reference model: its
public `setup`, `lint`, `test`, and `upgrade` recipes use the same private
product compatibility preflight, while its complete source-maintainer profiles
stay behind the product command boundary. The consumer template remains
product-owned and must not invoke Cargo analyzer packages directly.

## One-command path

For a new consumer repository, run `sc-lint init --just` and then `just setup`.
The bootstrap helper delegates to the installed product and supports only
`ensure`, `setup`, and `upgrade`.
