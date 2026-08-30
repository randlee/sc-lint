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
