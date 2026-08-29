---
id: G.0
title: Abandon Phase F And Record ADR-015 / ADR-016
status: planned
branch: sprint/G.0-abandon-phase-F
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.0-abandon-phase-F
stack: A
stack_base: feature/phase-G-planning
target: develop (via stack A, PR base feature/phase-G-planning)
owner: clint
---

# Sprint G.0 — Abandon Phase F And Record ADR-015 / ADR-016

## Goal

- close Phase F without merging any of it and record the replacement design
  decision so no later sprint can reintroduce a consumer-specific engine

## Hard Dependencies

- [phase-G-plan.md](./phase-G-plan.md)
- [docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md](../../sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md)
- `integrate/phase-F` and `sprint/F.*` branches (read-only reference)

## Exact Targets

- `docs/sc-lint/adr/ADR-015-standard-repo-tools-adoption-kit.md` (new)
- `docs/sc-lint/adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md` (new)
- `docs/sc-lint/adr/README.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`
- `docs/phase-E/phase-E-plan.md` (status only)
- `docs/plans/phase-G/*` (this plan set, merged to develop)

## Deliverables

- ADR-015 with Status `Accepted`, recording: the eight locked principles from
  the phase plan; that ADR-014 is rejected and never merged; the kit form
  (`packages/sc-lint-adoption` → consumer `plugins/sc-lint`); the sc-publish
  delegation rule for sc-lint setup.
- ADR-016 with Status `Accepted`, recording three linked decisions and their
  consequences: (a) **no Rust for configuration** — installation, templating,
  repo facts, CI wiring, and consumer scaffolding are Python, declarative
  assets, skills, and prompts. A thin maturin bridge may expose existing CLI
  behavior but adds no configuration policy or repository logic; (b) **the
  `sc-lint` Python wheel is
  the runtime delivery for every consumer helper** — built with maturin, pinned
  by `sc-lint.toml` `minimum_version`, provisioned into `.sc-lint/venv` by
  `.sc-lint/bootstrap setup`, replacing every copied `.just/*.py`; (c) **the
  `just` interface is exactly ADR-012's four recipes** and each is one line
  delegating to bootstrap, so a consumer's `Justfile` never encodes tool
  knowledge. ADR-016 must include the bootstrap → venv → wheel → binary
  resolution sequence as a code block and the rule that a profile entry may
  reference only a shipped binary or a `sc_lint` module.
- `docs/project-plan.md` links Phase G and marks Phase F abandoned with a
  one-line reason and the archive tag name.
- `docs/requirements.md` adds REQ-PRODUCT-023 (versioned reusable adoption
  kit with idempotent install and non-mutating drift detection) and
  REQ-PRODUCT-024 (version-matched wheel delivery for every consumer-run
  Python helper, with no source-tree helper dependency).
- `docs/architecture.md` links ADR-015 and ADR-016 from its consumer-adoption
  and repo-local-automation sections; the ADRs remain the detailed authority.
- `docs/phase-E/phase-E-plan.md` frontmatter status changed to `implemented`
  (PR #104 merged).
- Git housekeeping performed by team-lead, recorded in the PR description:
  PR #128 closed unmerged; tag `archive/phase-F` at `integrate/phase-F` head;
  worktrees `sprint/F.*` and `integrate/phase-F` removed; branches deleted
  after tagging; `worktree-tracking.md` updated.

## Acceptance Criteria

- `ls docs/sc-lint/adr/ADR-015-*.md docs/sc-lint/adr/ADR-016-*.md` lists
  both and each Status table row is `Accepted`.
- ADR-016 contains a fenced code block showing the resolution sequence and
  the four one-line recipes; `docs/sc-lint/adr/README.md` lists both ADRs.
- `rg -n "REQ-PRODUCT-023|REQ-PRODUCT-024" docs/requirements.md` returns the
  two new requirement records, and `docs/architecture.md` links both ADRs.
- `grep -c "phase-F" docs/project-plan.md` ≥ 1 and the line contains
  `abandoned`.
- `git tag -l archive/phase-F` prints the tag; `git worktree list` shows no
  `phase-F` or `sprint/F.` path.
- No `docs/sc-lint/adr/ADR-014*` file exists on `develop`.

## Required Validation

- `cargo test --workspace` unchanged (docs-only sprint).
- Docs link check: every relative link in `docs/plans/phase-G/*.md` resolves.

## Out Of Scope

- any code recovery (G.1)
- any change under `crates/`, `packages/`, `scripts/`
