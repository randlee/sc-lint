---
id: C.4
title: Consumer Integration Skill And Minimal Marketplace
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.4 — Consumer Integration Skill And Minimal Marketplace

## Goal

- define exactly what a consuming repository must do to adopt
  `sc-lint-version`
- package that guidance as a repo-local Claude Code skill
- advertise the skill through a minimal repo-local Claude Code marketplace

## Hard Dependencies

- [docs/sc-lint/sprint-C1.md](./sprint-C1.md)
- [docs/sc-lint/sprint-C2.md](./sprint-C2.md)
- [docs/sc-lint/sprint-C3.md](./sprint-C3.md)
- [docs/sc-lint/version-requirements.md](./version-requirements.md)
- `/Users/randlee/Documents/github/synaptic-canvas/docs/marketplace-forwarding.md`
- `/Users/randlee/Documents/github/synaptic-canvas/docs/claude-code-skills-agents-guidelines.md`

## Exact Targets

- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/sprint-C4.md`
- `docs/project-plan.md`
- `docs/sc-lint/README.md`
- `docs/sc-lint/roadmap.md`
- planned repo-local skill design surface:
  - `.claude/skills/sc-lint-version-adoption/SKILL.md`
- planned repo-local minimal marketplace surface:
  - `.claude-plugin/marketplace.json`

## Deliverables

- one explicit consumer integration document that states what a consuming repo
  must provide to exercise:
  - Rust public API checks
  - CLI interface checks
  - RPC/socket interface checks when present
- one planned repo-local Claude Code skill dedicated to `sc-lint-version`
  adoption and usage guidance
- one planned minimal repo-local marketplace entry advertising that skill
- explicit references in the sprint plan that:
  - the minimal marketplace design follows
    `synaptic-canvas/docs/marketplace-forwarding.md`
  - the skill design follows
    `synaptic-canvas/docs/claude-code-skills-agents-guidelines.md`

## Explicit Code Samples

```json
{
  "name": "sc-lint-version-adoption",
  "description": "Guides a consuming repo through sc-lint-version adoption."
}
```

```json
{
  "name": "sc-lint-marketplace",
  "plugins": [
    {
      "name": "sc-lint-version-adoption",
      "source": "./packages/sc-lint-version-adoption"
    }
  ]
}
```

## This Sprint Does Not Close

- implementation of the version-check engine itself
- implementation of HTML/XHTML report rendering
- final CI gate rollout for every consuming repository

## Acceptance Criteria

- the plan names one authoritative consumer-integration document rather than
  scattering adoption steps across multiple unrelated docs
- the plan explicitly requires a repo-local Claude Code skill for consumer
  adoption guidance
- the plan explicitly requires a repo-local minimal marketplace entry for that
  skill
- the sprint references
  `/Users/randlee/Documents/github/synaptic-canvas/docs/marketplace-forwarding.md`
  for the marketplace design
- the sprint references
  `/Users/randlee/Documents/github/synaptic-canvas/docs/claude-code-skills-agents-guidelines.md`
  for the skill design

## Required Validation

- `just lint`
