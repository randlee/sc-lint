---
id: F.3c
title: Agent JSON Configure Contract And Setup Skill
status: planned
target: develop
---

# Sprint F.3c — Agent JSON Configure Contract And Setup Skill

## Goal

Deliver the agent-facing documentation and conformance suite for the
already-implemented F.2 JSON route, using the same F.3a fixtures. The result
is a deterministic JSON workflow, never an interactive or browser-dependent
path. F.3c does not introduce a second dispatcher or schema authority.

## Hard Dependencies

- F.3a accepted page-to-JSON mapping;
- F.2 deterministic context/plan implementation.

## Exact Targets

- `.claude/skills/sc-lint-consumer-setup/SKILL.md` (new)
- `.claude/skills/sc-lint-consumer-setup/references/agent-json.md` (new)
- `tests/configure/test_agent_json_and_skill.py` (new)
- `tests/fixtures/configure/agent/` (new)

## Deliverables

- the skill and conformance fixtures exercise the F.2 command
  `sc-lint configure --request <path|-> --root <path> --dry-run --json`, which
  accepts F.3a request JSON, returns a normalized request and plan, and neither
  launches Wyvern nor writes the target repository.
- The skill explains root selection, fixture/context review, request creation,
  preview, conflict handling, explicit confirmation, and the prohibition on
  inventing repository probes or shell commands.

## Production-Ready Closure

Every listed deliverable must land production-ready for the F.3c agent route:
the public JSON command, conformance fixtures, and skill agree and are directly
executable. Nothing may be deferred except work explicitly listed in
[This Sprint Does Not Close](#this-sprint-does-not-close).

## Acceptance Criteria

- Every F.3a fixture can be submitted headlessly and yields deterministic,
  schema-valid JSON equal to the documented page selection result.
- Invalid pointer/value combinations return a stable error with recovery and
  mutate nothing.
- the skill contains no discovery logic, wrapper implementation, or mutable
  schema definition: it delegates all validation and planning to the public
  F.2 command and links to the F.1-owned schema reference.

## Required Validation

- schema/golden-envelope and repeatability tests
- skill examples executed against fixture roots
- `just lint` and `just test`

## This Sprint Does Not Close

- Wyvern launch, page rendering, or apply behavior.
