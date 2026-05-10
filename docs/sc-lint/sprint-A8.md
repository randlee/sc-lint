# Sprint A.8 — Per-Tool User Guides

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.8"
worktree: /Users/randlee/Documents/github/sc-lint
branch: develop
status: planned
estimated_scope: M
```

## Goal

Create one comprehensive user guide for each shipped linter tool in the
release-1 `sc-lint` surface.

## Scope Summary

This sprint is documentation-only. It turns the productized lint surface into
tool-by-tool user documentation that explains how each linter is invoked, what
it checks, how findings look, and how rules may be disabled when policy
permits.

## Governing Requirements

- `REQ-PRODUCT-001`
- `REQ-PRODUCT-004`
- `REQ-PRODUCT-004A`
- `REQ-PRODUCT-004B`
- `REQ-PRODUCT-010`
- `REQ-PRODUCT-011`
- `REQ-CLI-007F`

## Governing ADRs

- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`
- `docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md`

## Governing Boundaries

- `BOUNDARY-ScLintBoundaryAnalyzer`
- `BOUNDARY-ScLintPortabilityAnalyzer`
- `BOUNDARY-ScLintRuntimeAnalyzer`
- `BOUNDARY-ScLintCli`

## Prerequisites

- A.1a through A.7 complete or at least stable enough that the release-1 tool
  surfaces are not changing underneath the docs
- the primary crate-mapped lint target names are settled

## Hard Dependencies

- do not write one combined monolithic guide for all tools
- do not describe disable mechanisms that are not actually supported
- do not blur productized backend tools with repo-local convenience wrappers

## Non-Goals

- implementing new lint rules
- changing release-1 ownership boundaries
- adding disable paths where none are approved

## Primary Targets

- `docs/sc-lint/tools/`
- `docs/sc-lint/README.md`
- `README.md`
- `docs/project-plan.md`
- `docs/sc-lint/foundation-phase-plan.md`

## Sub-Tasks

1. Define the guide set
   Development work:
   - identify every shipped linter tool in the release-1 surface
   - use the canonical document path:
     - `docs/sc-lint/tools/`
   - name each document after the lint tool itself, for example:
     - `sc-boundary.md`
     - `sc-portability.md`
     - `sc-runtime.md`
   Required tests:
   - doc inventory review only
   Required doc or boundary updates:
   - update the docs index to point to the new guide set
   - update the repository-root `README.md` to link every tool guide directly

2. Write one guide per tool
   Each guide must cover:
   - purpose and scope
   - primary invocation path
   - example commands
   - representative passing and failing examples
   - output format notes
   Required tests:
   - doc review for coverage completeness
   Required doc or boundary updates:
   - add cross-links from the repository-root `README.md` and roadmap where
     useful

3. Align CLI and wrapper references
   Development work:
   - make sure guides use the primary crate-mapped lint target names
   - distinguish:
     - top-level CLI commands
     - repo-local `just` entrypoints
     - direct backend commands when relevant
   Required tests:
   - cross-doc review for invocation consistency
   Required doc or boundary updates:
   - update CLI docs if any guide reveals naming drift

## Split Recommendation

Keep A.8 together if the release-1 tool set stays small. If the guide set
grows materially, split by tool family rather than by document section.

## Acceptance Criteria

- each shipped linter tool has its own standalone user guide
- each guide lives under `docs/sc-lint/tools/` and is named after the tool
- each guide includes invocation examples
- each guide includes representative pass/fail examples
- each guide states clearly how rules are disabled, or that they cannot be
  disabled
- the guides use the primary crate-mapped CLI target names consistently
- the repository-root `README.md` links every per-tool guide directly

## Required Validation

- `just lint`
- `git diff --check`

## Required Document Updates

- `docs/sc-lint/README.md`
- `README.md`
- `docs/project-plan.md`
- `docs/sc-lint/foundation-phase-plan.md`
- the new per-tool guide documents created in this sprint

## Risks And Watchouts

- do not let convenience wrappers become the documented product contract
- do not imply that all tools share the same disable model
- do not omit examples for failure output and rule-disable guidance
