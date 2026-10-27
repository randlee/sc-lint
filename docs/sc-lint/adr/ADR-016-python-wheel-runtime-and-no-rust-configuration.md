# ADR-016 — Python Wheel Runtime And No Rust For Configuration

| Field | Value |
| --- | --- |
| ID | ADR-016 |
| Status | Accepted |
| Approved by | Rand, 2026-10-16 |
| Date | 2026-08-29, revised 2026-10-16 |
| Deciders | Rand |
| Related | ADR-012, ADR-015, ADR-017 |

## Context

`sc-lint` is a set of independent lint crates combined only at the CLI. Rust
is the wrong tool for repository configuration, templating and CI wiring: it
is slow to change and couples policy to a release. Consumers that copied
`.just/*.py` helpers from the source archive let their helper set diverge
silently from their pinned binary. `sc-publish` solves the same problem for
`sc-compose` by provisioning a pinned wheel.

## Decision

1. **No Rust for configuration.** Installation, templating, repository facts,
   CI wiring and consumer scaffolding are Python, declarative assets
   (TOML/YAML/Justfile), skills and prompts. A thin maturin bridge may expose
   existing CLI behavior to Python, but it adds no configuration policy or
   repository logic. A `configure`-style module in any crate is a violation.
   An analyzer reading its own `[tool.sc-lint.analyzers.<name>]` table at run
   time is analyzer behavior, not configuration.
2. **The `sc-lint` Python wheel is the runtime for every consumer-run
   helper.** It is built with maturin from `bindings/sc-lint-py/` (pyo3
   cdylib, ≤150 lines of bridge) plus the `sc_lint` Python package, published
   to PyPI through the `sc-publish` channel and pinned by
   `[tool.sc-lint].minimum_version`. Nothing is copied from a source archive.
3. **Kit entry points.** The kit owns the managed import block and its
   `setup`, `lint` and `upgrade` recipes; the consumer owns `test` and every
   recipe outside the block (ADR-012). `just lint [profile]` and
   `just test [layer]` pass an optional positional argument to the
   bootstrap. `sc-lint.toml` declares named `[tool.sc-lint.test.<layer>]` step
   lists and lint profiles; no argument uses the declared default, and
   `just test all` runs every layer in declaration order. New layers,
   profiles and analyzers are configuration, never bootstrap, wheel or Rust
   edits.
4. A **kit-rendered** profile or layer entry in `sc-lint.toml` may reference
   only a shipped binary or a `sc_lint` module, never a repository-relative
   script. Consumer-owned entries may invoke any command, including
   repository-local scripts; the kit never renders, validates or drifts on
   them.
5. **Declared tools.** A consumer may declare exact cargo-tool versions and
   rustup targets in `.sc-lint/install.json`; product defaults choose none.
   Bootstrap `setup` provisions them through `sc_lint.setup_tools`. Dry-run
   never provisions. Consumer lint and test profiles run sequentially, in
   declaration order, fail-fast [superseded for execution order and failure
   handling by ADR-020 decision 7 and REQ-CLI-023 as amended, phase L]. ADR-017 specifies translation, failure and
   compatibility behavior.

The bootstrap (`.sc-lint/bootstrap.py`, ADR-012) provisions `.sc-lint/venv`
with the pinned wheel, runs declared-tool setup and resolves the managed
binary before running the product.

## How To Extend

Use declarative `sc-lint.toml` lint profiles, test layers, per-analyzer
tables, step argv/exclusions, platform conditionals and per-step/per-layer
`env`. Pre/post work is an ordinary extra layer step, not a hook DSL. The kit
pins only `sc-lint`; other toolchain pins remain consumer-owned.
Validate/view/bench/fuzz dispatchers are consumer-owned recipes outside the
managed block. Patterns observed independently in 2+ consumer repositories
may be promoted into the kit (ADR-015). Consumer-owned domain-lint steps may
reference repository-local scripts and are the primary extension point for
repository-specific enforcement.

## Consequences

- Helper and binary versions cannot diverge: both derive from one pin.
- The release archive is self-contained.
- The wheel matrix (Linux/macOS/Windows × supported CPython) is part of the
  release gate; a wheel publish failure blocks the release.
- Rejected: copying `.just/*.py` from the archive (version drift,
  sc-lint#84); helpers as Rust subcommands (moves configuration policy into a
  release cycle, contradicts decision 1); a per-platform bootstrap (cannot be
  verified by one test suite).
