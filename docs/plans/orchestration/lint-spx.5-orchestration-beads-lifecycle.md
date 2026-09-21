---
sprint: lint-spx.5
bead: lint-spx.5
epic: lint-spx
status: complete
branch: chore/orchestration-beads-lifecycle
worktree: /Users/randlee/github/sc-lint-worktrees/chore/orchestration-beads-lifecycle
pr_target: chore/codex-orchestration-task-assign
closure_type: contract
---

# lint-spx.5 — Beads lifecycle for orchestration task assignment

Layer 2 of the orchestration stack. Layer 1 (`lint-spx.1`, PR #166) is the
atm-core port with no Beads semantics. This layer adds them. Focus is **dev
task, fix task, and QA task assignment**; nothing else in this first round of
Beads integration.

## The contract (state it once in SKILL.md, reference it elsewhere)

1. **Identity.** Every dev, fix, and QA task is a bead created before
   dispatch. Bead id == `atm task assign --task-id` == vars `task_id`. Release
   work is a child of its release epic (`bd create --parent <epic>`).
2. **Tandem lifecycle, performed by the assignee.**
   - start: `atm task start <id> "<plan>"` and `bd update <id> --claim`
   - close: `atm task close <id> completed --stdin` and `bd close <id>`
   - refused: `atm task close <id> refused "<reason>"`; the bead stays open
     with a note (`bd update <id> --notes`), it is not closed.
   - A push or progress message never closes either.
   - If the lead rejects closed work, the lead reopens the bead or files a new
     child bead; there is no "lead closes the bead" step.
3. **Dependency-driven flow.** The lead plans an epic as a chain of beads
   wired with `bd dep add <next> <prereq>` before the first dispatch; each dev
   bead is followed by its QA bead (`bd dep add <qa-bead> <dev-bead>`), and
   merge/release beads depend on QA beads. Closing a bead is what opens the
   next task: after every close the lead runs `bd ready` and immediately
   dispatches each newly unblocked bead with `atm task assign`. A blocked bead
   is never dispatched.
4. **QA findings.** After `/triaging-findings`, each promoted finding becomes
   a child bead of the epic; that id is the fix task id, and the fix bead is
   added as a blocker of the QA/merge bead it came from
   (`bd dep add <qa-or-merge-bead> <fix-bead>`). `quality-mgr` reports stable
   finding ids; it does not create beads.

## Deliverables

1. `codex-orchestration/SKILL.md`: a "Beads" section with the contract above,
   and Sprint Flow steps updated to say where beads are created, wired,
   claimed, closed, and where `bd ready` drives the next dispatch.
2. `dev-template.xml.j2` and `fix-assignment.xml.j2`: workflow start step adds
   `bd update {{ task_id }} --claim`; close step adds `bd close {{ task_id }}`;
   refused path leaves the bead open. `qa-template.xml.j2`: same tandem
   steps, so a QA bead that became ready when its dev or fix bead closed is
   claimed and closed by `quality-mgr` the same way.
3. `docs/team-protocol.md` Required Flow, `CLAUDE.md` + `AGENTS.md` (mirrored)
   contract paragraph, `triaging-findings/SKILL.md` (rule 4),
   `quality-mgr.md` (finding-id handoff), `team-lead` / `phase-orchestration`
   skills (rule 3): each references the one rule; no contradictory copy.
4. Sample vars use bead ids as `task_id`; installed templates under
   `~/.atm/templates/codex-orchestration/` refreshed.

## Acceptance criteria

- `git diff chore/codex-orchestration-task-assign..HEAD` contains only the
  Beads upgrade; layer 1 files are not otherwise reworked.
- No sentence anywhere says the lead closes beads on acceptance.
- Every template composes with its sample vars; composed dev and fix bodies
  show the claim and close commands with the task id substituted.
- `just lint` passes.
- This doc's frontmatter is `status: complete` at closeout, and the assignee
  claimed and closed `lint-spx.5` in tandem with the ATM task.

## This sprint does not close

- `review-template.xml.j2` and the reviewer-agent JSON assignment templates
  (first round of Beads integration is dev, fix, and QA tasks only).
- Any Rust source change; re-dispatching in-flight QA on stack #165.
