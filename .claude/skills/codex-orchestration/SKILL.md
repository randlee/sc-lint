---
name: codex-orchestration
version: 0.1.0
description: Orchestrate sprint work where an appointed lead coordinates, the developer the lead assigns each sprint to is its sole developer, and quality-mgr enforces the QA gate.
depends_on:
  quality-management-gh: 1.x
  quality-mgr: 0.x
  req-qa: 0.x
  arch-qa: 0.x
  flaky-test-qa: 0.x
  rust-best-practices-agent: 0.x
  rust-qa-agent: 0.x
  rust-best-practices-agent: 0.x
  rust-service-hardening-agent: 0.x
---

# Codex Orchestration

This skill defines the repo-local orchestration workflow for this repository.

## Model

- The **lead** coordinates sprint sequencing, worktree assignments, PR flow,
  and every dispatch and report in this skill. `team-lead` is the default
  lead; `flint` or any other identity may hold the role.
- the developer is the agent the lead assigns the task to:
  `atm task assign <agent> --template <template> --vars <json>`. That
  positional agent is the only place a developer is named. No template takes
  an assignee variable and no message body names its recipient; the agent
  that receives a task is its assignee, and the task ledger records it.
- a developer is the sole developer for **one sprint**. Sprints that are
  `parallel_safe` run at the same time under different developers: the lead
  dispatches each sprint to the agent its sprint doc names in
  `recommended_agent` (chosen at plan time from the repository's developer
  roster by model tier), substituting only an idle agent of the same or a
  higher tier. Sending every sprint to one agent makes
  a parallel plan serial again. Agent count is the lead's concern, not the
  plan's: when ready sprints outnumber idle developers of the needed tier,
  start another agent of that tier instead of queueing the sprint.
- `quality-mgr` runs the QA gate after each delivery

## Lead Role

- A lead must be appointed for every phase before its first dispatch. Record
  the appointment in the phase status row of `docs/project-plan.md` and in the
  phase ledger, and announce it over ATM to every agent in the roster.
- Every template in this skill addresses the lead through the `lead`
  variable (default `team-lead`). Dispatch with `--var lead=<identity>` or a
  `lead` key in the vars file so acks, push reports, and closes reach the
  identity that actually holds the role; never hard-code `team-lead`.
- The role can be transferred mid-phase. The outgoing lead sends the incoming
  lead a handoff message listing open task ids, open PRs, and pending QA
  rounds, records the transfer in the ledger, and announces the new lead to
  the roster. In-flight tasks keep their original assigner; the new lead
  reads those reports via `atm read --task <task-id>`. Tasks dispatched after
  the transfer name the new lead.

Reports carbon-copy `team-lead` by default. Every template also takes a `cc`
variable (default `team-lead`): when the lead is another identity, the
assignee sends a one-line plain copy of each push report and close summary
to `cc`; when `lead` and `cc` are the same identity, nothing extra is sent.
The lead may set `cc` to an empty string to switch copies off.

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
6. `quality-mgr` must read:
   - `.claude/assets/sc-rust/quality-mgr/quality-mgr.rust.md`
7. `quality-mgr` must also read:
   - `.claude/skills/quality-management-gh/SKILL.md`
8. Every ATM assignment is sent with
   `atm task assign <agent> --task-id "$TASK_ID" --template <template> --vars <json>`;
   the same `TASK_ID` is the `task_id` key in the vars file. Never
   render a template yourself and send the output as message text or via `--stdin`.
   To view or validate the exact body before sending, use
   `atm compose --template <template> --vars <json>` (same renderer, same
   vars). The template path goes through the daemon-owned admission path
   and the dispatch is queryable from outside.
9. `.claude/agents/rust-best-practices-agent.md` and
   `.claude/skills/codex-orchestration/rust-best-practices-agent-assignment.json.j2`
   exist for first-pass boundary optimization review.
10. Every agent pane exports `BEADS_ACTOR` equal to its `ATM_IDENTITY` (the
    pane name, not an alias), and bead assignee values use those same names.

