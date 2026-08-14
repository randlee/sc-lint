---
id: F.2
title: Shallow Repository Context And Deterministic Configuration Plan
status: planned
target: develop
---

# Sprint F.2 — Shallow Repository Context And Deterministic Configuration Plan

## Goal

Implement the deliberately small noninteractive core that checks conventional
Rust-repository paths, validates an explicit request, and produces a stable
no-write configuration plan. F.2 is UI-free and mutation-free so every later
wizard answer is testable as data. It must not grow into a repository-analysis
framework.

## Hard Dependencies

- F.1 accepted request/plan/error contracts and ADR-014
- existing `sc-lint` configuration, installer, and consumer-profile modules

## Exact Targets

- `scripts/sc_lint_configure.py` (new)
- `scripts/sc_lint_configure_schema.py` (new)
- `crates/sc-lint/src/cli.rs`
- `crates/sc-lint/src/command.rs`
- `tests/configure/test_context_and_plan.py` (new)
- `tests/fixtures/configure/empty-rust/` (new)
- `tests/fixtures/configure/existing-just/` (new)
- `tests/fixtures/configure/unknown-existing/` (new, minimized)
- `docs/sc-lint/cli-contract.md`

## Deliverables

- context collection records only: whether root `Cargo.toml` exists and has a
  package/workspace marker, and presence of `sc-lint.toml`, `Justfile`,
  `.sc-lint/`, and `.github/workflows/`. It does not parse arbitrary Just/YAML
  files, scan source code, run Cargo metadata/lint/test commands, install a
  tool, or mutate the target repository.
- recommendations are deterministic from the observation plus schema-versioned
  request defaults. They never depend on an LLM, terminal style, locale, or
  optional Wyvern installation.
- `sc-lint configure` is a deliberately thin command dispatcher to the shipped
  Python MVP. `--request <file|-> --root <path> --dry-run --json` parses the
  request once and all rejected fields include a JSON pointer/path, stable code,
  recovery action, and docs reference. The dispatcher carries no discovery,
  recommendation, or UI policy.
- the plan is advisory and typed (`propose_create`, `needs_confirmation`,
  `manual_conflict`), ordered, and includes the exact files that a later F.4
  apply operation could manage. It carries no inferred file rewrite.
- any existing Justfile or workflow is reported as present-but-not-inspected;
  the plan asks the user/agent to choose an integration posture. It never
  claims compatibility, reads an arbitrary recipe, or proposes deletion.

## Acceptance Criteria

- the same fixture/request yields byte-identical JSON plan output on repeated
  runs and on all supported platforms after path normalization.
- an empty Rust workspace receives recommendations for baseline lint/test
  profiles and all eligible family pages.
- a repository with a Justfile/workflow receives an explicit visible “existing
  integration not inspected” decision rather than an automated migration
  recommendation.
- a malformed JSON request, unknown schema version, invalid argv, invalid
  family settings, missing root, and unreadable target all return stable
  structured errors.
- context collection and planning execute no child process; tests prove no
  installer, network, Cargo metadata, lint, or test command is invoked.

## Required Validation

- unit tests for request validation, bounded context, recommendation, plan
  ordering, and deterministic JSON
- fixture tests for empty Rust, existing Justfile, existing workflow, and
  ambiguous/non-Rust repository context
- `just lint`
- `just test`

## This Sprint Does Not Close

- interactive/Wyvern presentation;
- any file mutation or rollback;
- GitHub Action/workflow generation, workflow parsing, or a real sc-compose
  conversion.
