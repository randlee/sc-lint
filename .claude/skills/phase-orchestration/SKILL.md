---
name: phase-orchestration
version: 0.1.0
description: Orchestrate multi-sprint phase execution as team-lead. Manages sprint waves, scrum-master lifecycle, PR merges, clint reviews, and integration branch strategy. This skill is for team-lead only, not for scrum-masters.
depends_on:
  scrum-master: 0.x
  rust-developer: 0.x
  rust-qa-agent: 0.x
  req-qa: 0.x
  rust-architect: 0.x
---

# Phase Orchestration

This skill defines how team-lead orchestrates a development phase consisting of
multiple sprints with dependency-aware parallelism.

Audience: team-lead only. Scrum-masters have their own process defined in
`.claude/agents/scrum-master.md`.

## Prerequisites

Before starting a phase:
1. The phase plan exists in `docs/project-plan.md` or a linked phase document.
2. The integration branch `develop` exists and is up to date with
   `develop`.
3. The Claude/ATM team is active.
4. `clint` is running and reachable via ATM CLI.

## Phase Execution Loop

### 1. Build the sprint dependency graph

Read the phase plan and identify:
- sprint dependencies
- parallel waves
- merge order within each wave

Translate the dependency graph into Beads before the first dispatch: create an
epic chain with `bd dep add <next> <prereq>`, assign each bead to the
recipient's ATM identity, and use the bead id as the ATM task id. Each QA bead
depends on its dev bead; merge/release beads depend on QA; promoted QA fixes are
child beads that feed a follow-up QA bead; that QA bead blocks merge. After a
QA close, read its verdict before `bd ready`: PASS may dispatch what opened;
FAIL first creates fix beads, QA-2 depending on every fix, and a merge
dependency on QA-2. A merge bead is dispatchable only after its latest QA bead
closes PASS. The lead's dispatch view is unfiltered `bd ready`; an assignee
uses `bd ready --assignee "$ATM_IDENTITY"`.

### 2. Execute sprints

For each sprint, respecting dependency order:

#### a. Spawn a fresh scrum-master

Each sprint gets a fresh scrum-master. Do not reuse scrum-masters across
sprints.

```json
{
  "subagent_type": "scrum-master",
  "name": "sm-{phase}-{sprint}",
  "team_name": "<team-name>",
  "model": "sonnet",
  "prompt": "<sprint prompt>"
}
```

Critical rules:
- `subagent_type` must be `scrum-master`
- `name` is required
- `team_name` is required
- scrum-master is a coordinator only
- scrum-master must not write code, run tests, or implement fixes itself

#### b. Sprint prompt template

The sprint prompt should include:
- phase and sprint id
- sprint title
- plan and requirements references
- worktree location
- branch name
- PR target
- reminder that scrum-master is a coordinator only

#### c. Monitor progress

- scrum-masters report completion or escalation to team-lead
- if a scrum-master reports sub-agent spawn failure, investigate and advise
- if a scrum-master escalates architecture risk, spawn `rust-architect`

### 3. Post-sprint: CI gate and merge

After each scrum-master reports completion:
1. before QA-1, require `clint` to run a self-directed Rust best-practices
   sweep on the integration branch using the planned QA-1 review targets and
   fix all findings found there
2. verify QA passed
   - QA-1 includes the Rust best-practices review
   - QA-2 and later rounds must omit Rust best-practices review entirely
   - unresolved QA-1 RBP findings not fixed in the first fix round carry to
     the next phase backlog instead of being re-raised in later rounds
3. wait for CI green
4. merge PR to `develop` in dependency order
5. update the integration branch

### 4. Post-sprint: clint design review

After every sprint PR is merged to `develop`, request an `clint`
review via ATM CLI. Do not block the next eligible sprint unless clint
reports critical blocking findings.

### 5. Fix sprint if needed

If clint finds issues:
1. create a new worktree from `develop`
2. let clint or a fresh scrum-master execute the fixes
3. run `rust-qa-agent` and `req-qa` before merge

### 6. Wave transitions

Before starting the next wave:
1. all prerequisite sprints must be merged
2. integration branch must be current
3. critical clint findings must be addressed first
4. new scrum-masters start from the updated integration branch

### 7. Phase completion

After all sprints merge:
1. perform any phase-end version or release prep required by the plan
2. create PR `develop -> develop`
3. wait for CI green
4. merge after user approval
5. shut down remaining scrum-masters
6. do not clean up worktrees until user review

## Scrum-Master Lifecycle

- fresh per sprint
- named ATM teammate
- can spawn background sub-agents
- shut down after sprint completion
- never does dev work

## Team Lifecycle

- team persists across phases
- scrum-masters are ephemeral
- clint is persistent and communicates via ATM CLI

## ATM CLI Communication

Assign work with a template and a task, never with a plain message:

```bash
atm task assign <agent> --task-id <task-id> --template <template.j2> --vars <vars.json>
atm read
atm inbox
```

The template tracks state and the task assignment queues the work and nudges
the agent; see `.claude/skills/codex-orchestration/SKILL.md` "Assignment
Templates". Plain `atm send` is for questions and notices only. ATM nudges the recipient
of every message, and an assigned task re-nudges an agent that stops working;
there is no manual nudge.

## Anti-Patterns

- do not use `rust-developer` as the scrum-master subagent type
- do not tell scrum-masters to do dev work themselves
- do not do dev or QA work as team-lead
- do not skip post-merge clint reviews
- do not merge without QA pass and CI green
- do not reuse scrum-masters across sprints
