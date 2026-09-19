# Claude Instructions for sc-lint

## Branch Management Rules

Keep the main repository checkout on `develop`.

- Use worktrees for all branches; one worktree per branch.
- Organize branches as `gh stack` stacks rooted on `develop`. Prefer a stack
  even for a single branch, so it can grow into layers without restructuring.
- Create the bottom layer's worktree from `develop`, never from `main`; create
  each higher layer's worktree from the layer directly below it.
- Each layer's PR base is the layer directly below it; the bottom layer's PR
  base is `develop`. Stacks land on `develop` via
  `gh stack merge <pr> --yes --merge`.
- Never `gh stack sync` or `gh stack rebase`; merge forward instead.
- Release PRs target `main`.

Sprint planning shape and parallel-stack rules live in
`.claude/skills/plan-hardening/sprint-planning-guidelines.md`.

## Project Overview

`sc-lint` is a Rust lint-tool workspace for reusable repository policy
enforcement. The current crate set is:

- `sc-lint-directives`
- `sc-lint-attributes`
- `sc-lint-boundary`

The project currently focuses on:

- boundary enforcement
- portability linting
- source-level lint attributes
- standalone CI and release automation

## Key Documentation

Read these as needed:

- `docs/team-protocol.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`
- `docs/sc-lint/README.md`

Rust development guidance:

- `.claude/skills/rust-best-practices/SKILL.md`

Repo-local coordination and review skills:

- `.claude/skills/team-lead/SKILL.md`
- `.claude/skills/quality-management-gh/SKILL.md`
- `.claude/skills/sprint-report/SKILL.md`

## Team Configuration

- Team: `sc-lint`
- Key teammates:
  - `team-lead`
  - `clint`
  - `cfast` — Codex development pane; uses the same team identity and task-list routing configuration as `clint`.
  - `quality-mgr`
  - `publisher`

Use `docs/team-protocol.md` as the source of truth for required
acknowledgement and completion behavior.

## Task–Bead Lifecycle

For every dev, fix, and QA task, create and dependency-wire a bead before ATM
dispatch; its id is the `--task-id` and template `task_id`. The lead assigns the
bead to the recipient's ATM identity. The assignee starts with `atm task start`
and `bd update <id> --claim --actor "$ATM_IDENTITY"`, and completes with `atm
task close` and `bd close <id> --actor "$ATM_IDENTITY"`. Refusal leaves the bead
open with an actor-attributed note. After each paired close the lead runs `bd
ready` and dispatches only newly unblocked work; rejected completed work is
reopened or replaced by a child bead, never closed by the lead on acceptance.


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:1105d646 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/core-concepts/sync-concepts.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->
