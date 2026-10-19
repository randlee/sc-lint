# Installation

`sc-lint` is the installed product. Consumers do not need to select a Cargo
package or copy analyzer scripts.

## Homebrew

The supported macOS/Linux Homebrew path is the `randlee/tap/sc-lint` formula:

```sh
brew tap randlee/tap
brew install randlee/tap/sc-lint
sc-lint --version
sc-lint docs
```

The formula installs executables in `bin` and this bundle as
`<prefix>/share/sc-lint/sc-lint-docs` (the formula-owned `pkgshare`);
documentation remains local and offline.

## Release archive

Download the archive matching the host triple from the project release page,
verify its published checksum, and put `sc-lint` on `PATH`. Keep the launcher
and three analyzer companions together, and preserve the archive's declared
`share/sc-lint/sc-lint-docs/` path for the offline manual. Managed setup
installs and checks the launcher, companions, and documentation bundle as one
complete unit before activation.
The [upgrade guide](./upgrade.md) explains health probes and rollback behavior.

```sh
tar -xzf sc-lint_0.8.0_x86_64-unknown-linux-gnu.tar.gz
cd sc-lint_0.8.0_x86_64-unknown-linux-gnu
install -d /usr/local/bin /usr/local/share/sc-lint
install -m 0755 bin/sc-lint bin/sc-lint-boundary bin/sc-lint-portability bin/sc-lint-runtime /usr/local/bin/
cp -R share/sc-lint/sc-lint-docs /usr/local/share/sc-lint/
sc-lint docs --path
```

Never replace a working installation with an unverified archive. The
product-owned `sc-lint setup` and `sc-lint upgrade` paths perform checksum and
post-install verification for consumer repositories.
The bootstrap's explicit, managed, and `PATH` binary selection is described in
[the best-practices guide](./best-practices.md#binary-resolution-and-path-fallback).

## CI

CI should install one pinned release, run `sc-lint --json version`, and then
use the same commands as local development. Consumer workflows can use the
shipped reusable Action, `randlee/sc-lint@v1`, with an explicit `version` input
(see [CI](./ci.md#reusable-github-action)); install a verified release archive;
or use the generated Just integration. The repository's `.github/actions/*`
composites are repository-only and are not consumer actions. Do not run
`cargo run -p sc-lint` in a consumer CI job.

## Verify an installation

```sh
sc-lint --json version
sc-lint docs
sc-lint docs --path
```

If the docs path is missing, the command returns a structured
`CLI.SC_LINT_DOCS_UNAVAILABLE` error with the searched bundle path and a
recovery reference to `sc-lint docs installation`.
