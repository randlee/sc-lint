---
name: codex-orchestration
version: 0.1.0
description: Orchestrate sc-lint sprint work where team-lead coordinates, clint is the sole developer, and quality-mgr enforces the QA gate.
depends_on:
  quality-management-gh: 1.x
  quality-mgr: 0.x
  req-qa: 0.x
  arch-qa: 0.x
  flaky-test-qa: 0.x
  rust-qa-agent: 0.x
  rust-best-practices-agent: 0.x
  rust-service-hardening-agent: 0.x
---

# Codex Orchestration

This skill defines the repo-local orchestration workflow for `sc-lint`.

## Model

- `team-lead` coordinates sprint sequencing, worktree assignments, and PR flow
- `clint` is the sole developer for Codex-driven implementation work
- `quality-mgr` runs the QA gate after each delivery

## Preconditions

Before starting a sprint:
1. `docs/requirements.md`, `docs/architecture.md`, and `docs/project-plan.md`
   define the sprint or phase review target.
2. A worktree exists for the sprint branch under the repo’s worktree strategy.
3. The target branch for the sprint is chosen from the current repo plan.
4. The following prompts exist in `.claude/agents/`:
   - `quality-mgr.md`
   - `req-qa.md`
   - `arch-qa.md`
   - `flaky-test-qa.md`
   - installed Rust reviewers from `sc-rust`
5. The following QA reporting skill exists in `.claude/skills/`:
   - `quality-management-gh/`
5. `quality-mgr` must read:
   - `.claude/assets/sc-rust/quality-mgr/quality-mgr.rust.md`
6. `quality-mgr` must also read:
   - `.claude/skills/quality-management-gh/SKILL.md`
7. `sc-compose` is available for rendering the JSON and markdown templates.

## Sprint Flow

1. `team-lead` assigns development to `clint` using `dev-template.xml.j2`.
2. `clint` ACKs, implements, commits, pushes, and reports branch plus SHA.
3. Before QA-1, `clint` performs a self-directed Rust best-practices sweep on
   the integration branch using the same `review_targets` planned for QA-1 and
   fixes all RBP findings found there. This is a developer cleanup step, not a
   QA surprise.
4. `team-lead` opens or updates the PR.
5. `team-lead` assigns QA to `quality-mgr` using `qa-template.xml.j2`.
6. `quality-mgr` launches the reviewer set:
   - `req-qa`
   - `arch-qa`
   - `rust-qa-agent`
   - `rust-best-practices-agent` in QA-1 only
   - `rust-service-hardening-agent` when service-runtime review is explicitly in scope
   - `flaky-test-qa` when test instability risk is present
7. QA-2 and later rounds must omit `rust-best-practices-agent`. Any unresolved
   QA-1 RBP findings that are not fixed in the first fix round carry to the
   next phase backlog automatically and are not re-raised in later QA rounds on
   the same sprint branch. QA-2+ stays in targeted-fix mode: use
   `triage_records`, `changed_files`, and `carry_forward_findings_json` rather
   than broad re-review by default.
8. If QA passes and CI is green, merge may proceed.
9. If QA fails, `team-lead` first runs `/triaging-findings` to correlate the
   findings across worktrees and determine the promoted fix branch. Do not send
   raw QA findings directly to `clint`.
10. After triage completes, `team-lead` routes concrete fixes back to
   `clint` using `fix-assignment.xml.j2`. For follow-up QA rounds, the routing
   layer must also preserve:
   - triage records from `qa-triage`
   - `carry_forward_findings_json` rendered by
     `scripts/triage_carry_forward.py`
   - TODO findings produced by `scripts/find_todos.py`

## Phase-End Review

For extraction-readiness or phase-close reviews, use `review-template.xml.j2`
to assign a read-only review to `clint`.

## CI

Use standard GitHub CLI:
- `gh pr checks <PR> --watch`
- `gh pr view <PR> --json mergeStateStatus,reviewDecision`

Do not assume ATM-specific PR monitoring commands exist.

## Assignment Templates

Use the templates in this skill directory:
- `dev-template.xml.j2`
- `fix-assignment.xml.j2`
- `qa-template.xml.j2`
- `review-template.xml.j2`
- `req-qa-assignment.json.j2`
- `arch-qa-assignment.json.j2`
- `flaky-test-qa-assignment.json.j2`
- reporting templates under `.claude/skills/quality-management-gh/`

Use the Rust assignment templates from:
- `.claude/assets/sc-rust/quality-mgr/templates/`

Authoritative B.4 QA-routing surfaces:
- `.claude/agents/quality-mgr.md`
- `.claude/agents/qa-triage.md`
- `.claude/skills/triaging-findings/SKILL.md`
- `.claude/skills/todo-triage/SKILL.md`
- `.claude/skills/codex-orchestration/qa-template.xml.j2`
- `.claude/skills/codex-orchestration/fix-assignment.xml.j2`
- `scripts/find_todos.py`
- `scripts/triage_carry_forward.py`

## Required Message Sequence

Every ATM task message must follow:
1. ACK
2. Work
3. Completion summary
4. Completion ACK by receiver
