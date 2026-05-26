---
name: sc-lint-version-adoption
version: 1.0.0
description: Guide a consuming repository through `sc-lint-version` adoption using the authoritative consumer-integration workflow, existing CLI testability seams, existing simulators or transcripts, and only thin normalization hooks when unavoidable.
---

# sc-lint-version Adoption

Use this skill when a consuming repository is adopting the planned
`sc-lint-version` interface-checking line.

## Required Reference

Always use:

- `docs/sc-lint/version-adoption.md`

That document is the authoritative adoption guide. Do not replace it with
spread-out repo notes or ad hoc instructions.

## Scope

This skill is narrowly scoped to `sc-lint-version` adoption only.

It covers:

- Rust public API family adoption
- CLI interface family adoption
- RPC/socket interface family adoption when present
- harness, fixture, simulator/transcript, and normalization responsibilities

It does not cover:

- general release policy
- broad repo process policy
- marketplace publication
- unrelated `sc-lint` feature planning

## Adoption Rules

1. Define enabled interface families under `[version.families.<family>]`.
2. Reuse existing CLI testability surfaces where available.
3. Reuse existing simulators or transcript fixtures where available.
4. Keep normalization hooks thin and use them only when unstable values cannot
   be removed from the canonical artifact generator.
5. Keep canonical machine-readable artifacts as the source of truth.

## Expected Consumer Inputs

The consuming repo should provide:

- baseline source definitions for enabled families
- CLI fixtures for stable command surfaces
- simulator or transcript fixtures for RPC/socket surfaces when present
- any repo-local template override config under
  `[reporting.templates.<report_kind>]`

## Expected Outcome

The consuming repo can run the planned interface-check flow and produce:

- canonical interface artifacts
- shared HTML/XHTML/JSON reports
- hard-fail verdicts for breaking drift
