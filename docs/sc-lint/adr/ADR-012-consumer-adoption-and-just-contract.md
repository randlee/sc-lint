# ADR-012 — Consumer Adoption And Just Contract

| Field | Value |
| --- | --- |
| ID | ADR-012 |
| Status | Accepted |
| Date | 2026-08-11 |
| Deciders | team-lead, clint |

## Context

Consumer repositories need a small, memorable interface that agents and
developers can run without knowing this repository's Cargo package topology,
Python adapters, or source-maintainer-only checks. Earlier source-versus-
consumer runner work in parked PR #87 did not establish a product-owned
contract and is not an input to this decision.

## Decision

1. Installed `sc-lint` owns consumer setup, compatibility, complete lint,
   complete test, and upgrade behavior. Consumer-facing commands never use
   `cargo run -p`, an analyzer package name, or copied `.just/*.py` scripts.
2. `just` is the thin consumer interface. A generated consumer `Justfile` has
   exactly four public recipes: `setup`, `lint`, `test`, and `upgrade`; each
   depends on the same private `_ensure-sc-lint` preflight.
3. The generated preflight calls the product-owned
   `.sc-lint/bootstrap ensure --config sc-lint.toml`. The bootstrap asset is
   the only generated executable helper and delegates to installed `sc-lint`.
4. `sc-lint init --just` is the one-command consumer integration path. It
   creates or updates only `sc-lint.toml`, `Justfile`, and
   `.sc-lint/bootstrap`; it never writes a consumer `README.md` and reports a
   conflict rather than overwriting a user-owned integration file.
5. `--check` and `--dry-run` are non-mutating. Re-running a current generated
   integration is idempotent and reports the managed files.
6. Source-maintainer recipes remain in this repository's root `Justfile` and
   retain all development gates. Source versus consumer behavior is selected
   by explicit generated integration, never inferred from a directory name,
   Cargo manifest, or a backend package.

## Canonical Generated Template

```just
default: lint

[private]
_ensure-sc-lint:
    .sc-lint/bootstrap ensure --config sc-lint.toml

setup: _ensure-sc-lint
    .sc-lint/bootstrap setup --config sc-lint.toml

lint: _ensure-sc-lint
    sc-lint lint ci --config sc-lint.toml

test: _ensure-sc-lint
    sc-lint test --config sc-lint.toml

upgrade: _ensure-sc-lint
    .sc-lint/bootstrap upgrade --config sc-lint.toml
```

## Consequences

- missing or incompatible installations stop `lint` and `test` before work
  begins, through the E.1 structured recovery contract.
- E.2 owns downloading, checksum verification, atomic replacement, and the
  bootstrap implementation; E.3 only renders its product-owned asset.
- E.4 owns the offline documentation bundle and E.5 owns package/release
  distribution. This decision only records their discovery references.
- `just lint` and `just test` in consumer repositories always mean complete
  configured profiles, never an advisory subset.
