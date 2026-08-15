---
id: F.3d
title: Thin Wyvern Launcher And Configure Wizard Pages
status: planned
target: develop
---

# Sprint F.3d — Thin Wyvern Launcher And Configure Wizard Pages

## Goal

Implement the F.3a pages over the F.3b-qualified released Wyvern wizard API.
The adapter transports bounded context and normalized request data only; it has
no repository-discovery, recommendation, mutation, or navigation policy.

## Hard Dependencies

- F.3a UX contract;
- F.3b PASS on the pinned released Wyvern artifact;
- F.3c agent JSON schemas and equivalence fixtures.

## Exact Targets

- `scripts/sc_lint_configure_wyvern.py` (new)
- `assets/configure-wizard/` (new static page assets)
- configure front-end fixtures/tests
- `docs-bundle/configuration.md`

## Deliverables

- A Python launcher that invokes only the qualified Wyvern protocol, passes the
  F.3a context/configuration, validates the terminal full-stack result, and
  normalizes it through the F.3c request schema.
- Ten page assets matching F.3a exactly, with no embedded product-policy
  branches beyond rendering the provided descriptor/configuration.
- Stable unavailable, invalid-result, timeout, cancel, and dismissed responses
  with a JSON recovery path.

## Acceptance Criteria

- Each recorded UI flow normalizes to exactly the same request and dry-run plan
  as its F.3c JSON fixture.
- No page executes a command, parses an arbitrary Justfile/workflow, writes a
  target file, or receives a shell-command string.

## Required Validation

- F.3b headless navigation matrix plus cross-adapter equivalence tests
- isolated launcher unavailable/timeout/malformed-result tests
- `just lint` and `just test`

## This Sprint Does Not Close

- `configure --apply`, legacy migration, or consumer conversion.
