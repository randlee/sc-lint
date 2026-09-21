---
sprint: lint-spx.17
bead: lint-spx.17
epic: lint-spx
status: complete
branch: fix/inventory-planning-required
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-planning-required
pr_target: fix/inventory-strict-edges
closure_type: contract
---

# lint-spx.17 — Require planning metadata and restore owner-path validation

Implemented the `lint-spx.15` decision for PR #115's inventory layer.

## Deliverables

- Require `boundaries/planning.toml` whenever `boundaries/` exists, with an
  actionable error when it is missing.
- Restore the `owner_crate_path == owner_package.replace('-', '_')` invariant.
- Add rejection tests for missing planning metadata and mismatched owner paths.
- Preserve the empty-inventory behavior when no `boundaries/` directory exists.
- Update ATM and CLI fixtures with authoritative planning metadata.

## Closeout

Commits `d35afa8`, `1cc917d`, and `6bd1827` implemented and completed this
sprint. Targeted inventory tests and the final acceptance tests passed;
`just lint` and `just test` both passed.
