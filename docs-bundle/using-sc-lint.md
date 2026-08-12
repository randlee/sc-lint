# Using sc-lint

The installed product owns consumer lint, test, setup, upgrade, and diagnostic
behavior. `just` is a thin memorable interface over these commands.

## First use

From a consumer repository, generate the managed integration once:

```sh
sc-lint init --just
just setup
```

This creates only `sc-lint.toml`, `Justfile`, and `.sc-lint/bootstrap`.

## Daily commands

```sh
just lint
just test
```

`just lint` runs the complete configured lint profile and `just test` runs the
complete configured test profile. Both perform the same minimum-version
preflight before starting work. A preflight failure stops before any backend
command runs.

Direct product equivalents are useful in automation:

```sh
sc-lint lint ci --consumer --config sc-lint.toml
sc-lint test --config sc-lint.toml
```

## Inspecting output

Add `--json` to receive the stable success/error envelope. Human output names
the command, stable code, cause, suggested recovery, and documentation guide.
Use `sc-lint docs troubleshooting` to map a code to the next action.

## Source maintainers versus consumers

Source maintainers use the repository's `just lint` and `just test` aggregate
recipes, which include source-only checks. Consumers use the generated thin
Justfile and never need to know analyzer package names or the source checkout.
