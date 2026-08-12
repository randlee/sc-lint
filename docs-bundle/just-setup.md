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

The command manages `sc-lint.toml`, `Justfile`, and `.sc-lint/bootstrap` only.
Conflicting user-owned files produce a structured error instead of an
overwrite.

## Public recipes

The generated file has exactly these public entry points:

```just
default: lint

[private]
_ensure-sc-lint:
    .sc-lint/bootstrap ensure --config sc-lint.toml

setup: _ensure-sc-lint
    .sc-lint/bootstrap setup --config sc-lint.toml

lint: _ensure-sc-lint
    sc-lint lint ci --consumer --config sc-lint.toml

test: _ensure-sc-lint
    sc-lint test --config sc-lint.toml

upgrade: _ensure-sc-lint
    .sc-lint/bootstrap upgrade --config sc-lint.toml
```

`setup` installs the configured floor, `lint` and `test` preflight before
running complete profiles, and `upgrade` safely moves the managed installation
forward. The private preflight is shared by every work recipe.

## Consumer and source-maintainer modes

Consumers run the generated file from their repository root. Source maintainers
may retain repository-specific checks in the root project's Justfile, but the
consumer template must remain product-owned and must not invoke Cargo analyzer
packages directly.

## One-command path

For a new consumer repository, run `sc-lint init --just` and then `just setup`.
The bootstrap helper delegates to the installed product and supports only
`ensure`, `setup`, and `upgrade`.
