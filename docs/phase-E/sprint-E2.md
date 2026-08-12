---
id: E.2
title: Installation And Upgrade Engine
status: implemented
branch: feature/phase-E2-install-upgrade-engine
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E2-install-upgrade-engine
target: integrate/phase-E
---

# Sprint E.2 — Installation And Upgrade Engine

## Goal

Deliver the version-aware bootstrapper and upgrader that make the Phase E
consumer contract usable from a fresh local checkout.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Hard Dependencies

- [Sprint E.1](./sprint-E1.md) compatibility contract
- existing release artifact naming/checksum metadata, consumed read-only

## Exact Targets

- installer/bootstrap implementation and tests
- `sc-lint setup` and `sc-lint upgrade` command implementation
- product-owned bootstrap/template asset and tests
- configuration/template migration implementation and tests
- `docs/requirements.md` (`REQ-PRODUCT-020`),
  `docs/sc-lint/cli-requirements.md`, and
  `docs/sc-lint/cli-contract.md`
- `docs/phase-E/sprint-E2.md`

## Deliverables

- an idempotent bootstrapper resolves the configured floor, detects the active
  platform/architecture, and selects the product's verified release installer
  when the binary is absent or too old. It does not downgrade a newer
  compatible version.
- `sc-lint setup` reports whether it found, installed, or upgraded a binary;
  `sc-lint upgrade` supports `--check` and `--dry-run` and updates only
  product-owned config/template/action pins after successful validation.
- installer downloads verify the release checksum before the binary becomes
  active and retain a usable existing installation if a replacement fails.
- all installer failures have stable codes and recovery guidance (`RBP-001`),
  including unsupported platform, unavailable release, checksum mismatch,
  permission failure, and failed post-install version verification.
- `sc-lint init --just` has one product-owned managed bootstrap asset at
  `.sc-lint/bootstrap`; it is the only generated executable implementation in
  a consumer repository. The asset exposes `ensure`, `setup`, and `upgrade`,
  reads `sc-lint.toml`, and is regenerated/updated only by product commands.
  Every public Just recipe calls `ensure` before its own work; `setup` may
  install or upgrade, while `lint` and `test` never proceed after an
  incompatible/missing-install verdict.
- `REQ-PRODUCT-020` and CLI docs define the managed bootstrap path, its ownership,
  its SemVer/atomic-install guarantees, and its offline recovery behavior.

## Canonical Bootstrap Contract

```text
.sc-lint/bootstrap ensure  --config sc-lint.toml
.sc-lint/bootstrap setup   --config sc-lint.toml
.sc-lint/bootstrap upgrade --config sc-lint.toml --check
```

## Installation State Rules

- installation is atomic: download/extract/verify into a temporary location,
  then replace the managed target only after verification succeeds.
- `just lint`/`just test` may invoke the preflight every time, but automatic
  installation policy and any network action must be visible in output and
  configurable for offline/CI use.
- CI can disable auto-install only when its preceding action/setup step has
  already installed a compatible binary; the same compatibility check still
  runs.

## This Sprint Does Not Close

- generated consumer Just integration and `sc-lint init --just`; E.3 owns the
  template contract over this sprint's bootstrap engine
- documentation-bundle content and package-guide recovery tables; E.4 owns
  the operator manual and help surface
- release-archive staging, Homebrew formula rendering, documentation-bundle
  installation layout, and GitHub Action publication; E.5/E.6 own delivery

## Acceptance Criteria

- missing, old, current, and newer installed-version cases are deterministic
  and covered by integration tests.
- upgrade never overwrites arbitrary consumer files or consumer README files.
- interrupted/failed installation leaves the last working managed installation
  intact.
- bootstrapper behavior is independently usable by the subsequent E.3
  generated template and needs no source checkout or manually copied `.just`
  implementation files.
- `REQ-PRODUCT-020` and CLI contract updates land with the bootstrap engine; no
  installer/upgrade behavior is documented only in implementation notes.

## Required Validation

- installer checksum, rollback, and version comparison tests
- unit/integration tests using a local release-artifact fixture
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Implementation Record

- `sc-lint setup [--dry-run]` and `sc-lint upgrade [--check] [--dry-run]`
  resolve the canonical minimum version, short-circuit for a compatible newer
  installation, and otherwise select the release workflow's host archive.
- release download and `checksums.txt` are staged before extraction; activation
  is an atomic rename with a retained backup until the activated binary passes
  the stable `sc-lint --json version` probe. If rollback itself fails, the
  installer reports `CLI.SC_LINT_INSTALL_ROLLBACK_FAILED` with the backup path
  rather than claiming the prior installation was restored.
- Windows does not self-replace a running managed `sc-lint.exe`; setup stops
  before moving files and directs the operator to rerun from a separate release
  executable. The normal Windows replacement path is verified-release staging,
  retained backup rename, replacement, and post-install probe.
- the product-owned bootstrap source is
  `crates/sc-lint/assets/bootstrap`; E.3 renders that source into the managed
  consumer `.sc-lint/bootstrap` path and owns the Just template.
- installer failures use the documented `CLI.SC_LINT_*` recovery codes and
  link to the forthcoming E.4 offline installation guide.
