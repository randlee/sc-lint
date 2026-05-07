# Claude Instructions for sc-lint

## Branch Management Rules

Keep the main repository checkout on `develop`.

- Use worktrees for feature branches.
- Create worktrees from `develop`, not from `main`.
- Feature PRs target `develop`.
- Release PRs target `main`.

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
  - `quality-mgr`
  - `publisher`

Use `docs/team-protocol.md` as the source of truth for required
acknowledgement and completion behavior.
