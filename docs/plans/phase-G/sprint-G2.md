---
id: G.2
title: Adoption Skill, Agent Prompts, Marketplace Entry
status: planned
branch: sprint/G.2-adoption-skill
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.2-adoption-skill
stack: A
stack_base: sprint/G.1-adoption-kit
target: develop (via stack A, PR base sprint/G.1-adoption-kit)
owner: clint
---

# Sprint G.2 — Adoption Skill, Agent Prompts, Marketplace Entry

## Goal

- make adoption an agent-run procedure: gather repo facts, author
  `install.json`, dry-run, apply, open the consumer PR — using only the G.1 kit

## Hard Dependencies

- G.1's **Unblock Milestone** committed on `sprint/G.1-adoption-kit`; this
  sprint's PR base is that branch, so implementation may begin before G.1
  CI, QA, review, or merge.
- reference form: `../sc-publish/plugins/sc-publish/.claude/skills/publishing/SKILL.md`
  and `.claude/agents/publisher.md`
- [docs/sc-lint/skill-authoring-constraints.md](../../sc-lint/skill-authoring-constraints.md)
- [docs/sc-lint/minimal-marketplace-constraints.md](../../sc-lint/minimal-marketplace-constraints.md)
- [ADR-016](../../sc-lint/adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md)

## Exact Targets

- `packages/sc-lint-adoption/.claude/skills/sc-lint-adoption/SKILL.md` (new)
- `packages/sc-lint-adoption/.claude/skills/sc-lint-adoption/adopt.xml.j2` (new)
- `packages/sc-lint-adoption/.claude/agents/sc-lint-adopter.md` (new)
- `packages/sc-lint-adoption/.claude/skills/sc-lint-adoption/evals/` (new)
- `.claude-plugin/marketplace.json` (add `sc-lint-adoption` entry)
- `docs/sc-lint/adoption.md` (new; authoritative consumer guide)
- `docs/sc-lint/README.md`
- `README.md` (adoption pointer only)

## Governing Contract

This sprint documents and evaluates the G.1 implementation of
REQ-PRODUCT-019, REQ-PRODUCT-021, and REQ-PRODUCT-023. It may orchestrate an
external consumer PR, but it does not add deletion, configuration, or policy
logic beyond the G.1 kit.

## Deliverables

- `SKILL.md` steps, each with the exact command: (1) collect facts —
  workspace members, existing root Justfile recipes, existing CI matrix, Rust
  toolchain; (2) write `install.json` and validate against
  `install.schema.json`; (3) `--dry-run`; (4) install; (5) `just setup && just
  lint && just test`; (6) remove any consumer-local sc-lint scaffolding the
  kit now provides, listed by the agent in the PR body; (7) open the PR with
  the drift check output attached. Step 6 is a consumer-PR action by the
  agent; the kit never deletes.
- `sc-lint-adopter.md` agent prompt in the sc-publish publisher style, with
  the same acknowledgement/completion protocol as `docs/team-protocol.md`.
- `evals/` with at least two durable evaluations: empty workspace and
  established workspace, asserting the PR body contains the dry-run exit 0
  line.
- `docs/sc-lint/adoption.md` documents end state, `install.json` fields,
  drift semantics, and the sc-publish delegation rule; `version-adoption.md`
  links to it.
- Marketplace entry `sc-lint-adoption` with `source: ./packages/sc-lint-adoption`.

## Acceptance Criteria

- `jq '.plugins[]|select(.name=="sc-lint-adoption").source' .claude-plugin/marketplace.json` → `"./packages/sc-lint-adoption"`.
- Every command quoted in `SKILL.md` runs verbatim against
  `tests/fixtures/adoption/empty-workspace` and exits 0.
- `grep -rEn "sc-compose|atm-core|wyvern" packages/sc-lint-adoption/.claude docs/sc-lint/adoption.md` returns nothing.
- The established-workspace evaluation asserts that the generated PR body
  records the successful `--dry-run` exit-0 result.
- `docs/sc-lint/adoption.md` documents the consumer end state, `install.json`
  fields, drift semantics, and the sc-publish delegation rule.
- `sc-lint-adopter.md` requires the acknowledgement and completion protocol
  defined by `docs/team-protocol.md`.
- The skill derives analyzer enablement/reason fields from async-runtime and
  target-platform facts, migrates existing `just test-*` recipes into declared
  layers, and both the skill and adoption guide include a “How to extend”
  section for analyzers, test layers, lint profiles, and consumer-owned recipes
  using `tests/fixtures/adoption/analyzer-worked-example/`.
- ``grep -rEn "command arrays must name a shipped binary or `sc_lint` module only" docs/sc-lint/adoption.md packages/sc-lint-adoption/.claude`` finds the required boundary guidance, and no example step argv in those files names a repository-relative path.

## How To Extend

- **Lint profiles:** declare ordered `[tool.sc-lint.lint.<profile>]` step lists.
- **Test layers:** declare `[tool.sc-lint.test.<layer>]`; `unit` is default,
  `just test <layer> *args` passes through, and `just test all` follows order.
- **Analyzer config:** use `[tool.sc-lint.analyzers.<crate>]` with `enabled`,
  `reason`, `targets`, and crate-specific keys.
- **Step argv/exclusions:** express flags as command arrays in a step list;
  command arrays must name a shipped binary or `sc_lint` module only; a
  repository-relative script is not authorized here.
- **Platform conditionals:** use `platforms = [...]` on a step.
- **Environment:** use per-step or per-layer `env` tables.
- **Pre/post work:** model it as ordinary extra steps in the layer list.
- **Tool pins:** only `sc-lint` is kit-pinned; all other pins are consumer-owned.
- **Non-test dispatchers:** add consumer-owned Justfile recipes outside the
  managed import block.

Patterns independently observed in 2+ consumer repositories are candidates for
promotion into the kit as a new default under Locked Principle 9.

## Unblock Milestone

None. G.2 is the top layer of Stack A; no sprint starts on a G.2 commit. The
external sprints G.4a and G.4b start on a released artifact (the versioned
`sc-lint` release containing G.2 and G.3b), as recorded in the phase plan's
parallel-vs-sequential table.

## Required Validation

- `python3 -m pytest tests/adoption -q`
- skill eval run recorded in the PR description

## Out Of Scope

- running the skill against a real consumer (G.4a–G.4c)
