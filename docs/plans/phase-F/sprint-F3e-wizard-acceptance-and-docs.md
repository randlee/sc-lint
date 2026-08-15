---
id: F.3e
title: Configure Wizard Acceptance, Accessibility, And Documentation
status: planned
target: develop
---

# Sprint F.3e — Configure Wizard Acceptance, Accessibility, And Documentation

## Goal

Prove that the implemented wizard follows the F.3a contract on supported
platforms and explain the same flow in the installed documentation.

## Hard Dependencies

- F.3a UX contract, F.3b released-capability PASS, F.3c JSON path, and F.3d
  page/launcher implementation.

## Exact Targets

- configure end-to-end/accessibility fixtures and tests
- `docs-bundle/configuration.md`
- `docs-bundle/using-sc-lint.md`
- `docs-bundle/best-practices.md`
- `docs-bundle/troubleshooting.md`

## Deliverables

- Platform evidence for keyboard-only traversal, visible focus, labels,
  validation announcement, contrast, responsive minimum window behavior,
  cancellation/dismissal, and no-write guarantee.
- Documentation with the ten-page map, JSON alternative, screenshot/fixture
  references, preview/apply distinction, conflict recovery, and Wyvern
  availability recovery.

## Acceptance Criteria

- The full F.3a scenario matrix passes on Linux, macOS, and Windows; every
  terminal result matches the F.3c JSON route and cancellation leaves no diff.
- Installed docs provide a user and an agent enough information to complete or
  safely stop the workflow without reading source code.

## Required Validation

- automated headless end-to-end flows and targeted native-viewer smoke tests
- accessibility checks against F.3a criteria
- documentation-link/bundle validation
- `just lint` and `just test`

## This Sprint Does Not Close

- file application, transactional rollback, or Phase P reference-consumer
  qualification.
