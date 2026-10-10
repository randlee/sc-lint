# Anchors - phase-i

Facts stated during planning, kept verbatim. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-i/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | Four layers: shared types/attributes, cargo tools, CLI, just template | 2026-10-10 |
| ANC-002 | proposed | just test is repo-customized, not implemented in sc-lint | 2026-10-10 |
| ANC-003 | proposed | Each gap fix lands at its appropriate layer | 2026-10-10 |
| ANC-004 | proposed | Substantial redesign is discussed with Rand before planning | 2026-10-10 |
| ANC-005 | proposed | No additions beyond what the filed issues require | 2026-10-10 |
| ANC-006 | proposed | Issue fixes are line-item targeted additions, not redesign | 2026-10-10 |
| ANC-007 | proposed | Sprint plans carry code samples; no dev-time decisions | 2026-10-10 |

## ANC-001

> sc-lint must be a layered design.  w/ layers (lowest to highest):  a) sc-lint-types (shared types) and sc-lint-attribute (shared attributes)  b) independent cargo tools (i.e. sc-lint-boundary)  c) sc-lint cli which calls into (b)  d) just template (recommended just command layout w/ recommended mapping to (b) and (c) )

Said by Rand Lee, planning session, 2026-10-10. Status: locked. Version 1.

Summary: Four layers: shared types/attributes, cargo tools, CLI, just template

Scope: every phase-i change: architecture documentation, crate dependencies, CLI commands, and the adoption-kit just template

Deterministic check to pair with the development gate: cargo metadata --format-version 1 --no-deps: sc-lint-boundary, sc-lint-portability and sc-lint-runtime depend on no sc-lint-* crate outside sc-lint-schema, sc-lint-directives, sc-lint-analyzer-support, sc-lint-attributes; those (a) crates depend on no (b) or (c) crate; git diff of the phase shows each change in the layer its sprint names

Beads: anchor `lint-hl1`, plan questions `lint-hl1.1`, dev questions `lint-hl1.2`

## ANC-002

> I would expect 'just test' for the most part to be customized by the repo.

Said by Rand Lee, planning session, 2026-10-10. Status: proposed. Version 1.

Summary: just test is repo-customized, not implemented in sc-lint

Scope: the adoption-kit just template and any CLI test command

Deterministic check to pair with the development gate: git diff of the phase adds no consumer test logic to crates/sc-lint; the kit justfile test recipe is a consumer-editable recipe

## ANC-003

> The intent is to make these changes at the appropriate layer

Said by Rand Lee, planning session, 2026-10-10. Status: proposed. Version 1.

Summary: Each gap fix lands at its appropriate layer

Scope: every phase-i sprint that addresses a GitHub issue filed by atm-core (#193-#204)

Deterministic check to pair with the development gate: git diff --stat per sprint touches only the crates or kit files of the layer the sprint names

## ANC-004

> any substantial redesign work is to be discussed w/ me before documented or planned.

Said by Rand Lee, planning session, 2026-10-10. Status: proposed. Version 1.

Summary: Substantial redesign is discussed with Rand before planning

Scope: all phase-i plan and documentation changes

Deterministic check to pair with the development gate: no ADR, requirement or architecture redesign lands on plan/phase-i without a recorded Rand ruling

## ANC-005

> I do NOT want unauthorized additions beyond what is required to address the issues filed.

Said by Rand Lee, planning session, 2026-10-10. Status: proposed. Version 1.

Summary: No additions beyond what the filed issues require

Scope: every phase-i sprint and bead

Deterministic check to pair with the development gate: every phase-i sprint bead cites the GitHub issue (#193-#204) each deliverable addresses

## ANC-006

> These should all be line item targeted additions not redesign

Said by Rand Lee, planning session, 2026-10-10. Status: proposed. Version 1.

Summary: Issue fixes are line-item targeted additions, not redesign

Scope: every phase-i sprint that addresses a GitHub issue filed by atm-core

Deterministic check to pair with the development gate: git diff --stat per sprint is additive and confined to the files the issue names

## ANC-007

> I want the plans to be rich with code samples so that the agent doesn't need to make decisions when developing.

Said by Rand Lee, planning session, 2026-10-10. Status: proposed. Version 1.

Summary: Sprint plans carry code samples; no dev-time decisions

Scope: every phase-i sprint bead's design field

Deterministic check to pair with the development gate: each sprint bead design contains fenced code for every deliverable
