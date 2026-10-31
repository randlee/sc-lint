# sc-lint

## Purpose and ownership

`sc-lint` is the product-owned CLI and the only consumer-facing orchestration
entry point. It owns compatibility preflight, installation, upgrade,
initialization, documentation discovery, profiles, and normalized diagnostics.

## Intended users

Consumer developers, release engineers, CI jobs, and coding agents use the
installed binary. Source maintainers may also use it through the repository's
aggregate Just recipes.

## Configuration and inputs

Read `[tool.sc-lint].minimum_version` and the named `lint`/`test` argv arrays
from `sc-lint.toml`. Global flags include `--config`, `--root`, `--json`, and
logging controls. Consumer profiles require explicit `--config sc-lint.toml`.

## Command surface

```sh
sc-lint init --just
sc-lint setup
sc-lint lint ci --consumer --config sc-lint.toml --profile ci
sc-lint test --config sc-lint.toml
sc-lint upgrade --check --config sc-lint.toml
sc-lint docs
```

The consumer lint selector, named profiles, and platform scope are described
in [using sc-lint](../using-sc-lint.md). The `ci` and `full` source-maintainer
profiles include the runtime analyzer. Consumer test steps remain consumer
declared; `sc-lint test` runs the default selector (every row of a
`[[tool.sc-lint.test]]` list) or the named selector, with forwarded arguments.
Steps run in parallel (cargo steps serialized in declaration order) and fail
slow, reporting every failed step; `--fail-fast` stops starting new cargo-lane
and `after_all` steps after the first failure.
Lint steps accept `env` and `cwd` just like test command selectors. `env`
values reach the child verbatim and `cwd` is relative to the repository root.
With `--log-root`, every executed step writes stdout and stderr
under `<log-root>/steps/`; both streams are shown when a step fails.

## Output and CI

Human output is operator-oriented. `--json` emits the stable success/error
envelope with a command identifier, status, details, recovery action, and docs
reference. A lint-fail verdict exits 10 while retaining the report. CI should
run the same `just lint` and `just test` contract.

## Common failures

Compatibility and installer codes are mapped in
[troubleshooting](../troubleshooting.md). Missing documentation is
`CLI.SC_LINT_DOCS_UNAVAILABLE`; run `sc-lint docs --path` to inspect the bundle.

## Related packages

[Boundary](./sc-lint-boundary.md), [portability](./sc-lint-portability.md),
[runtime](./sc-lint-runtime.md), and [schema](./sc-lint-schema.md) provide the
analyzer and output layers used by this CLI.
