---
sprint: lint-spx.18
bead: lint-spx.18
epic: lint-spx
status: complete
branch: fix/inventory-edge-cause-py-parity
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-edge-cause-py-parity
pr_target: fix/inventory-planning-required
closure_type: contract
---

# lint-spx.18 — Forbidden-edge error cause and Python validator parity for the #115 schema

Fix layer A of QA-1 `lint-spx.16` (FAIL @ 6bd1827) on stack #169
(#115 ← #168 ← #171). Stacked on PR #171 (`fix/inventory-planning-required`
@ 5a729fa). Fix layer B (`lint-spx.19`, cfast) stacks on this layer and must
follow it; QA-2 is `lint-spx.20`.

Finding ids: RBP-F007, SC-QA-009 (edge part), RBP-F008, ARCH-002 and
SC-QA-006 (only the items introduced by PR #115).

## Deliverables

1. **RBP-F007** — `crates/sc-lint-boundary/src/inventory/dependency_policy.rs`:
   `RawForbiddenPackageEdge` is `#[serde(untagged)]`, so serde replaces the
   real cause ("unknown field `x`", "missing field `to`") with "data did not
   match any variant". Replace the untagged derive with a hand-written
   `Deserialize` (visitor accepting a string or a map) that:
   - for a map, deserializes `RawStructuredForbiddenPackageEdge` and
     propagates its error unchanged, so the message names the unknown or
     missing field;
   - for a string, produces the arrow-delimited form;
   - for any other TOML type, fails with an error naming both accepted forms.
   Both accepted forms and their resulting `ForbiddenPackageEdge` values stay
   exactly as they are today.
2. **Tests assert the cause.** Update
   `rejects_forbidden_edge_inline_table_unknown_fields` and add a
   missing-`to` test so they assert the serde cause text (the unknown field
   name; the missing field name), not only the file path or
   `forbidden_edges`. Add a test for a non-string, non-table entry (integer).
3. **RBP-F008** — `InvalidForbiddenEdge`: distinguish no arrow, more than one
   arrow, and empty/whitespace-only side, each with a message stating the
   expected `from -> to` form and which side is at fault. Remove the dead
   `from.contains("->")` branch. One test per message.
4. **ARCH-002 / SC-QA-006, PR #115 items only** —
   `bindings/sc-lint-py/python/sc_lint/lint_boundaries.py` must accept every
   inventory that the Rust loader accepts *because of PR #115*:
   - `[public]`: `trait` and `notes` keys; exactly one non-empty of
     `facade` | `trait` (same rule and same wording as Rust);
   - `[status]`: `notes`;
   - top level: `callers`, `ownership`, `contracts` tables, with the same
     allowed keys as `inventory/types.rs`.
   Add Python tests in `bindings/sc-lint-py/python/sc_lint/tests/test_lint_boundaries.py`:
   one acceptance test per new key/table, and rejection tests for both/neither
   facade|trait. Add one cross-check test that loads every boundary TOML
   fixture string PR #115 added to `inventory/tests.rs` as *accepted* through
   the Python validator, if those fixtures can be reached without copying
   them; otherwise state in the closeout why not.

## Acceptance criteria

- No `#[serde(untagged)]` remains in `crates/sc-lint-boundary`.
- The tests in deliverables 2–4 exist and pass; all existing inventory and
  Python tests pass.
- `just lint` and `just test` pass locally (PR CI does not run for
  non-develop bases); `git diff --check` clean.
- Changes confined to this layer. No rebase, no `gh stack sync`; merge the
  parent forward if it moves.
- This doc's frontmatter is `status: complete` with the final commit SHA and
  gate evidence recorded in a `## Closeout` section; add the
  `docs/project-plan.md` entry for `lint-spx.18`.
- Bead `lint-spx.18` claimed with task start and closed with task close.

## Out of scope

- Pre-existing Python/Rust divergence not introduced by #115: forbidden-edge
  content validation in Python, sprint id validation, unknown `planning.toml`
  keys, and Python's behaviour when `boundaries/` is absent (bead
  `lint-spx.22`). Visibility values, `constructor`, and their non-`none`
  variants were introduced by PR #115 and are handled by `lint-spx.27`.
- Docs, plan entries for other beads, REQ-SCB-013 wording, remaining test
  gaps, dead `validate_planning_metadata`, the `owner_crate_path` helper
  (all `lint-spx.19`).
- Pre-existing debt listed in bead `lint-spx.21`.

## Closeout

Focused validation passed before the final aggregate gates: `cargo test -p
sc-lint-boundary inventory::tests --lib` (40 passed),
`python -m unittest ...test_lint_boundaries` (6 passed), and `git diff --check`.
Implementation commit: `703df05`. Final aggregate validation passed with
`just lint` and `just test`; the ATM closeout records the gate evidence.
