---
sprint: lint-spx.19
bead: lint-spx.19
epic: lint-spx
status: complete
branch: fix/inventory-qa1-docs-tests
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-qa1-docs-tests
pr_target: fix/inventory-edge-cause-py-parity
closure_type: contract
adrs: [ADR-004, ADR-011]
requirements: [REQ-SCB-012, REQ-SCB-013, REQ-SCB-020]
---

# lint-spx.19 — QA-1 docs, plan entries, test gaps and dead code on the #115 stack

Fix layer B of QA-1 `lint-spx.16` (FAIL @ 6bd1827) on stack #169
(#115 ← #168 ← #171 ← #173). Stacked on PR #173
(`fix/inventory-edge-cause-py-parity` @ de76d18). QA-2 is `lint-spx.20`.

Governing documents: `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`;
`docs/sc-lint-boundary/requirements.md` (REQ-SCB-012, REQ-SCB-013,
REQ-SCB-020). Read them before editing; nothing in this layer may contradict
them. If a deliverable below conflicts with ADR-004, stop and report it on the
task instead of implementing it.

Full QA-1 report: `atm read --task lint-spx.16 --all`.

## Deliverables

1. **Plan and sprint docs** (SC-QA-001, SC-QA-002, SC-QA-013, QA-003,
   ARCH-012):
   - `docs/plans/release-0.6.0/lint-spx.14-inventory-strict-edges.md`: replace
     the statement that planning.toml handling and `owner_crate_path` are
     unchanged with the recorded outcome (decision `lint-spx.15`, implemented
     by `lint-spx.17`), and add a `## Closeout` with commit 1ae9893 and gate
     evidence.
   - Add `docs/plans/release-0.6.0/lint-spx.17-inventory-planning-required.md`
     (scope from `bd show lint-spx.17` and `bd show lint-spx.15`, commits
     d35afa8, 1cc917d, 6bd1827, `status: complete`).
   - `docs/project-plan.md`: entries for `lint-spx.17`, `lint-spx.19` and the
     QA beads `lint-spx.16` / `lint-spx.20`, in dependency order.
2. **Requirements and model docs** (SC-QA-003, SC-QA-007):
   - Amend REQ-SCB-013 in `docs/sc-lint-boundary/requirements.md` and the
     matching text in `docs/sc-lint-boundary/boundary-enforcement-model.md`:
     `boundaries/planning.toml` is required whenever `boundaries/` exists; when
     `boundaries/` is absent the loader returns an empty inventory and no
     planning metadata is required.
   - Document in `boundary-enforcement-model.md` and the boundary README
     section: the arrow-delimited `from -> to` forbidden-edge form alongside
     the structured form; `public.trait` and the exactly-one-of
     `facade` | `trait` rule; the `owner_crate_path` rule.
3. **ARCH-001 ruling** (lead: not blocking; the check is required by the
   `lint-spx.15` decision): state the `owner_crate_path` rule as a requirement
   in `docs/sc-lint-boundary/requirements.md`, and centralize the
   `owner_package.replace('-', "_")` derivation in one Rust helper used by
   `inventory/mod.rs` and by the Rust tests that currently repeat it
   (`crates/sc-lint-boundary/src/tests.rs`, `crates/sc-lint/src/tests.rs`).
   Do not change the Python copy (bead `lint-spx.22`).
4. **Restore lost CLI coverage** (SC-QA-004, ARCH-009) in
   `crates/sc-lint/src/tests.rs`: a test that `boundaries/` without
   `planning.toml` maps to `CLI.CONFIG_ERROR` (compare `origin/develop`
   `backend_execution_failure_maps_to_backend_failure_error`), and keep
   `empty_boundary_inventory_maps_to_backend_failure_error` only if its name
   and fixture are accurate: remove the `planning.toml` it writes if `--root`
   never reads it.
5. **Test gaps** in `crates/sc-lint-boundary/src/inventory/tests.rs`
   (SC-QA-005, SC-QA-009, SC-QA-010):
   - planning.toml missing `[planning]`; missing `current_sprint`; empty and
     malformed sprint id;
   - unknown-field rejection for `[public]`, `[implementation]`,
     `[composition]`, `[references]`, `[testing]`, `[enforcement]`,
     `[planning]` and a planned item, each asserting the field name appears in
     the error; upgrade the existing `[ownership]` / `[contracts]` /
     `[status]` tests to assert the field name too;
   - duplicate `allowed_dependents` entry; `facade` set with empty `trait`.
6. **Dead code and wording** (ARCH-003, SC-QA-011, QA-002): delete
   `validate_planning_metadata` if `PlanningKey::parse` already enforces the
   same rule (prove it with the existing tests), add one `BOUNDARY_ID_PREFIX`
   const for the repeated `"BOUNDARY-"` literal in Rust, and fix the
   "for public visibility" message in `inventory/mod.rs` for the widened
   `Public | Private | PubCrate` arm, with a test.
7. **Residual from `lint-spx.18`**: one Python test that feeds the boundary
   TOML accepted by PR #115's Rust tests through `validate_inventory` and
   expects no errors, or a sentence in the closeout explaining why the Rust
   fixtures cannot be reached from Python without copying them.

## Acceptance criteria

- Every finding id above is addressed and listed in `## Closeout` with the
  commit that fixed it.
- No doc contradicts ADR-004 or REQ-SCB-012/013/020.
- `just lint` and `just test` pass locally (PR CI does not run for
  non-develop bases); `git diff --check` clean.
- Changes confined to this layer. No rebase, no `gh stack sync`; merge the
  parent forward if it moves.
- Frontmatter `status: complete`; bead `lint-spx.19` claimed with task start
  and closed with task close.

## Out of scope

- Pre-existing Python/Rust divergence (`lint-spx.22`) and pre-existing
  inventory debt (`lint-spx.21`).
- Forbidden-edge deserialization and Python parity for #115 (done,
  `lint-spx.18`).

## Closeout

Finding disposition for QA-1:

- SC-QA-001, SC-QA-002, SC-QA-003, SC-QA-007, SC-QA-013, QA-003, and
  ARCH-012: fixed in the plan, requirements, model, and README updates.
- SC-QA-004 and ARCH-009: fixed with explicit CLI configuration-error
  coverage for a present `boundaries/` directory without planning metadata;
  the empty-inventory fixture retains valid planning metadata because the
  discovered workspace root reads it.
- SC-QA-005, SC-QA-009, and SC-QA-010: fixed with planning-header,
  unknown-field, duplicate-dependent, and facade/empty-trait tests.
- SC-QA-011 and ARCH-003: fixed by removing redundant planning validation and
  centralizing `BOUNDARY_ID_PREFIX`.
- QA-002: fixed by correcting the widened implementation-visibility error and
  asserting the diagnostic in a test.
- ARCH-001: fixed as a documented requirement and a `pub(crate)` Rust helper in
  `sc-lint-boundary`; the Python derivation remains unchanged per scope.
- ARCH-009 and ARCH-012: fixed by the restored CLI test and closeout records.
- Residual QA-003 from `lint-spx.18`: fixed by extending the Python parity test
  with the arrow-delimited forbidden-edge form.

The repository's `closing-triage` skill/query script was not present in the
available worktree or local skill catalog, so the assignment's promoted finding
IDs were verified directly against the cited current files.

Round 2 LEAD-001: fixed by moving `BOUNDARY_ID_PREFIX` and
`owner_crate_path_for_package` out of the published `sc-lint-schema` crate and
into `sc-lint-boundary` inventory types. `sc-lint-schema` is untouched in this
round because its published interface must remain rule-neutral and stable.
