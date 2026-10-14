# `sc-lint-py` Architecture

## Role

`sc-lint-py` builds the version-matched Python wheel runtime. Its small native
extension exposes the existing CLI through `version_json()` and `run(argv)`;
the Python package carries consumer-run helpers without adding lint or
configuration policy.

## Governing ADRs

- [ADR-012 — Consumer adoption and Just contract](../sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md)
- [ADR-015 — Standard repo-tools adoption kit](../sc-lint/adr/ADR-015-standard-repo-tools-adoption-kit.md)
- [ADR-016 — Python wheel runtime and no Rust configuration](../sc-lint/adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md)

## Boundary Rules

- Keep the native bridge thin and limited to existing CLI behavior.
- Keep installation, templating, repository facts, and consumer scaffolding
  in Python and declarative assets.
- Ship consumer-run helpers from the version-matched wheel, not a source
  checkout.

## Related Docs

- [requirements.md](./requirements.md)
- [Python bindings contract](../sc-lint/python-bindings.md)
- [crate README](../../bindings/sc-lint-py/README.md)
