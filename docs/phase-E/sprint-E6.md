---
id: E.6
title: GitHub Action Consumer Delivery
status: planned
branch: feature/phase-E6-github-action-consumer-delivery
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E6-github-action-consumer-delivery
target: integrate/phase-E
---

# Sprint E.6 — GitHub Action Consumer Delivery

## Goal

Provide the versioned GitHub Action that installs and invokes the released
consumer contract without exposing Cargo/package-manager details to adopters.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Hard Dependencies

- [Sprint E.3](./sprint-E3.md)
- [Sprint E.5](./sprint-E5.md)
- release artifact/checksum metadata and current CI action conventions

## Exact Targets

- versioned GitHub Action source and metadata
- action unit/integration fixtures
- action documentation and consumer workflow template
- release publication wiring for the Action version/tag
- `docs-bundle/ci.md` and `docs-bundle/troubleshooting.md` (or their final
  E.4 bundle paths)
- `docs/phase-E/sprint-E6.md`

## Deliverables

- the Action obtains a verified E.5 release artifact for the runner platform,
  exposes the installed binary/documentation path, and validates it using the
  E.1 compatibility contract.
- Action inputs cover the required consumer operations: `setup`, `lint`, and
  `test`; consumer workflow examples do not invoke Cargo packages or copied
  scripts.
- Action failures distinguish unavailable artifact, incompatible configured
  minimum, checksum failure, and failed command execution with recovery text.
- the Action has a stable major-version adoption form and a documented exact
  version/commit pinning recommendation for reproducible consumer workflows.
- `ci.md` documents the complete consumer workflow, version pinning policy,
  permissions, cache/offline behavior, outputs, and failure recovery; the
  troubleshooting guide contains the Action-specific stable error codes.

## This Sprint Does Not Close

- use of the Action by this repository's own CI or final consumer fixture
  acceptance; E.7 owns dogfooding and cross-platform end-to-end proof

## Acceptance Criteria

- Action fixture tests prove setup/lint/test against a published-layout local
  artifact on Linux, macOS, and Windows.
- the Action never falls back to `cargo run`, a source checkout, or an
  analyzer package name.
- documentation discovery resolves after Action installation without network
  access.

## Required Validation

- Action metadata/schema validation
- published-layout local artifact fixtures on Linux, macOS, and Windows
- consumer workflow example validation
