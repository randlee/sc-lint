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

```sh
tar -xzf sc-lint_0.7.3_x86_64-unknown-linux-gnu.tar.gz
install -d /usr/local/bin /usr/local/share/sc-lint
install -m 0755 sc-lint sc-lint-boundary sc-lint-portability sc-lint-runtime /usr/local/bin/
cp -R share/sc-lint/sc-lint-docs /usr/local/share/sc-lint/
sc-lint docs --path
```

Never replace a working installation with an unverified archive. The
product-owned `sc-lint setup` and `sc-lint upgrade` paths perform checksum and
post-install verification for consumer repositories.

## CI

CI should install one pinned release, run `sc-lint --json version`, and then
use the same commands as local development. The repository's
`.github/actions/setup-sc-lint` action is a repository workflow component; it
is not a published reusable consumer action. Consumer workflows can install a
verified release archive or use the generated Just integration. Do not run
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
