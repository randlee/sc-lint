---
sprint: lint-spx.27
bead: lint-spx.27
epic: lint-spx
status: complete
branch: fix/inventory-qa2-parity-assertions
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-qa2-parity-assertions
pr_target: fix/inventory-qa1-docs-tests
closure_type: contract
adrs: [ADR-004, ADR-011]
requirements: [REQ-SCB-012, REQ-SCB-013, REQ-SCB-020, REQ-SCB-022, REQ-SCB-023]
---

# lint-spx.27 — QA-2 fixes on the #115 stack: Python parity, assertion strength, records

Fix layer C of QA-2 `lint-spx.20` (FAIL @ eedbb3e) on stack #169
(#115 ← #168 ← #171 ← #173 ← #175). Stacked on PR #175
(`fix/inventory-qa1-docs-tests` @ eedbb3e). QA-3 is `lint-spx.28`.

Governing documents: `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`,
`docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md`,
`docs/sc-lint-boundary/requirements.md` (the REQ ids in the frontmatter). Read
them before editing. If a deliverable conflicts with one of them, stop and
report it on the task.

Full QA-2 report: `atm read --task lint-spx.20 --all`. Lead ruling: every
finding below is accepted. ARCH-101 is blocking.

Correction of record: `lint-spx.18` listed "visibility values, `constructor`"
as pre-existing divergence. That was wrong. `origin/develop`
`inventory/types.rs:546-555` has `Visibility::{Public, TraitOnly}` and
`Constructor::None` only; `Private`, `pub(crate)` and the non-`none`
constructors were added by PR #115 and are in scope here.

## Deliverables

Each test named below must assert the exact substring given. An assertion on
a file path, a table name or a generic word does not satisfy the deliverable.

1. **ARCH-101 (blocking)** — `bindings/sc-lint-py/python/sc_lint/lint_boundaries.py`:
   mirror `validate_boundary_schema` in
   `crates/sc-lint-boundary/src/inventory/mod.rs` and the enums in
   `inventory/types.rs`.
   - `implementation.visibility` accepts `public`, `trait_only`, `private`,
     `pub(crate)`.
   - `implementation.constructor`, when present, accepts `none`, `public`,
     `private`, `pub(crate)`; any other value is an error naming the value.
   - For `public`, `private` and `pub(crate)`: `type`, `module` and
     `constructor` are required. Remove the `constructor == "none"` rule.
   - For `trait_only`: `type` and `module` must be absent (keep whatever
     Python already enforces here; add what Rust enforces and Python lacks).
   - Python tests: one acceptance test per new visibility value and per new
     constructor value; one rejection test for an unknown visibility and one
     for an unknown constructor.
2. **ARCH-102** — same file: replace `bool(facade) == bool(trait)` with the
   three Rust rules, using the Rust wording after the path prefix:
   `defines an empty public.facade` / `defines an empty public.trait` (key
   present, value empty after trim); `must define exactly one of public.facade
   or public.trait` (both present); `must define a non-empty public.facade or
   public.trait` (neither). Python tests: `facade = "x"` with `trait = ""`;
   both; neither; each asserting its own message.
3. **ARCH-103, ARCH-104, SC-QA-108** —
   `crates/sc-lint-boundary/src/inventory/tests.rs`:
   - `rejects_planning_metadata_without_planning_table` asserts
     ``missing field `planning` ``.
   - `rejects_invalid_planning_item_key_shape` formats with `{:#}`, asserts
     `planning keys must use`, and gains a case for a `BOUNDARY-` key without a
     dot.
   - The three tests at `:420`, `:438`, `:456` assert
     ``unknown field `unexpected` ``.
   - `rejects_duplicate_allowed_dependents` asserts `duplicate`.
   - The nine-case unknown-field test reports every failing case, not only the
     first (collect failures, assert the list is empty).
4. **ARCH-105** — tests for `visibility = "private"` and
   `visibility = "pub(crate)"` covering the missing `implementation.type`,
   `implementation.module` and `implementation.constructor` messages.
5. **SC-QA-110** — test `sc-lint-boundary -> ` asserting
   ``right `to` side is empty``.
6. **SC-QA-104** — `crates/sc-lint/src/tests.rs`:
   - rename `empty_boundary_inventory_maps_to_backend_failure_error` to state
     what fails (the workspace graph build, per `dispatch.rs:31-46`); keep the
     `planning.toml` fixture (lead ruling on `lint-spx.20`);
   - `missing_boundary_planning_maps_to_cli_config_error` asserts the cause
     contains `planning.toml`.
7. **ARCH-108 / SC-QA-105** — one `write_member_boundary_record`. The
   `crates/sc-lint/src/tests.rs` copy must not pass `owner_package` verbatim as
   `owner_crate_path`. Do not make the `sc-lint-boundary` helper `pub` and do
   not add anything to `sc-lint-schema` (ADR-011, LEAD-001). If the two test
   crates cannot share the helper without widening a published interface, keep
   two copies, derive `owner_crate_path` correctly in both, and say so in the
   closeout.
8. **Docs** (ARCH-109 model part, SC-QA-106):
   - `docs/sc-lint-boundary/boundary-enforcement-model.md:138`: include
     malformed arrow strings alongside malformed inline tables;
   - move the exactly-one-of and `owner_crate_path` rules out of "Dual-Loader
     Behavior During TOML Migration" into the record-schema section;
   - `crates/sc-lint-boundary/README.md:164-178`: move the inserted paragraphs
     so "That entry" directly follows the `[dependencies]` example.
   Do not edit ADR-004 (tracked on `lint-spx.25`).
9. **Records** (ARCH-106, ARCH-107, SC-QA-109):
   - `docs/project-plan.md`: one bullet per bead `lint-spx.14`, `.15`
     (decision; link the bead id, no doc), `.16`, `.17`, `.18`, `.19`, `.20`,
     `.27`, `.28` in dependency order (.14, .15, .17, .16, .18, .19, .20, .27,
     .28); move the stray `lint-spx.18` paragraph at `:347-349` into the list;
     QA beads state their verdict (`.16` FAIL, `.20` FAIL).
   - `lint-spx.19` sprint doc `## Closeout`: add the fixing commit per finding
     (1324e7f or eedbb3e) and the gate results; change SC-QA-011 to
     "partially fixed, completed by lint-spx.27"; rename the mislabelled
     "Residual QA-003" to "lint-spx.18 residual (deliverable 7)".
   - `lint-spx.18` sprint doc: correct the out-of-scope sentence per the
     correction of record above.
   - SC-QA-109: state in this doc's closeout why the Rust fixture strings
     cannot be consumed from Python without copying them.

## Acceptance criteria

- Every finding id above appears in `## Closeout` with its fixing commit SHA.
- `## Closeout` records the exit status of `just lint`, `just test` and
  `git diff --check`, and the result of `gh pr checks <pr>` (this PR is in
  stack #169, so CI runs its 12 checks).
- No doc contradicts ADR-004, ADR-011 or the listed REQ ids.
- Changes confined to this layer. No rebase, no `gh stack sync`; merge the
  parent forward if it moves. Link the PR into stack #169 with `gh stack link`.
- Frontmatter `status: complete`. Bead `lint-spx.27` is claimed with task
  start and closed with task close. Do not close before the lead has replied
  to your completion message (bead `lint-spx.26`).

## Out of scope

- ADR-004 requirement range and wording (`lint-spx.25`).
- Pre-existing Python/Rust divergence: `owner_crate_path` derivation and
  `BOUNDARY-` literal in Python, forbidden-edge content, sprint ids, unknown
  `planning.toml` keys (`lint-spx.22`).
- Pre-existing inventory debt (`lint-spx.21`).

## Closeout

All QA-2 findings are fixed in implementation commit `7f83534`:

- ARCH-101: Python accepts the Rust visibility and constructor vocabulary,
  enforces the widened required fields, and has acceptance/rejection tests.
- ARCH-102: Python public-surface validation matches Rust wording, including
  empty, both-present, and neither-present cases.
- ARCH-103: missing `[planning]` asserts `missing field `planning``.
- ARCH-104: both invalid planning-key shapes assert `planning keys must use`.
- SC-QA-108: ownership, contracts, and status tests assert
  `unknown field `unexpected``; duplicate dependents assert `duplicate`, and
  all unknown-field cases are collected before the test reports failures.
- ARCH-105: private and `pub(crate)` records cover missing type, module, and
  constructor diagnostics.
- SC-QA-110: the empty right side of an arrow forbidden edge is covered by an
  exact `right `to` side is empty` assertion.
- SC-QA-104: the CLI workspace-graph test name is explicit, and missing
  planning configuration asserts that its cause contains `planning.toml`.
- ARCH-108 and SC-QA-105: the top-level fixture derives `owner_crate_path`
  explicitly for its supported packages; the two test crates retain separate
  helpers because sharing it would widen a published interface.
- ARCH-109 and SC-QA-106: model and README structure now document malformed
  arrow strings and place record-schema rules in their proper sections.
- ARCH-106 and ARCH-107: project and sprint records were corrected, including
  dependency order, QA verdicts, prior closeout commit mappings, and the
  `lint-spx.18` correction of record.
- SC-QA-109: Rust fixture strings cannot be consumed directly from Python
  because the Rust fixtures are compile-time test data in a separate crate;
  the parity fixture is therefore intentionally copied into the Python test.

Validation:

- `just lint`: passed.
- `just test`: passed.
- `git diff --check`: passed.
- `gh pr checks 176`: all 12 checks pending at closeout time.
- Draft PR #176 is linked into stack #169 with base
  `fix/inventory-qa1-docs-tests`.

Post-close CI follow-up: `Test (ubuntu-latest)` later reported red in run
`35473395473` (job `105978334178`); the workflow was still in progress when
the failure was observed, so GitHub had not published failed-step logs yet.
