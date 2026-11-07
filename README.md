# sc-lint

`sc-lint` checks Rust repositories for boundary, portability, runtime, and
repository-policy problems. The Rust CLI is the product command surface; the
separately published Python distribution provides bindings and repository
helper modules.

Current release: **0.8.6**.

## Install

Install the CLI and its analyzer binaries with Homebrew:

```bash
brew install randlee/tap/sc-lint
```

Release archives are available from
[GitHub Releases](https://github.com/randlee/sc-lint/releases). The `sc-lint`
crate page is [on crates.io](https://crates.io/crates/sc-lint); crate API docs
are on [docs.rs](https://docs.rs/sc-lint/0.8.6).

The Python package is installed separately:

```bash
python -m pip install sc-lint
```

It does not install the Rust CLI binaries. Install the CLI from Homebrew or a
release archive before using Python helpers that locate or invoke `sc-lint`.
See the [Python bindings guide](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/python-bindings.md).

Other supported distribution and integration channels are
[Scoop](https://github.com/randlee/scoop-bucket),
[WinGet](https://github.com/randlee/sc-lint/blob/main/docs/WINGET_SETUP.md),
the [GitHub Action](https://github.com/randlee/sc-lint/blob/main/action.yml),
and the Rust crates on [crates.io](https://crates.io/crates/sc-lint). The
Python package supplies bindings and helpers; it is not the CLI installer.

## Run checks

With a configured `sc-lint.toml`, run the consumer CI lint profile for a
consumer repository with:

```bash
sc-lint lint ci --consumer
```

Bare `sc-lint lint ci` runs sc-lint's own maintainer profile.

Run an individual analyzer with:

```bash
sc-lint lint sc-boundary
sc-lint lint sc-portability
sc-lint lint sc-runtime
```

Use `sc-lint --json lint ci --consumer` for machine-readable output. The offline manual
is available with `sc-lint docs`; start with its `installation`,
`configuration`, `using-sc-lint`, and package guides.

The analyzers can also be run directly when working on their Rust crates:

```bash
cargo run -p sc-lint-boundary -- analyze --root . --format text
cargo run -p sc-lint-portability -- analyze --root . --format text
cargo run -p sc-lint-runtime -- analyze --root . --format text
```

## Analyzer rules

The rule inventories below match the 0.8.6 analyzer crates. Full descriptions,
configuration, and examples are in each linked crate guide.

### Boundary

[`sc-lint-boundary`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-boundary/README.md)
checks graph cycles, source boundaries, dependency policy, features, and
manifests. Its rules are:

- `SCB-CYCLE-001`–`SCB-CYCLE-003`
- `SCB-BOUNDARY-001`–`SCB-BOUNDARY-005`
- `SCB-CALLER-001`
- `SCB-DEPENDENCY-001`–`SCB-DEPENDENCY-004`
- `SCB-FEATURE-001`
- `SCB-MANIFEST-001`–`SCB-MANIFEST-006`

### Portability

[`sc-lint-portability`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-portability/README.md)
checks cross-platform paths, environment access, Unix shell assumptions, and
`cfg` parity:

- `PORT-001`–`PORT-010`

### Runtime

[`sc-lint-runtime`](https://github.com/randlee/sc-lint/blob/main/crates/sc-lint-runtime/README.md)
checks blocking waits, sleep calls, `block_on`, and mutex guards held across
`.await`:

- `SCB-RUNTIME-001`–`SCB-RUNTIME-007`

## Packages

The workspace version is **0.8.6**. The `sc-lint` CLI composes the analyzer
crates; the support crates are reusable building blocks.

| Package | Role |
|---|---|
| [`sc-lint`](https://crates.io/crates/sc-lint) | CLI and product command surface |
| [`sc-lint-boundary`](https://crates.io/crates/sc-lint-boundary) | Boundary analyzer |
| [`sc-lint-portability`](https://crates.io/crates/sc-lint-portability) | Portability analyzer |
| [`sc-lint-runtime`](https://crates.io/crates/sc-lint-runtime) | Runtime analyzer |
| [`sc-lint-analyzer-support`](https://crates.io/crates/sc-lint-analyzer-support) | Shared analyzer implementation support |
| [`sc-lint-attributes`](https://crates.io/crates/sc-lint-attributes) | `#[sc_lint(...)]` procedural attribute |
| [`sc-lint-directives`](https://crates.io/crates/sc-lint-directives) | Shared directive parser and types |
| [`sc-lint-schema`](https://crates.io/crates/sc-lint-schema) | Analyzer report and finding types |
| [`sc-lint` on PyPI](https://pypi.org/project/sc-lint/) | Python bindings and repository helpers |

## Project links

- [Offline manual source](https://github.com/randlee/sc-lint/tree/main/docs-bundle)
- [Architecture and requirements](https://github.com/randlee/sc-lint/tree/main/docs/sc-lint)
- [Issue tracker](https://github.com/randlee/sc-lint/issues)
- [Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md)
- [MIT license](https://github.com/randlee/sc-lint/blob/main/LICENSE)

## Contributing

From a source checkout, run the complete repository gates:

```bash
just lint
just test
```

Before running `cargo test` directly in a fresh checkout, run `just setup` to
build the local wheels required by the consumer fixture.
