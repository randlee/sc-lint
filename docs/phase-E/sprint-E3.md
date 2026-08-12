---
id: E.3
title: Consumer CLI And Canonical Just Integration
status: implemented
branch: feature/phase-E3-consumer-cli-just
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E3-consumer-cli-just
target: integrate/phase-E
---

# Sprint E.3 — Consumer CLI And Canonical Just Integration

## Goal

Make installed `sc-lint` the sole consumer orchestration surface and generate
a thin, predictable Just interface that agents can use without tool-specific
knowledge.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Relationship To Parked PR #87

E.3 supersedes the parked Python source-versus-consumer runner rework in
PR #87. It neither depends on that PR nor permits copying its runner changes.
The separate cargo-deny CI repair is outside this sprint. E.3 closes the
consumer path only through the explicit product-owned bootstrap and installed
`sc-lint` contract defined here.

## Hard Dependencies

- [Sprint E.1](./sprint-E1.md) compatibility preflight contract
- [Sprint E.2](./sprint-E2.md) installation/upgrade bootstrap engine
- current `Justfile`, `.just/run_lint.py`, and `.just/` adapters
- `crates/sc-lint/src/workflow.rs` profile orchestration
- `crates/sc-lint/src/dispatch.rs` backend resolution

## Exact Targets

- `crates/sc-lint/src/cli.rs`
- `crates/sc-lint/src/command.rs`
- `crates/sc-lint/src/workflow.rs`
- `crates/sc-lint/src/dispatch.rs`
- `crates/sc-lint/src/config.rs`
- consumer templates owned by the product
- `Justfile` and `.just/` only where source-repository maintenance remains
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md`
- `docs/sc-lint/adr/README.md`
- `docs/phase-E/sprint-E3.md`

## Deliverables

- installed `sc-lint` exposes documented consumer operations for setup,
  complete lint, complete test, and compatibility checking. Product commands
  must not depend on a source checkout's `.just` directory.
- `sc-lint init` creates/updates only product-owned integration files:
  `sc-lint.toml` and thin Just integration. The optional consumer CI workflow
  is an E.6 delivery concern because it depends on that sprint's reusable
  release-verified Action contract. It never
  overwrites a repository README. `sc-lint init --just` is the one-command
  consumer setup path; it is idempotent, reports its managed files, and offers
  `--check`/`--dry-run` before modifying an existing repository.
- generated Just integration has four public recipes: `setup`, `lint`, `test`,
  and `upgrade`. Every public recipe depends on the same private preflight.
- `just lint` executes the whole configured lint profile; `just test` executes
  the whole configured test profile. Neither is a fast/advisory alias.
- source checkout recipes retain full source-maintainer checks while consuming
  repositories use installed-product commands; this distinction is explicit in
  templates/command selection, never guessed from file paths.
- remove consumer reliance on `cargo run -p sc-lint-boundary`, copied Python
  runner scripts, and the `crates/sc-lint-boundary/Cargo.toml` mode heuristic.
- a missing installed `sc-lint` or required backend returns the E.1 shared
  structured recovery diagnostic, including install/recovery guidance, rather
  than an exception or traceback.
- ADR-012 records the durable consumer contract: Just is a thin interface,
  public recipes always preflight the tracked floor, installed `sc-lint` owns
  consumer orchestration, source and consumer modes are explicit, and product
  integration never overwrites consumer-owned files. Requirements and
  architecture/CLI docs link to that decision and the generated contract.

## Canonical Generated Template

```just
default: lint

[private]
_ensure-sc-lint:
    .sc-lint/bootstrap ensure --config sc-lint.toml

setup:
    .sc-lint/bootstrap setup --config sc-lint.toml

lint: _ensure-sc-lint
    sc-lint lint ci --consumer --config sc-lint.toml

test: _ensure-sc-lint
    sc-lint test --config sc-lint.toml

upgrade:
    .sc-lint/bootstrap upgrade --config sc-lint.toml
```

The exact command spelling may be refined during implementation, but the four
public recipe names, preflight dependency, and no-Cargo-package consumer
contract are mandatory.

## This Sprint Does Not Close

- installer/bootstrap download, atomic replacement, or upgrade policy; E.2
  owns those behaviors
- documentation bundle content/distribution; E.4/E.5 own the operator manual
  and release packaging
- replacement of every repository-specific policy script that has not yet been
  productized; such scripts must remain explicitly source-local until migrated
- consumer CI workflow generation and installation logic; E.6 owns the
  reusable release-verified Action that makes that workflow viable

## Paths To Delete

- None from this source repository unless a script becomes fully product-owned
  and its source-maintainer replacement lands in the same change.
- `sc-lint init --just` must never create `.just/lint_*.py` copies in consumer
  repositories; the generated `.sc-lint/bootstrap` is the sole managed helper.

## Acceptance Criteria

- a generated consumer Justfile contains no analyzer crate/package name.
- lint/test work never begins when the compatibility preflight fails.
- a source checkout and a consumer repository exercise distinct explicit
  command paths without directory-name detection.
- `sc-lint init --just --check` proves the generated configuration and Just
  template are current without modifying consumer files; a re-run is
  idempotent and reports any user-owned file conflict instead of overwriting.
- ADR-012, requirements, product architecture, CLI requirements, CLI
  architecture, and CLI contract are updated with the shipped command surface.
- `just lint` and `just test` each have tests proving all required profile
  members run and that any member failure fails the aggregate command.
- new CLI/configuration types use validated wrappers at boundaries (`RBP-004`)
  and new error paths comply with `RBP-001`.

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- generated consumer fixture: `just lint` and `just test`
- regression test: missing installed backend reports a structured failure, not
  `FileNotFoundError` or a Python traceback

## Implementation Record

- `sc-lint init --just [--check|--dry-run]` owns only `sc-lint.toml`,
  `Justfile`, and `.sc-lint/bootstrap`; it reports managed paths, is
  idempotent, and rejects a conflicting user-owned path without touching a
  README.
- Consumer lint uses the explicit `sc-lint lint ci --consumer` command path;
  consumer test uses `sc-lint test`. Both validate named argv profiles in
  `sc-lint.toml` and run from that file's directory without source checkout
  discovery.
- The generated Just fixture verifies that lint and test each preflight first,
  then call the installed product command. Missing product/backend commands
  return a stable recovery code and documentation reference without a
  traceback.
