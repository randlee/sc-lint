# Anchors - phase-i

Facts stated during planning, kept verbatim. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-i/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | Four layers: shared types/attributes, cargo tools, CLI, just template | 2026-10-10 |
| ANC-002 | locked | Repo owns just test; sc-lint supplies patterns and helpers | 2026-10-10 |
| ANC-003 | proposed | Each change lands in the lowest capable layer; sprint names it | 2026-10-10 |
| ANC-004 | proposed | Redesign needs Rand's prior approval | 2026-10-10 |
| ANC-005 | proposed | Only targeted additions required by #193-#204 | 2026-10-10 |
| ANC-006 | proposed | Sprint designs carry code for every deliverable | 2026-10-10 |

## ANC-001

> sc-lint must be a layered design.  w/ layers (lowest to highest):  a) sc-lint-types (shared types) and sc-lint-attribute (shared attributes)  b) independent cargo tools (i.e. sc-lint-boundary)  c) sc-lint cli which calls into (b)  d) just template (recommended just command layout w/ recommended mapping to (b) and (c) )

Said by Rand Lee, planning session, 2026-10-10. Status: locked. Version 2.

Summary: Four layers: shared types/attributes, cargo tools, CLI, just template

Scope: every phase-i change: architecture documentation, crate dependencies, CLI commands, and the adoption-kit just template

Deterministic check to pair with the development gate: cargo metadata --format-version 1 --no-deps: sc-lint-boundary, sc-lint-portability and sc-lint-runtime depend on no sc-lint-* crate outside sc-lint-schema, sc-lint-directives, sc-lint-analyzer-support, sc-lint-attributes; those (a) crates depend on no (b) or (c) crate; docs/architecture.md has a Layered Architecture section naming layers (a)-(d)

Beads: anchor `lint-hl1`, plan questions `lint-hl1.1`, dev questions `lint-hl1.2`

## ANC-002

> I expect 'just test' to be managed and customized by the repo.  sc-lint supplies patterns and general helper functions

Said by Rand Lee, planning session, 2026-10-10. Status: locked. Version 3.

Summary: Repo owns just test; sc-lint supplies patterns and helpers

Scope: the adoption-kit just template, any sc-lint CLI test command, and any helper sc-lint ships for consumer test recipes

Deterministic check to pair with the development gate: git diff of the phase adds no consumer test orchestration to crates/sc-lint; the kit's test recipe is a consumer-editable pattern; any sc-lint addition for testing is a general helper the recipe calls

Beads: anchor `lint-9lk`, plan questions `lint-9lk.1`, dev questions `lint-9lk.2`

## ANC-003

> Each phase-i change lands in the lowest layer that can carry it, and each sprint bead names that layer.

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; awaiting Rand approval; source words: "The intent is to make these changes at the appropriate layer", 2026-10-10. Status: proposed. Version 2.

Summary: Each change lands in the lowest capable layer; sprint names it

Scope: every phase-i sprint bead and its delivered diff

Deterministic check to pair with the development gate: each sprint bead names one layer (a)-(d); git diff --stat of its PR touches only that layer's crates or kit files

## ANC-004

> No phase-i document or plan bead changes a layer's responsibilities, an ADR decision, or the crate set unless Rand approved that change in discussion first.

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; awaiting Rand approval; source words: "any substantial redesign work is to be discussed w/ me before documented or planned.", 2026-10-10. Status: proposed. Version 2.

Summary: Redesign needs Rand's prior approval

Scope: all phase-i documentation, ADR and plan-bead changes

Deterministic check to pair with the development gate: every diff to docs/architecture.md, docs/requirements.md or docs/sc-lint/adr/ on plan/phase-i, and every sprint bead that changes a layer responsibility, cites a recorded Rand approval

## ANC-005

> Every phase-i deliverable is a targeted addition that addresses a named GitHub issue from #193-#204; nothing is added that those issues do not require.

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; awaiting Rand approval; source words: "I do NOT want unauthorized additions beyond what is required to address the issues filed. / These should all be line item targeted additions not redesign", 2026-10-10. Status: proposed. Version 2.

Summary: Only targeted additions required by #193-#204

Scope: every phase-i sprint bead and deliverable

Deterministic check to pair with the development gate: every deliverable in every sprint bead cites one issue from #193-#204; each sprint PR diff is additive and confined to what that issue names

## ANC-006

> Every sprint bead's design gives the code for each deliverable, so the developer makes no design decisions.

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; awaiting Rand approval; source words: "I want the plans to be rich with code samples so that the agent doesn't need to make decisions when developing.", 2026-10-10. Status: proposed. Version 2.

Summary: Sprint designs carry code for every deliverable

Scope: every phase-i sprint bead's design field

Deterministic check to pair with the development gate: each sprint bead design contains a fenced code block for every numbered deliverable
