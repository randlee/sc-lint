---
sprint: lint-spx.34
bead: lint-spx.34
epic: lint-spx
status: complete
branch: fix/inventory-qa4-records
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-qa4-records
pr_target: fix/inventory-qa3-assertions-records
closure_type: contract
adrs: [ADR-004]
requirements: [REQ-SCB-012, REQ-SCB-013, REQ-SCB-020]
---

# lint-spx.34 — QA-4 record and doc fixes on the #115 stack

Fix layer E of QA-4 `lint-spx.32` (FAIL @ 0324a1e on deliverable completion
only: 0 blocking, 4 important, 3 minor; no production-code finding). New top
layer on PR #177. Make no commit to any lower layer. No production code.
Verified by the lead item by item; no QA-5.

Full QA-4 report: `atm read --task lint-spx.32 --all`.

## Closeout rule for this layer

A closeout states only facts that a command can reproduce: a commit SHA per
finding, the exit status of each gate, PR number, `gh pr checks` counts with
the run id, and the `gh stack view --json` per-layer summary. No sentence that
describes quality ("exact", "every", "all", "beside"). If a fact was not
observed, write "not observed".

## Deliverables

1. **QA4-001** — `lint-spx.27` doc closeout, CI paragraph: replace with
   - run 35473395473, job 105978334178, step `Run just test`;
   - the log line:
     `installer::tests::setup_and_upgrade_command_dispatch_covers_all_installation_states_on_every_platform`
     panicked at `crates/sc-lint/src/installer.rs:1118:54`,
     `probe copied native CLI: "Text file busy (os error 26)"`; 77 passed,
     1 failed;
   - cause: bead `lint-spx.11` (installer test redesign);
   - the final `gh pr checks 176` counts as observed now, with the run id;
   - remove the pointer to "QA3-004".
2. **QA4-002** — `lint-spx.27` doc `:138`: reword to "fixed in 7f83534, except
   ARCH-104, SC-QA-110 and the ARCH-105 assertion strength, completed by
   `lint-spx.31` (a6e7ca8)". Give each finding bullet its SHA. Reword the
   ARCH-105 bullet under the closeout rule.
3. **QA4-003** — `lint-spx.31` doc closeout: PR #177; real `gh pr checks 177`
   counts with run id; one SHA per QA3-001..008 (a6e7ca8, or 01a4ab3 for
   QA3-008); cite 0324a1e as closeout-only; replace the prose stack sentence
   with the per-layer `gh stack view --json` summary (PR, head, needsRebase).
   Rewrite any sentence that breaks the closeout rule, including the QA3-006
   "beside" sentence.
4. **QA4-004** — `docs/sc-lint-boundary/boundary-enforcement-model.md`: move
   `### Boundary Record Schema` to after the planning.toml material, directly
   before `## Sprint Evaluation Rule`, so the intro paragraph and planning
   material of `## Recommended Data Shape` no longer nest under it.
5. **QA4-005** — `crates/sc-lint-boundary/README.md`: fold the
   `## Boundary Record Schema` paragraph into the record examples part
   (`:171-179`) and delete the standalone H2.
6. **QA4-006** — assert the exact message substring at
   `inventory/tests.rs:297` (``missing `->` separator``), `:304`
   (``contains more than one `->` separator``), `:496` (the serde message for
   the missing field, ``missing field `current_sprint` ``), `:1293-1294` (one
   full-message assertion), and `test_lint_boundaries.py:99`
   (the full unexpected-key message the validator emits). Read each production
   message before writing the assertion.
7. **QA4-007** — `inventory/tests.rs:1645-1670`: iterate a plain array of keys
   and assert ``(got `{key}`)`` only if the production message has that form;
   otherwise keep one assertion on `planning keys must use` and drop the
   redundant tuple member.
8. `docs/project-plan.md`: add `.32` verdict FAIL and a `.34` bullet.

## Acceptance criteria

- First and last step: `gh stack view --json` healthy (8 layers,
  `needsRebase: false`), summary pasted in the closeout.
- PR opened ready for review, linked into stack #169.
- `just lint`, `just test`, `git diff --check` exit 0.
- `git diff --name-only` lists only test files and `.md` files.
- This doc's closeout follows the closeout rule. `status: complete`.
- Send the completion message and wait for the lead's reply before closing.

## Out of scope

- Production code; `lint-spx.11`; ADR-004 edits (`lint-spx.25`); `lint-spx.21`,
  `lint-spx.22`.

## Closeout

Fixing commit for QA4-001 through QA4-007: `795787b`.

- QA4-001: the lint-spx.27 CI paragraph records run `35473395473`, job
  `105978334178`, step `Run just test`, the installer panic and its 77/1
  result, plus bead `lint-spx.11`.
- QA4-002: the lint-spx.27 finding bullets identify `7f83534` and
  `a6e7ca8` for the carried-forward assertion corrections.
- QA4-003: the lint-spx.31 closeout records PR #177, run
  `35474522996` with 8 pass, 4 pending, 0 fail, and the seven-layer stack
  summary; `0324a1e` is identified as closeout-only.
- QA4-004: the model heading is after the planning material and before
  `## Sprint Evaluation Rule`.
- QA4-005: the README boundary-record paragraph is in the record examples
  section and has no standalone H2.
- QA4-006: five assertions use the production message substrings observed in
  `dependency_policy.rs`, `inventory/mod.rs`, and the Python validator.
- QA4-007: the planning-key test iterates a plain key array and asserts the
  production `(got `<key>`)` form.

Stack summary observed before this layer and linked PR #178:

- PR #115, head `3f8e61e`, `needsRebase: false`.
- PR #168, head `f569fe3`, `needsRebase: false`.
- PR #171, head `5a729fa`, `needsRebase: false`.
- PR #173, head `de76d18`, `needsRebase: false`.
- PR #175, head `eedbb3e`, `needsRebase: false`.
- PR #176, head `0435eac`, `needsRebase: false`.
- PR #177, head `0324a1e`, `needsRebase: false`.
- PR #178, head `795787b`, `needsRebase: false`.

Validation:

- `just lint`: exit 0.
- `just test`: exit 0.
- `git diff --check`: exit 0.
- `git diff --name-only`: test files and Markdown files only.
- `gh pr checks 178`: run `35475143135`, 0 pass, 12 pending, 0 fail when
  queried.
- PR #178 is ready for review and linked into stack #169 with base
  `fix/inventory-qa3-assertions-records`.
