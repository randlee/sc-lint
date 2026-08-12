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

The formula installs the executable in `bin` and this bundle in its formula
`pkgshare`; documentation remains local and offline.

## Release archive

Download the archive matching the host triple from the project release page,
verify its published checksum, and put `sc-lint` on `PATH`. The archive's
`sc-lint-docs/` directory must stay beside the executable (or be installed in
the platform's `share/sc-lint/docs-bundle` directory).

```sh
tar -xzf sc-lint_0.4.0_x86_64-unknown-linux-gnu.tar.gz
install -m 0755 sc-lint /usr/local/bin/sc-lint
sc-lint docs --path
```

Never replace a working installation with an unverified archive. The
product-owned `sc-lint setup` and `sc-lint upgrade` paths perform checksum and
post-install verification for consumer repositories.

## CI and GitHub Action

CI should install one pinned release, run `sc-lint --json version`, and then
use the same commands as local development. A workflow may use the reusable
project Action once E.6 publishes it, or install the release archive directly.
Do not run `cargo run -p sc-lint` in a consumer CI job.

## Verify an installation

```sh
sc-lint --json version
sc-lint docs
sc-lint docs --path
```

If the docs path is missing, the command returns a structured
`CLI.SC_LINT_DOCS_UNAVAILABLE` error with the searched bundle path and a
recovery reference to `sc-lint docs installation`.
