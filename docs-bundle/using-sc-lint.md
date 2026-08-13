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
manifest, and installs it in the managed location before invoking the product.
Set `SC_LINT_INSTALL_DIR` to choose that location or `SC_LINT_BIN` to use one
specific compatible binary for all four recipes.

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
sc-lint lint --consumer --config sc-lint.toml ci
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
