# Repository Quality Policy

This file contains repository-specific QA policy. Reusable agents and skills
read this file rather than embedding repository names, commands or
exceptions in their own prompts. The orchestration templates name this file
as `policy_path` (`.claude/project/atm-bd-orchestration.yaml`).

## Repository Baseline

- Default comparison branch: `develop`
- Requirements index: `docs/requirements.md`, plus `docs/*/requirements.md`
  (for example `docs/sc-lint/cli-requirements.md`,
  `docs/sc-lint-boundary/requirements.md`)
- Architecture index: `docs/architecture.md` (layers (a)-(d)); ADRs are
  `docs/sc-lint/adr/ADR-NNN-<slug>.md`
- Project plan: `docs/project-plan.md`
- Team protocol: `docs/team-protocol.md`
- Developer roster: `.atm.toml` and the ATM roster (`atm members --team
  sc-lint`) are authoritative for live identities.

## Guidelines Reviewers Score Against

- Rust: `.claude/skills/rust-best-practices/SKILL.md`
- Portability: `docs/cross-platform-guidelines.md`

## Reviewer Policy

- Initial implementation review (sprint QA-1): the default reviewer set in
  `.claude/agents/quality-mgr.md` ("Default Reviewer Set"); the
  repository's round-1 reviewers are `reviewers_round1` in
  `.claude/project/atm-bd-orchestration.yaml`
- Fix verification (sprint QA-2 and later): only the reviewer that filed a
  finding verifies its fix, scoped to that finding
- Plan review: `plan-review-template.xml.j2` (atm-bd-orchestration)
- Every QA round requires an open PR; the rendered report is posted to it

## Validation Policy

- Lint: `just lint`
- Tests: `just test`

### Function length

A fix that clears the function-length gate must remove a repeated shape or give
a unit its own inputs and outputs. Reducing the count by compacting statements,
merging blocks, or moving the body behind an out-parameter without a new
responsibility is not a fix. Do not raise a baseline allowance to land a
feature; record why in the bead. A function may stay long only as a flat,
ordered checklist with no shared mutable state, named in the baseline with a
one-line reason in the commit message.

## Plan Naming

| Thing | Form | Example |
|---|---|---|
| Phase id | next unused letter, lower case | `i` |
| Plan file | `docs/plans/phase-<phase>.jsonl` | `docs/plans/phase-i.jsonl` |
| Phase branch | `integrate/phase-<phase>`, cut from `develop` | `integrate/phase-i` |
| Sprint branch | `sprint/<phase>-<n>-<slug>` | `sprint/i-1-lint-verdict-exit` |

## Governed Interfaces

None declared.

## Repository Exceptions

None.
