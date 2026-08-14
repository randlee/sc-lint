---
id: F.5
title: Release, Documentation, And Dual-Consumer Qualification Handoff
status: planned
target: develop
---

# Sprint F.5 — Release, Documentation, And Dual-Consumer Qualification Handoff

## Goal

Prepare the released `sc-lint configure` product and its supporting evidence
for Phase P's real-consumer qualification. This sprint proves that the product
artifact, contract, fixtures, and documentation are ready to be tested; it does
not claim that a tool works in `sc-compose` or `atm-core` until Phase P runs the
same released artifact against both disposable copies and their consumer-owned
PRs.

## Hard Dependencies

- F.1 through F.4
- a released sc-lint version containing the F.4 configuration/Action contract
- Phase P approval and access to dedicated, clean disposable consumer copies;
  Phase P owns any sc-compose/atm-core worktree and team acceptance.

## Exact Targets

### sc-lint repository

- Phase F plans and requirements/ADR/architecture docs
- consumer lifecycle/acceptance fixtures
- docs-bundle setup, Just, CI, upgrade, troubleshooting, and best-practice
  guides
- root `README.md`, `AGENTS.md`, and Phase F project-plan/roadmap entries
- reusable Action docs/tests/release validation

## Deliverables

- the released artifact, checksum manifest, supported platform targets, public
  configuration schema version, and installed-documentation path are recorded
  for Phase P. A source checkout or ambient developer binary is not valid
  handoff evidence.
- product fixtures cover empty repositories, recognized legacy migrations,
  near-miss/no-write conflicts, transactional rollback, marker idempotency,
  and the reusable Action on Linux, macOS, and Windows.
- sc-lint documentation explains the public setup workflow: discovery,
  JSON/Wyvern selection, preview, explicit apply, reapply, conflict recovery,
  and the four standard `just` commands. It clearly says that a real consumer
  conversion is complete only after Phase P's dual-reference evidence.
- Phase P receives generic JSON request examples and schema-validation
  guidance. It creates the actual sanitized requests from current sc-compose
  and atm-core facts; F.5 must not substitute a template for that evidence.

## Acceptance Criteria

- the public product contract, error/recovery documentation, and offline bundle
  describe the same request/preview/apply/reapply workflow on all platforms.
- the recorded release artifact passes the product fixture and Action matrices;
  those tests contain no source checkout, copied `.just` utility, or ambient
  executable fallback.
- the Phase P handoff identifies the exact evidence it must collect: both
  current baseline commits, two real requests, preview/apply/reapply output,
  four `just` commands, and Linux/macOS/Windows CI.
- if Phase P exposes an unsupported existing-repository shape, it returns a
  concrete sc-lint finding to F.2/F.4; it does not authorize a manual consumer
  workaround or a false close.

## Required Validation

- sc-lint: `just lint`, `just test`, configure fixture matrix, Action matrix,
  documentation/link validation
- released binary / Homebrew installation smoke and reusable Action execution
  on Linux, macOS, and Windows
- Phase P handoff review against the required dual-consumer qualification
  matrix and no-workaround rules

## This Sprint Does Not Close

- any real sc-compose or atm-core worktree change, PR, or direct write; Phase P
  owns those consumer operations;
- arbitrary consumer migrations beyond the supported contract;
- Action secret management, auto-commit, or auto-PR behavior.
