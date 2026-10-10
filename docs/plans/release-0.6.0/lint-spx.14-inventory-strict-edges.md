---
sprint: lint-spx.14
bead: lint-spx.14
epic: lint-spx
status: complete
branch: fix/inventory-strict-edges
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-strict-edges
pr_target: fix/public-trait-boundary-schema
closure_type: contract
---

# lint-spx.14 — Strict forbidden-edge deserialization and inventory rejection tests

Fix layer stacked on PR #115 (`fix/public-trait-boundary-schema` @ e49d0a8).
Source: review `lint-spx.3`, findings 1 and 4.

## Deliverables

1. Merge `origin/develop` forward into this layer first (PR #115 is 112
   commits behind; `git merge-tree` showed it clean). Do not rebase.
2. `crates/sc-lint-boundary/src/inventory/dependency_policy.rs`:
   `RawForbiddenPackageEdge::Structured` must reject unknown fields.
   Deserialize the structured form through a dedicated
   `#[serde(deny_unknown_fields)]` struct (serde ignores `deny_unknown_fields`
   on an untagged enum variant). The arrow-delimited string form stays
   accepted. Error text for a bad entry must still name the file and field.
3. Tests in `crates/sc-lint-boundary/src/inventory/tests.rs`:
   - forbidden edge inline table with an unknown field is rejected;
   - structured and arrow forms both accepted and produce equal edges;
   - malformed arrow strings rejected: no arrow, two arrows, empty side,
     whitespace-only side;
   - `[public]` with both `facade` and `trait`; with neither; with empty or
     whitespace-only values: assert the intended accept/reject for each and
     state the rule in the test name;
   - unknown field inside each of `[ownership]`, `[contracts]`, `[status]`
     is rejected.

## Acceptance criteria

- Every case above is a named test and passes; REQ-SCB-020 strictness holds
  for every nested table and the forbidden-edge entries.
- `just lint` and `just test` pass on the merged layer; `git diff --check`
  clean.
- Planning metadata and `owner_crate_path` were intentionally deferred during
  this sprint. The `lint-spx.15` decision required authoritative
  `boundaries/planning.toml` whenever `boundaries/` exists and restored the
  `owner_crate_path` invariant; `lint-spx.17` implemented that decision.
- Frontmatter `status: complete` at closeout; bead claimed and closed in
  tandem with the ATM task.

## Closeout

Implemented by `lint-spx.14` commit `1ae9893` on the merged layer. Targeted
inventory tests passed (35 tests at closeout), `git diff --check` passed, and
the aggregate `just lint` and `just test` gates passed.