## Beads

This is the one lifecycle contract for development, fix, and QA work.

1. **Identity and dispatch.** Before dispatch, the lead creates one bead for
   each task, sets its assignee to the recipient's ATM identity, and uses that
   bead id everywhere: `bd update <bead-id> --assignee=<atm-identity>`;
   `atm task assign --task-id <bead-id>`; and template-vars
   `task_id`. Release work is a child of its release epic (`bd create --parent
   <epic>`).
2. **Assignee-owned tandem lifecycle.** First self-check `[ "$BEADS_ACTOR" =
   "$ATM_IDENTITY" ]`. If it fails, stop and report to the lead rather than
   claiming under the shared git user. Then start with `atm task start <id>
   "<plan>"` and `bd update <id> --claim`; after successful validation, close
   with `atm task close <id> completed --stdin` and `bd close <id>`. A refusal
   runs `atm task close <id> refused "<reason>"` and leaves the bead open with
   a note (`bd update <id> --notes "<reason>"`). A push or progress report
   closes neither system.
3. **Dependency-driven flow.** Before the first dispatch, the lead creates an
   epic chain with `bd dep add <next> <prereq>`; each QA bead depends on its
   dev bead, and merge/release beads depend on the latest QA bead. After a QA
   paired close, the lead reads the verdict before running `bd ready`: on
   PASS, dispatch what opens; on FAIL, first create fix child beads and QA-2,
   make QA-2 depend on every fix bead, and make merge depend on QA-2 (`bd dep
   add <merge> <qa-2>`). Then `bd ready` must show the fixes, not merge. A
   merge bead is dispatchable only when its latest QA bead closed PASS. An
   assignee views its own work with `bd ready --assignee "$ATM_IDENTITY"`; the
   lead uses unfiltered `bd ready` for dispatch.
4. **QA findings and rejected work.** After `/triaging-findings`, each promoted
   finding is a child bead of the epic and its id is the fix task id. For a
   failed QA round, every fix bead blocks its follow-up QA bead, which in turn
   blocks merge; it never tries to block the already-closed QA bead. `quality-mgr`
   reports stable finding ids but does not create beads. If a lead rejects
   completed work, the lead reopens its bead or creates a child bead; the lead
   never closes a bead on acceptance.

## Sprint Flow

1. the lead creates and wires the dev bead, then assigns development to its
   assignee using `dev-template.xml.j2` with the bead id as `task_id`.
   Every dev assignment must include the sprint-plan document path as
   `sprint_doc`, and that sprint document is the authoritative source for the
   task. Assignment prose may summarize, but it must not replace or weaken the
   sprint doc.
2. the developer claims the bead in tandem with task start, then implements,
   commits, pushes, reports branch plus SHA, and closes both task and bead
   after validation.
3. Before QA-1, the developer performs a self-directed Rust best-practices sweep on
   the integration branch using the same `review_targets` planned for QA-1 and
   fixes all RBP findings found there. This is a developer cleanup step, not a
   QA surprise.
4. the lead opens or updates the PR.
5. after the dev bead closes, the lead runs `bd ready`, then assigns the
   newly-ready QA bead to `quality-mgr` using `qa-template.xml.j2`.
   Every QA assignment must include `sprint_doc`, and `quality-mgr` must treat
   that sprint document as the authoritative QA scope source.
6. `quality-mgr` launches the full reviewer set on QA-1 (the sprint's first
   QA pass):
   - `req-qa`
   - `arch-qa`
   - `rust-best-practices-agent`
   - `rust-qa-agent`
   - `rust-best-practices-agent`
   - `rust-service-hardening-agent`
   - `flaky-test-qa` when test instability risk is present
