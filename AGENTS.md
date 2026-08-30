# AGENTS Instructions for sc-lint

## MUST READ

Before participating in sc-lint team work, read:
- `docs/team-protocol.md`

The messaging protocol in that document is mandatory for all team
communications.

## Quick Rule

Always follow this sequence for every team message:
1. Immediate acknowledgement
2. Do the work
3. Completion summary
4. Immediate completion acknowledgement by receiver

No silent processing.

## Branch Management Rules

Same rules as `CLAUDE.md`; keep the two in sync.

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

## Rust Guidance

For Rust design and review work, also read:
- `.claude/skills/rust-best-practices/SKILL.md`

Use it as the baseline for state machines, newtypes, sealed traits, structured
error design, and crate-boundary review.

## Completion Contract

For repository changes, agents finish with exactly:

```sh
just lint
just test
```

These are complete aggregate gates, not advisory shortcuts. Use `just setup`
when the product compatibility preflight needs to be checked or repaired.
