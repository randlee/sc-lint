---
id: C.5
title: Minimal Marketplace Publication For sc-lint-version Adoption Skill
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.5 — Minimal Marketplace Publication For sc-lint-version Adoption Skill

## Goal

- define how the repo-local `sc-lint-version` adoption skill is advertised
  through a minimal repo-local Claude Code marketplace
- keep the marketplace closure separate from the skill-design closure

## Hard Dependencies

- [docs/sc-lint/sprint-C4.md](./sprint-C4.md)
- `/Users/randlee/Documents/github/synaptic-canvas/docs/marketplace-forwarding.md`

## Exact Targets

- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/sprint-C5.md`
- `docs/project-plan.md`
- `docs/sc-lint/README.md`
- `docs/sc-lint/roadmap.md`
- planned repo-local minimal marketplace surfaces:
  - `.claude-plugin/marketplace.json`
  - `packages/sc-lint-version-adoption/.claude-plugin/plugin.json`

## Deliverables

- one planned minimal repo-local marketplace publication set advertising the
  `sc-lint-version` adoption skill through:
  - `.claude-plugin/marketplace.json`
  - `packages/sc-lint-version-adoption/.claude-plugin/plugin.json`
- explicit reference in the sprint plan that the marketplace design follows
  `/Users/randlee/Documents/github/synaptic-canvas/docs/marketplace-forwarding.md`
- clear division between:
  - skill creation in `C.4`
  - marketplace advertisement in `C.5`

## Explicit Code Samples

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

```json
{
  "name": "sc-lint-version-adoption",
  "version": "1.0.0",
  "author": { "name": "sc-lint" }
}
```

## This Sprint Does Not Close

- skill design or authoring rules for the adoption skill itself
- implementation of the version-check engine
- HTML/XHTML report rendering

## Acceptance Criteria

- the sprint explicitly references
  `/Users/randlee/Documents/github/synaptic-canvas/docs/marketplace-forwarding.md`
  for the marketplace design
- the sprint closure is only marketplace publication, not skill creation
- the plan names both planned repo-local marketplace publication surfaces:
  - `.claude-plugin/marketplace.json`
  - `packages/sc-lint-version-adoption/.claude-plugin/plugin.json`
- the plan matches the source-repo publication shape described in
  `/Users/randlee/Documents/github/synaptic-canvas/docs/marketplace-forwarding.md`

## Required Validation

- `just lint`
