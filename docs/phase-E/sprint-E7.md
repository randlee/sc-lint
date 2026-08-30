---
id: E.7
title: Dogfooding, Consumer Fixtures, And Cross-Platform Acceptance
status: complete
branch: feature/phase-E7-dogfood-consumer-contract
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E7-dogfood-consumer-contract
target: integrate/phase-E
---

# Sprint E.7 — Dogfooding, Consumer Fixtures, And Cross-Platform Acceptance

## Implementation Record

Completed with root Justfile dogfooding, three-platform aggregate CI, staged
release-binary consumer lifecycle fixtures, and generated Windows bootstrap
coverage. Root `setup` and `upgrade` intentionally use `--dry-run` so the
source checkout exercises installation selection and compatibility without
mutating a developer's managed installation; consumer generated recipes do not
add that source-only safeguard.

## Goal

Make `sc-lint`'s own repository the maintained reference example and prove the
complete consumer lifecycle from a clean fixture across supported platforms.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Hard Dependencies

- [Sprint E.1](./sprint-E1.md)
- [Sprint E.2](./sprint-E2.md)
- [Sprint E.3](./sprint-E3.md)
- [Sprint E.4](./sprint-E4.md)
- [Sprint E.5](./sprint-E5.md)
- [Sprint E.6](./sprint-E6.md)
- current CI and release workflows

## Exact Targets

- root `Justfile`
- root `sc-lint.toml` and `.sc-lint/bootstrap` / `.sc-lint/bootstrap.ps1`
- `.github/workflows/ci.yml`
- release/action integration workflow(s)
- disposable consumer fixture(s) and test harness
- `AGENTS.md`, root `README.md`, and installed docs as required
- `docs/phase-E/sprint-E7.md`

## Deliverables

- this repository's documented developer completion contract is exactly
  `just lint` and `just test`; both run the entire required suite and the
  standard compatibility preflight.
- the root `Justfile` is the maintained, executable model implementation for
  `docs-bundle/just-setup.md`: public `setup`, `lint`, `test`, and `upgrade`
  recipes use the same private preflight, while source-maintainer steps remain
  behind the product-owned aggregate profile rather than copied into consumer
  templates.
- CI invokes the same aggregate commands after its standard setup lane rather
  than maintaining a divergent hand-assembled tool sequence.
- fresh and outdated consumer fixtures prove initialization, setup, lint,
  test, docs discovery, and upgrade through release binaries, not source-only
  `cargo run` paths.
- CI retains usable artifacts/transcripts for each fixture lane without
  treating generated logs as source changes.

## Model Root Justfile Contract

The root `Justfile` is the maintained executable model. The exact source-only
profile command may differ, but its public consumer-facing shape must remain:

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

## Acceptance Criteria

- a fresh consumer passes `sc-lint init`, `just setup`, `just lint`, and
  `just test` on Ubuntu, macOS, and Windows.
- a compatible installation makes the preflight a no-op; a too-old or missing
  installation upgrades or emits the documented structured offline failure.
- the overview, Just guide, and every package guide resolve from the installed
  binary on all three platforms.
- `sc-lint upgrade` is a no-op when current and safely migrates a supported
  older fixture without overwriting consumer README files.
- the root repo's `AGENTS.md` and documentation require only `just lint` and
  `just test` for agent completion, while the commands retain all source
  maintenance gates.
- the root Justfile passes a golden-template parity test against the canonical
  Just document and an end-to-end `sc-lint init --just` consumer fixture;
  drift is a failing gate, not a documentation-only observation.
- an in-repo `missing-installed-sc-lint` consumer-fixture lane removes the
  installed binary from `PATH`, invokes `just lint` and `just test`, and proves
  both stop before work with the E.1 structured recovery result (including
  required version and recovery action), not `FileNotFoundError` or a Python
  traceback.

## Required Validation

- `just setup`
- `just lint`
- `just test`
- fresh-consumer, compatible-version, too-old-version, missing-version, docs,
  and upgrade fixture lanes on Ubuntu, macOS, and Windows
- release archive, Homebrew, and GitHub Action smoke installation where the
  runner supports each distribution path
