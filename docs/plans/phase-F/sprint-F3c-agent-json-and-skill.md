---
id: F.3c
title: Agent JSON Configure Contract And Setup Skill
status: planned
target: develop
---

# Sprint F.3c — Agent JSON Configure Contract And Setup Skill

## Goal

Deliver the UI-independent agent route and the setup skill using the same F.3a
fixtures. The result is deterministic JSON planning, never an interactive or
browser-dependent path.

## Hard Dependencies

- F.3a accepted page-to-JSON mapping;
- F.2 deterministic context/plan implementation.

## Exact Targets

- `scripts/sc_lint_configure.py`
- `schemas/sc-lint-configure-context.schema.json`
- `schemas/sc-lint-configure-request.schema.json`
- `.claude/skills/sc-lint-consumer-setup/SKILL.md` (new)
- `.claude/skills/sc-lint-consumer-setup/references/agent-json.md` (new)
- configure JSON fixtures/tests

## Deliverables

- `sc-lint configure --request <path|-> --root <path> --dry-run --json`
  accepts the F.3a request JSON, returns a normalized request and plan, and
  neither launches Wyvern nor writes the target repository.
- The skill explains root selection, fixture/context review, request creation,
  preview, conflict handling, explicit confirmation, and the prohibition on
  inventing repository probes or shell commands.

## Acceptance Criteria

- Every F.3a fixture can be submitted headlessly and yields deterministic,
  schema-valid JSON equal to the documented page selection result.
- Invalid pointer/value combinations return a stable error with recovery and
  mutate nothing.

## Required Validation

- schema/golden-envelope and repeatability tests
- skill examples executed against fixture roots
- `just lint` and `just test`

## This Sprint Does Not Close

- Wyvern launch, page rendering, or apply behavior.