7. QA-2 and later (fix-verification) rounds on the same sprint branch omit
   `rust-best-practices-agent`, `rust-best-practices-agent`, and
   `rust-service-hardening-agent` unconditionally — they reliably surface
   findings on any diff regardless of size, which turns a small fix-round
   into unbounded review churn. QA-2+ rounds launch `req-qa` + `arch-qa`
   (scoped to the dispatched finding ids) plus `rust-qa-agent` (its
   objective execution-fact gates — fmt, clippy, tests, lint, RULE-003,
   pytests — are not a subjective findings pass and stay in every round).
   The verdict is each dispatched finding's fixed/regressed/open status
   plus `rust-qa-agent`'s gate results, nothing else. Anything req-qa or
   arch-qa notices outside the dispatched findings goes in a debt-notes
   section of the report and does not affect the verdict. All QA-1
   first-pass findings from every reviewer must still be fixed before
   merge — merge gate is 0B+0I+0m with no exceptions and no backlog
   deferral. QA-1 findings route back to the developer via
   `fix-assignment.xml.j2` before QA-2, following the standard
   triage-and-fix path. `rust-best-practices-agent`, `rust-best-practices-agent`,
   and `rust-service-hardening-agent` remain part of docs-only plan review
   and phase-ending review regardless of sprint round.
8. After a QA close, the lead reads the verdict before `bd ready`: PASS and
   green CI permit the now-ready merge work; FAIL must not expose merge.
9. On any QA FAIL, the lead runs `/triaging-findings`, then creates a fix bead
   for every promoted finding and a QA-2 bead that depends on every fix. The
   merge bead depends on QA-2, so `bd ready` shows only fix work. Repeat this
   rule for each failed follow-up QA round; merge becomes eligible only after
   the latest QA bead closes PASS.
10. After every QA round that reports any finding, at any severity, the lead
   runs `/triaging-findings` (where the repository carries that skill) the
   same way: every finding is recorded, correlated
   across worktrees, and promoted to the current top layer of the stack. No
   finding is skipped, deferred, or left without a fix dispatch.
11. After triage completes, the lead creates and wires finding child beads,
   then routes concrete fixes to a developer of
   the tier the fix needs, using `fix-assignment.xml.j2`: easy fixes go to the
   fast tier for speed, not back to the sprint's developer by default. Fix assignments must also include
   `sprint_doc`, and the sprint document remains authoritative if the task
   summary omits or compresses details.

## Stacked Phases

Every phase runs as one append-only `gh stack` of sprint and fix layers
above `develop`. The rule is defined once, in
[`CLAUDE.md` and `AGENTS.md`](../../../CLAUDE.md)
§0, and is not restated here. What it means for this skill: the lead owns
the stack and each dev owns exactly one layer; every dispatch below — dev,
fix, cleanup — is a new worktree cut from the current top, and the
`<stack-discipline>` element every template carries is the dev-facing copy
of §0.

## Plan Review Flow

1. the lead completes `/plan-hardening` steps 1 through 5.
2. the lead assigns plan QA to `quality-mgr` using `qa-template.xml.j2`
   with `review_mode: plan`.
3. The QA assignment must include the phase-plan document as `sprint_doc`, and
   that plan document is the authoritative scope source for plan QA.
4. `quality-mgr` treats `review_mode: plan` as docs-only review and launches:
   - `req-qa`
   - `arch-qa`
   - `rust-best-practices-agent`
   - `rust-best-practices-agent`
   - `rust-service-hardening-agent`
5. If plan QA passes, the hardened plan is ready for implementation dispatch.
6. If plan QA fails, the lead uses the normal codex-orchestration
   triage-and-fix loop to route concrete fixes back to the developer.

## QA Coverage Rule

- `quality-mgr` must extract every deliverable, acceptance criterion, deletion
  target, required validation item, and expected artifact from `sprint_doc`
  before launching `req-qa`
- `req-qa` must independently treat `sprint_doc` as authoritative
- `req-qa` must count deliverable completion and report a completion percentage
- `arch-qa` must inspect sprint-doc structural gate artifacts directly when a
  deliverable points to a boundary, packaging, release-tracking, readiness, or
  validation gate
