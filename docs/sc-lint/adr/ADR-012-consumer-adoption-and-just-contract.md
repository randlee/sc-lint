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
   delegates to one product-owned bootstrap resolver.
3. The generated resolver calls the product-owned
   `.sc-lint/bootstrap <operation> --config sc-lint.toml` on POSIX and the
   product-owned `.sc-lint/bootstrap.ps1` companion on Windows. These are the
   only generated executable helpers. They resolve `SC_LINT_BIN`, the managed
   binary, and `PATH` consistently; when none exists, `setup` installs the
   verified configured release before product work starts.
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

```just
set windows-shell := ["pwsh", "-NoLogo", "-Command"]

default: lint

# Invoke the Windows bootstrap from Just's PowerShell command wrapper and
# explicitly return its native exit code, preserving typed CLI exits such as
# usage=2 and config=3 instead of collapsing them into recipe failure (1).
bootstrap_command := if os_family() == "windows" { "& .\\.sc-lint\\bootstrap.ps1" } else { ".sc-lint/bootstrap" }
bootstrap_exit := if os_family() == "windows" { "; exit $LASTEXITCODE" } else { "" }

setup:
    {{bootstrap_command}} setup --config sc-lint.toml{{bootstrap_exit}}

lint *profile:
    {{bootstrap_command}} lint --config sc-lint.toml {{profile}}{{bootstrap_exit}}

test *layer:
    {{bootstrap_command}} test --config sc-lint.toml {{layer}}{{bootstrap_exit}}

upgrade:
    {{bootstrap_command}} upgrade --config sc-lint.toml{{bootstrap_exit}}
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

## Amendment 2026-10-10 — the consumer owns its `just` recipes

| Field | Value |
| --- | --- |
| Status | Accepted (Rand, 2026-10-10) |
| Decider | Rand |

Rand's rulings, verbatim: "sc-lint does not 'own' just test. sc-lint will have
a recommended 'just test' implementation AND sc-lint should have a 'default'
'just' implementation it can install on a new repo." and "sc-lint doesn't own
'just', it has best practices and recommendations for naming conventions that
we want enforced across all repos. 'just' is fully working on 10-20 repos
today."

1. Replaces "complete test" in Decision 1: installed `sc-lint` owns setup,
   compatibility, complete lint and upgrade behaviour. The consumer repository
   owns its `Justfile`, including `just test`; sc-lint provides `sc-lint test`,
   a recommended `test` recipe and the recipe naming conventions.
2. Replaces Decision 2 for the adoption kit: the imported kit justfile defines
   `setup`, `lint` and `upgrade` only. The installer adds the recommended `test`
   recipe outside the managed block when the consumer `Justfile` has none, and a
   repository with no `Justfile` starts from the default one (`set
   windows-shell`, `default: lint`). An existing `test` recipe is never changed.
3. The Canonical Generated Template above stays the `sc-lint init --just`
   output.
