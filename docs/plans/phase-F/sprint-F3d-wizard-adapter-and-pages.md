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
- F.3c agent JSON conformance and equivalence fixtures.

## Exact Targets

- `scripts/sc_lint_configure_wyvern.py` (new)
- `assets/configure-wizard/` (new static page assets)
- `tests/configure/test_wyvern_adapter.py` (new)
- `tests/fixtures/configure/wyvern/` (new)

## Deliverables

- A Python launcher that invokes only the qualified Wyvern protocol, passes the
  F.3a context/configuration, validates the terminal full-stack result, and
  normalizes it through the F.3c request schema.
- Ten page assets matching F.3a exactly, with no embedded product-policy
  branches beyond rendering the provided descriptor/configuration.
- Stable unavailable, invalid-result, timeout, cancel, and dismissed responses
  with a JSON recovery path.

The launcher boundary remains this small:

```text
F.2 context + normalized request -> qualified Wyvern page descriptors
qualified terminal stack -> F.1 request-schema validation -> F.2 dry-run plan
```

It may start the pinned installed Wyvern binary and pass JSON through stdin or
the qualified local protocol, but it may not inspect a consumer path beyond the
F.2 context, calculate recommendation policy, retain page history, or render a
separate browser application.

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
