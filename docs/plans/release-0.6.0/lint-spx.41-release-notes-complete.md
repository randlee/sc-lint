---
sprint: lint-spx.41
bead: lint-spx.41
epic: lint-spx
status: in-progress
branch: docs/release-notes-0.6.0-boundary
worktree: /Users/randlee/github/sc-lint-worktrees/docs/release-notes-0.6.0-boundary
pr_target: develop
pr: 189
closure_type: docs
adrs: []
requirements: []
---

# lint-spx.41 — Complete the 0.6.0 release notes (PR #189)

PR #189 is the bottom (and today only) layer of its stack on `develop`. It
extends `release/RELEASE-NOTES-0.6.0.md` for the boundary fixes but does not
cover everything that ships in 0.6.0. 0.6.0 is everything on `develop` since
`v0.5.0` (`git log v0.5.0..origin/develop`, 182 commits at `e2477c3`).

The `chore/sc-publish-wheel-release` layer (lint-spx.42) is stacked on this
branch and merges this layer forward once this push lands.

## Deliverables

1. Audit every merged PR and commit in `v0.5.0..origin/develop` against the
   notes. Every user-visible, consumer-facing, or release-relevant change
   appears in the right section with its PR number(s). Internal-only changes
   (agent prompts, skills, orchestration) get one grouped entry, not silence.
2. Known gaps to close (the audit is authoritative; this list is a floor):
   - Beads-driven codex-orchestration port: #166, #167, #170, #174, #179, #181.
   - CI runs on every pull-request target: #182.
   - Identity-literals unicode-escape parser fix (G.3c): #140; fixes issues
     #85 and #88.
   - Phase F line archived (G.0): #139.
   - Packaging: 0.6.0 is the first PyPI publication of the `sc-lint` wheel
     (`bindings/sc-lint-py`); the adoption kit bootstrap
     (`packages/sc-lint-adoption/.sc-lint/bootstrap:95`) installs
     `sc-lint==<version>` from PyPI unless `SC_LINT_WHEEL_DIR` is set.
   - Release date and Validation section (develop SHA, CI run id) refreshed to
     the current `develop` head.
3. `Approval: pending release publication approval` stays as is.

## Acceptance Criteria

- Every PR merged into `develop` since `v0.5.0` is either cited in the notes
  or listed in the PR #189 body as deliberately omitted, with a reason.
- Every factual claim (file:line, schema version, behavior) is verified
  against `origin/develop`.
- `just lint` passes; PR #189 CI green.
