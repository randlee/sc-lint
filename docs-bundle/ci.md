# Continuous integration

Use the released reusable Action for consumer CI. It selects the verified
release from `[tool.sc-lint].minimum_version` in `sc-lint.toml`, verifies the
matching SHA-256 manifest, runs compatibility preflight, and exposes the
shipped offline documentation bundle. It does not install Cargo packages,
clone tooling, copy scripts, or use a package-manager fallback.

```yaml
name: sc-lint

on: [pull_request]

permissions:
  contents: read

jobs:
  sc-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: randlee/sc-lint@v1
        with:
          operation: setup
          config-path: sc-lint.toml
      - uses: randlee/sc-lint@v1
        with:
          operation: lint
          config-path: sc-lint.toml
      - uses: randlee/sc-lint@v1
        with:
          operation: test
          config-path: sc-lint.toml
```

The Action's optional `version` input is an assertion only. When supplied, it
must semantically equal the configured `minimum_version`; it cannot select a
different release. Pin `randlee/sc-lint` to a reviewed full commit SHA when
your assurance policy requires an immutable Action implementation.

## Consumer command contract

The managed local contract has exactly four commands:

```text
just setup
just lint
just test
just upgrade
```

`setup` and `upgrade` acquire the configured verified release; `lint` and
`test` run the complete consumer profiles. CI uses the released Action above;
the same configuration and profile contract apply locally.

## Offline and failure handling

For an internal verified mirror, provide both `artifact-url` and
`checksums-url`; the archive name and checksum entry must still match the
configured release. The Action publishes `binary-path`, `docs-path`, and
`version` outputs, and validates `sc-lint docs --path` after extraction. Keep
stderr in CI logs so its stable artifact, checksum, compatibility, or command
error code and recovery guidance remain available. See
[troubleshooting](./troubleshooting.md) for recovery details.
