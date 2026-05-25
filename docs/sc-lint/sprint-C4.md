---
id: C.4
title: Consumer Integration And Skill Design
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.4 — Consumer Integration And Skill Design

## Goal

- define exactly what a consuming repository must do to adopt
  `sc-lint-version`
- package that guidance as a repo-local Claude Code skill
- keep marketplace publication out of this sprint so the skill closure remains
  reviewable on its own

## Hard Dependencies

- [docs/sc-lint/sprint-C1.md](./sprint-C1.md)
- [docs/sc-lint/sprint-C2.md](./sprint-C2.md)
- [docs/sc-lint/sprint-C3.md](./sprint-C3.md)
- [docs/sc-lint/version-requirements.md](./version-requirements.md)
- [docs/sc-lint/skill-authoring-constraints.md](./skill-authoring-constraints.md)

## Exact Targets

- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/sprint-C4.md`
- planned authoritative consumer-adoption document:
  - `docs/sc-lint/version-adoption.md`
- `docs/project-plan.md`
- `docs/sc-lint/README.md`
- `docs/sc-lint/roadmap.md`
- planned repo-local skill design surface:
  - `.claude/skills/sc-lint-version-adoption/SKILL.md`

## Deliverables

- one explicit consumer integration document that states what a consuming repo
  must provide to exercise:
  - Rust public API checks
  - CLI interface checks using existing CLI testability surfaces where
    available
  - RPC/socket interface checks when present, using existing simulators or
    transcript fixtures where available
  - repo-local normalization hooks only when unstable values cannot be
    removed at the canonical artifact layer
- one planned repo-local Claude Code skill dedicated to `sc-lint-version`
  adoption and usage guidance
- documentation updates that keep the adoption path discoverable:
  - `docs/sc-lint/README.md` adds the `C.4` links for:
    - `docs/sc-lint/version-adoption.md`
    - `.claude/skills/sc-lint-version-adoption/SKILL.md`
  - `docs/sc-lint/roadmap.md` adds the `C.4` roadmap entry for the
    adoption/skill milestone only
- explicit references in the sprint plan that:
  - the skill design follows
    `docs/sc-lint/skill-authoring-constraints.md`

## Explicit Code Samples

```json
{
  "adoption_doc": "docs/sc-lint/version-adoption.md",
  "cli_commands": ["sc-lint --json check interfaces --family cli"],
  "rpc_simulators": ["tests/interface/simulators/session_start.json"]
}
```

```yaml
---
name: sc-lint-version-adoption
version: 1.0.0
description: Guides a consuming repository through sc-lint-version adoption.
---
```

## This Sprint Does Not Close

- implementation of the version-check engine itself
- implementation of HTML/XHTML report rendering
- final CI gate rollout for every consuming repository
- minimal marketplace publication for the adoption skill

## Acceptance Criteria

- `docs/sc-lint/version-adoption.md` is the authoritative consumer-integration
  document rather than scattering adoption steps across multiple unrelated
  docs
- `docs/sc-lint/version-adoption.md` defines harness, fixture,
  simulator/transcript, and normalization responsibilities for consuming repos
- `docs/sc-lint/version-adoption.md` explicitly states that consuming repos
  should reuse existing CLI testability and simulator infrastructure where
  available
- `.claude/skills/sc-lint-version-adoption/SKILL.md` exists and contains
  versioned frontmatter scoped to `sc-lint-version` adoption
- `.claude/skills/sc-lint-version-adoption/SKILL.md` is updated to stay
  narrowly scoped to `sc-lint-version` adoption rather than broad repo policy
  guidance
- `docs/sc-lint/README.md` and `docs/sc-lint/roadmap.md` are updated so the
  adoption document and skill remain discoverable from the normal Phase `C`
  entry points without becoming the authoritative source of adoption steps
- `docs/sc-lint/skill-authoring-constraints.md` requires the planned skill
  surface to be versioned and to stay scoped to `sc-lint-version` adoption
- the sprint references `docs/sc-lint/skill-authoring-constraints.md` as the
  authoritative skill-design constraint doc

## Required Validation

- `just lint`