- QA cannot PASS unless deliverable completion is 100%
- QA scope follows the sprint doc's `closure_type`
  (`.claude/skills/plan-hardening/sprint-planning-guidelines.md`): a
  `contract` or `boundary` sprint is judged against its one
  `target_boundary`, its boundary-rooted criteria and its per-crate
  validation; behaviour listed under "This Sprint Does Not Close" is not a
  finding against it. End-to-end, CLI, live-daemon and smoke proof is judged
  in `integration` sprints and the phase-ending review

## Phase-End Review

For extraction-readiness or phase-close reviews, use `review-template.xml.j2`
to assign a read-only review to a developer.
After the phase lands, where the repository carries `triaging-findings`, run
its post-mortem
(`.claude/skills/triaging-findings/references/post-mortem.md`); the write-up
lives in `docs/plans/` and feeds the branch-management rules in `CLAUDE.md` and `AGENTS.md`.

For phase-ending QA routed through `quality-mgr`, the reviewer set is
mandatory:
- `req-qa`
- `arch-qa`
- `rust-best-practices-agent`
- `rust-qa-agent`
- `rust-best-practices-agent`
- `rust-service-hardening-agent`
- `flaky-test-qa`

Before phase-ending QA can report PASS, `quality-mgr` must require a successful
`just lint && just test` result from the assigned execution reviewer (normally
`rust-qa-agent`). To preserve `quality-mgr`'s no-foreground-QA rule, it verifies
the delegated `executed_checks.artifacts` output and source revision instead of
executing that broad command itself. The phase-end rust-qa assignment supplies
`just lint && just test` through its artifact-command channel. `just lint && just test` includes
the committed CLI-surface contract check, so released CLI changes cannot bypass
its baseline gate.

## CI

Use standard GitHub CLI:
- `gh pr checks <PR> --watch`
- `gh pr view <PR> --json mergeStateStatus,reviewDecision`

Do not assume ATM-specific PR monitoring commands exist.

## Assignment Templates

Dispatch form (mandatory for every assignment below):

```bash
VARS=<vars.json>                       # carries task_id
TASK_ID="$(jq -r .task_id "$VARS")"
atm task assign <agent> \
  --task-id "$TASK_ID" \
  --template <path/to/template.j2> \
  --vars "$VARS"
```

This form is mandatory for every orchestration assignment, to developers and
to `quality-mgr` alike (`atm task assign quality-mgr ... --template
qa-template.xml.j2`), for two reasons:

- **the template** is what makes state tracked: its frontmatter declares the
  message type, tags and workflow state/stage/scope, so the dispatch is
  queryable with `atm search` and the reports can count it. Rendered text
  sent with `--stdin`, `--file` or inline carries none of that.
- **the task assignment** is what queues the work on the agent and nudges it
  at the right time, and what `atm task start` / `atm task close` act on. A
  plain `atm send` opens no task.

`<agent>` is the only routing input. `atm send <agent> --task-id ... --template
... --vars ...` is the same operation under its alias and is acceptable;
nothing else is. A re-dispatch to an idle or silent agent re-issues the same
`atm task assign` with the same `--task-id`, template and vars. Status
questions, notices and replies that assign no work stay plain `atm send` or
`atm queue`.

Install the repository templates on the daemon host after this change merges:

```bash
mkdir -p ~/.atm/templates/codex-orchestration && cp .claude/skills/codex-orchestration/*.j2 ~/.atm/templates/codex-orchestration/
```

On atm 1.5.16, closing an assignee task with its final report also closes the
assigner's mirror task. The lead must not issue a second `--task-complete` for
that mirror; it reads the assignee's close report and proceeds with QA or the
next orchestration step.

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

## Required Message Sequence

The sequence for every ATM task assignment — start, work, task close; the
receiver never acks a close — is defined once in
[`docs/team-protocol.md`](../../../docs/team-protocol.md) (Required Flow).
The Beads lifecycle is defined in this skill's **Beads** section; templates
carry its assignee commands.
