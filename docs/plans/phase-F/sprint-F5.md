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

- F.1 through F.4b
- release infrastructure capable of producing the F.4a configuration and F.4b
  Action contract as an immutable distributed artifact

## Exact Targets

- `docs/sc-lint/configure-release-handoff.md` (new)
- `tests/fixtures/configure/release-artifact/` (new)
- `tests/configure/test_release_handoff.py` (new)

## Deliverables

- an immutable distributed release artifact, checksum manifest, supported
  platform targets, public configuration schema version, and installed
  documentation path are recorded in `configure-release-handoff.md` for Phase
  P. A source checkout or ambient developer binary is not valid handoff
  evidence. If release policy distinguishes a candidate from a promoted
  release, the record names that status and distribution URL; Phase P may use
  only the normal released-artifact installation path, never a local build.
- product fixtures cover empty repositories, recognized legacy migrations,
  near-miss/no-write conflicts, transactional rollback, marker idempotency,
  and the reusable Action on Linux, macOS, and Windows.
- F.5 validates (rather than rewrites) the F.3e/F.4a/F.4b installed documentation
  bundle against the public setup workflow: discovery, JSON/Wyvern selection,
  preview, explicit apply, reapply, conflict recovery, and the four standard
  `just` commands. It clearly says that a real consumer conversion is complete
  only after Phase P's dual-reference evidence.
- Phase P receives generic JSON request examples and schema-validation
  guidance. It creates the actual sanitized requests from current sc-compose
  and atm-core facts; F.5 must not substitute a template for that evidence.
- the handoff states that P.1 cannot begin until the consumer team has approved
  its own Phase P qualification plan. F.5 supplies product evidence only and
  neither authors that consumer plan nor performs a consumer conversion.

## Acceptance Criteria

- the public product contract, error/recovery documentation, and offline bundle
  describe the same request/preview/apply/reapply workflow on all platforms.
- the recorded release artifact passes the product fixture and Action matrices;
  those tests contain no source checkout, copied `.just` utility, or ambient
  executable fallback.
- the Phase P handoff identifies the exact evidence it must collect: both
  current baseline commits, two real requests, preview/apply/reapply output,
  four `just` commands, and Linux/macOS/Windows CI.
- F.5 can close without Phase P access or a reference-consumer worktree: its
  closure artifact is the distributable product evidence and handoff manifest.
  Phase P alone owns the subsequent two-repository qualification and PRs.
- if Phase P exposes an unsupported existing-repository shape, it returns a
  concrete sc-lint finding to F.2, F.4a, or F.4b; it does not authorize a
  manual consumer workaround or a false close.

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
