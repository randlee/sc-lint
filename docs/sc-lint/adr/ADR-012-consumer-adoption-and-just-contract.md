# ADR-012 — Consumer Adoption And Just Contract

| Field | Value |
| --- | --- |
| ID | ADR-012 |
| Status | Proposed (amendment 2026-10-10, phase-h) |
| Date | 2026-08-11 |
| Deciders | team-lead, clint |
| Amendment | 2026-10-10 phase-h: item 3 bootstrap argv grammar, `[script]`/`[positional-arguments]` recipes, absolute `SC_LINT_BIN` export and exit-status passthrough, `setup` runs `sc-lint provision`; 0.6.1 canonical template note; awaiting approval in PR #206 |

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
   delegates to one product-owned bootstrap resolver.
3. The generated recipes call the product-owned
   `.sc-lint/bootstrap <operation> --config sc-lint.toml [arguments...]` on
   POSIX and the product-owned `.sc-lint/bootstrap.ps1` companion on Windows,
   as platform-attributed `[script]` recipes, with `[positional-arguments]` on
   `lint` and `test`, so `--`, dash-prefixed tokens and arguments containing
   spaces arrive unchanged. These are the only generated executable helpers.
   They resolve `SC_LINT_BIN`, the managed binary, and `PATH`, in that order,
   and make the result absolute; before `lint` and `test` they export that
   absolute `SC_LINT_BIN` and prepend its directory to `PATH` for the product
   run and every consumer step it starts. When no binary exists, `setup`
   installs the verified configured release before product work starts.
   `setup` then runs `sc-lint provision` (consumer `setup.pre` and declared
   tools) with the same flags. The product exit status, including 7, is
   returned unchanged. (Amended 0.6.1.)
4. `sc-lint init --just` is the one-command consumer integration path. It
   creates or updates only `sc-lint.toml`, `Justfile`, `.sc-lint/bootstrap`,
   and `.sc-lint/bootstrap.ps1`; it never writes a consumer `README.md` and
   reports a conflict rather than overwriting a user-owned integration file.
5. `--check` and `--dry-run` are non-mutating. Re-running a current generated
   integration is idempotent and reports the managed files.
6. Source-maintainer recipes remain in this repository's root `Justfile` and
   retain all development gates. Source versus consumer behavior is selected
   by explicit generated integration, never inferred from a directory name,
   Cargo manifest, or a backend package.
7. E.3 does not generate a consumer CI workflow. E.6 owns that optional
   surface because only its reusable Action can install a verified release and
   make the generated local compatibility contract viable in CI.

## Canonical Generated Template

This block is the 0.6.0 form of the standalone Justfile written by
`sc-lint init --just`, kept for history. Since 0.6.1 the canonical template
is the shipped asset `crates/sc-lint/assets/consumer-Justfile`, mirrored
verbatim in `docs-bundle/just-setup.md`. The adoption kit's imported
`.sc-lint/justfile` is that template without the leading `default: lint`
recipe and declares no settings (ADR-015 Decision 4).

```just
set windows-shell := ["pwsh", "-NoLogo", "-Command"]

default: lint

bootstrap_command := if os_family() == "windows" { "& .\\.sc-lint\\bootstrap.ps1" } else { ".sc-lint/bootstrap" }

setup:
    {{bootstrap_command}} setup --config sc-lint.toml

lint *profile:
    {{bootstrap_command}} lint --config sc-lint.toml {{profile}}

test *layer:
    {{bootstrap_command}} test --config sc-lint.toml {{layer}}

upgrade:
    {{bootstrap_command}} upgrade --config sc-lint.toml
```

Recipe arguments per ADR-016 Decision 3 (2026-08-30).

## Consequences

- missing or incompatible installations stop `lint` and `test` before work
  begins, through the E.1 structured recovery contract.
- E.2 owns downloading, checksum verification, atomic replacement, and the
  bootstrap implementation; E.3 only renders its product-owned asset.
- E.4 owns the offline documentation bundle and E.5 owns package/release
  distribution. This decision only records their discovery references.
- `just lint` and `just test` in consumer repositories always mean complete
  configured profiles, never an advisory subset.
- Consumer mode is encoded in the generated command path: `lint --consumer --config sc-lint.toml ci`
  and `test` read named argv profiles from `sc-lint.toml`; source-maintainer
  `lint ci` remains a separate source-checkout command path.
