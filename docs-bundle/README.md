# sc-lint operator manual

This is the offline, version-matched documentation bundle shipped with the
`sc-lint` release. It is usable without a source checkout or network access.
Start with [installation](./installation.md), then follow the
[Just setup guide](./just-setup.md) for the recommended consumer workflow.

## Operator guides

- [Installation](./installation.md) — Homebrew, release archives, and CI.
- [Using sc-lint](./using-sc-lint.md) — setup, lint, test, and daily commands.
- [Configuration](./configuration.md) — `sc-lint.toml`, profiles, and policy.
- [Just setup](./just-setup.md) — the canonical copyable consumer integration.
- [CI](./ci.md) — reproducible checks in continuous integration.
- [Upgrade](./upgrade.md) — safe version upgrades and rollback behavior.
- [Troubleshooting](./troubleshooting.md) — stable error codes and recovery.
- [Best practices](./best-practices.md) — agent, developer, and CI workflow.

## Package guides

Every published package has a guide, including library-only packages:

- [sc-lint](./packages/sc-lint.md)
- [sc-lint-attributes](./packages/sc-lint-attributes.md)
- [sc-lint-analyzer-support](./packages/sc-lint-analyzer-support.md)
- [sc-lint-boundary](./packages/sc-lint-boundary.md)
- [sc-lint-directives](./packages/sc-lint-directives.md)
- [sc-lint-portability](./packages/sc-lint-portability.md)
- [sc-lint-runtime](./packages/sc-lint-runtime.md)
- [sc-lint-schema](./packages/sc-lint-schema.md)

Use `sc-lint docs` to list this bundle, `sc-lint docs just-setup` to print a
guide, and `sc-lint docs --path` when an automation tool needs the installed
bundle directory.
