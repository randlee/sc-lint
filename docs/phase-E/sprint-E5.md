---
id: E.5
title: Release Distribution And Documentation Package
status: implemented
branch: feature/phase-E5-release-documentation-package
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E5-release-documentation-package
target: integrate/phase-E
---

# Sprint E.5 — Release Distribution And Documentation Package

## Goal

Ship the product and static documentation bundle through one deterministic
release layout for release archives and Homebrew.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Hard Dependencies

- [Sprint E.1](./sprint-E1.md)
- [Sprint E.2](./sprint-E2.md)
- [Sprint E.3](./sprint-E3.md)
- [Sprint E.4](./sprint-E4.md)
- `release/publish-artifacts.toml`
- `scripts/release_artifacts.py`
- `.github/workflows/release.yml`
- current Homebrew formula renderer

## Exact Targets

- `release/publish-artifacts.toml`
- `scripts/release_artifacts.py` and tests
- `.github/workflows/release.yml`
- generated Homebrew formula/template and tests
- documentation-bundle staging source and package manifest
- root `README.md` and installed docs as required
- `docs/requirements.md` (`REQ-PRODUCT-021`) release-distribution requirements
  and release docs
- `docs/phase-E/sprint-E5.md`

## Deliverables

- release archives contain a first-class static `sc-lint-docs` documentation
  package alongside the shipped binaries. It contains the E.4 `README.md`,
  `just-setup.md`, and every package guide in the documented logical layout.
- the primary Homebrew formula installs binaries in `bin` and the
  `sc-lint-docs` package in its formula-owned `pkgshare`; it does not write a
  `README.md` into any consumer repository.
- the release manifest and artifact tooling make documentation-bundle staging,
  archive contents, and Homebrew installation deterministic and testable.
- installed `sc-lint docs --path` resolves the actual archive/Homebrew bundle
  path; no network fetch is required after installation.
- `REQ-PRODUCT-021` and release documentation explicitly record the documentation
  package as a required artifact of the primary `sc-lint` distribution.

## This Sprint Does Not Close

- GitHub Action implementation and consumer workflow ergonomics; E.6 owns the
  Action surface after E.5's archive and Homebrew layout are stable
- repository dogfooding and the full consumer lifecycle matrix; E.7 owns final
  acceptance

## Acceptance Criteria

- archive content validation fails when any required documentation file is
  absent, unexpected, or not recorded in the package manifest.
- generated Homebrew formula syntax and staged installation layout are tested.
- package docs resolve from release archive and Homebrew `pkgshare` layouts
  without source-checkout fallback.

## Required Validation

- release manifest and archive-content validation
- generated Homebrew formula syntax and installation-layout test
- `sc-lint docs --path` and every package lookup against each staged layout

## Implementation Record

- `release/publish-artifacts.toml` now declares the exact `sc-lint-docs`
  inventory, including the bundle manifest and every operator and package
  guide. Release tooling rejects files that are missing, unexpected, or absent
  from the bundle's own manifest.
- Release archives stage `sc-lint-docs/` beside every declared binary and are
  validated after compression. Unix packaging disables macOS AppleDouble
  metadata so the strict declared-file contract is portable.
- The primary Homebrew formula installs `sc-lint-docs` through its
  formula-owned `pkgshare`, which resolves at
  `<prefix>/share/sc-lint/sc-lint-docs`; `sc-lint docs --path` discovers both
  this layout and the archive layout without a source-checkout fallback.
- Tests cover exact staging, Windows executable names, compressed archive
  contents, package-manifest drift, generated formula syntax, Homebrew layout,
  and actual compiled-binary guide resolution for every published package.
