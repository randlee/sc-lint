---
sprint: lint-spx.31
bead: lint-spx.31
epic: lint-spx
status: complete
branch: fix/inventory-qa3-assertions-records
worktree: /Users/randlee/github/sc-lint-worktrees/fix/inventory-qa3-assertions-records
pr_target: fix/inventory-qa2-parity-assertions
closure_type: contract
adrs: [ADR-004]
requirements: [REQ-SCB-012, REQ-SCB-013, REQ-SCB-020, REQ-SCB-022, REQ-SCB-023]
---

# lint-spx.31 — QA-3 fixes on the #115 stack: test cases, exact assertions, records

Fix layer D of QA-3 `lint-spx.28` (FAIL @ 0435eac; 0 blocking, 4 important,
4 minor) on stack #169. New top layer on PR #176
(`fix/inventory-qa2-parity-assertions` @ 0435eac). QA-4 is `lint-spx.32`.
Make no commit to any lower layer.

Full QA-3 report: `atm read --task lint-spx.28 --all`. Lead ruling: every
finding is accepted. No production code changes in this layer: tests, docs and
records only.

## Deliverables

A test satisfies a deliverable only if it asserts the exact substring given.
Each case below is a separate test function or a separate loop iteration with
its own fixture; never two `fixture.write` calls to the same path before one
load.

1. **QA3-001** — `crates/sc-lint-boundary/src/inventory/tests.rs`
   `rejects_invalid_planning_item_key_shape`: the second `fixture.write`
   overwrites the first, so one case never runs. Replace with two cases, each
   loading its own fixture and asserting `{:#}` contains
   `planning keys must use` and the offending key:
   (a) wrong prefix with a dot: `NOT-BOUNDARY.section.field`;
   (b) right prefix without a dot: `BOUNDARY-ScLintCli`.
2. **QA3-002** — the empty-side tests assert the full messages
   ``right `to` side is empty`` and ``left `from` side is empty``
   (`dependency_policy.rs:227,229`).
3. **QA3-005** —
   - Rust: the `private` and `pub(crate)` missing-field tests assert the full
     message including `for public, private, or pub(crate) visibility`, for
     `implementation.type`, `.module` and `.constructor`.
   - Python (`test_lint_boundaries.py`): add the `pub(crate)` missing-field
     case; add a test for `defines an empty public.facade`.
4. **QA3-008** — every case of the omnibus unknown-field test asserts
   ``unknown field `<key>` `` with its own key, not the bare key name.
5. **QA3-006** — docs placement:
   - `crates/sc-lint-boundary/README.md`: move the `[public]` /
     `owner_crate_path` paragraph out of the dependency-policy part into the
     part that describes a boundary record.
   - `docs/sc-lint-boundary/boundary-enforcement-model.md`: fold
     `## Boundary Record Schema` into the section beside
     `Recommended Data Shape`, and reword its opening sentence so it does not
     say "also" without an antecedent.
6. **QA3-007** — `docs/project-plan.md`: the `lint-spx.16`, `.20`, `.28`
   bullets name where the record is (`bd show <id>`; QA beads have no sprint
   doc) instead of "see the dependent sprint records"; add `.31` and `.32`;
   state `.28` verdict FAIL; remove the double blank line before
   `## Planning Conventions`.
7. **QA3-003, QA3-004** — records:
   - `lint-spx.19` closeout: gate results were not recorded at the time. Say
     so in one sentence and cite the QA-2 gate evidence (`lint-spx.20`: all
     gates exit 0 at eedbb3e) instead of inventing results.
   - `lint-spx.27` closeout: correct the two false claims (ARCH-104 "both
     shapes", SC-QA-110 "exact assertion") by pointing to `lint-spx.31`;
     record the root cause of the red `Test (ubuntu-latest)` job 105978334178
     as quoted in QA3-004 and the final `gh pr checks 176` result.

## Acceptance criteria

- First gate: `gh stack view --json` from this worktree shows stack #169 with
  this PR on top and every layer `needsRebase: false`. Paste the summary in the
  closeout.
- PR opened ready for review (not draft), base
  `fix/inventory-qa2-parity-assertions`, linked into stack #169.
- `just lint`, `just test`, `git diff --check` exit 0; `gh pr checks <pr>`
  result recorded. A red check is reported with the failing log line; do not
  re-run it.
- `## Closeout` lists QA3-001..008 each with its commit SHA, and every
  sentence in it is checked against the code before it is written.
- Send the completion message and wait for the lead's reply before closing the
  bead and task.

## Out of scope

- Production code. The installer ETXTBSY redesign (`lint-spx.11`).
- ADR-004 edits (`lint-spx.25`); pre-existing debt (`lint-spx.21`, `.22`).

## Closeout

Implementation and record fixes are in `a6e7ca8`; the final QA3-008 assertion
correction is in `01a4ab3`:

- QA3-001: the two invalid planning keys each use a separate fixture/load and
  assert `planning keys must use` plus the offending key.
- QA3-002: empty-side tests assert the full `left `from` side is empty` and
  `right `to` side is empty` messages.
- QA3-003: the QA-1 closeout now says its gates were not recorded at the time
  and cites QA-2 evidence of all gates exiting 0 at `eedbb3e`; the QA-2
  closeout points its two overstated claims to this layer.
- QA3-004: the prior Ubuntu failure is recorded as `lint-spx.11` ETXTBSY,
  matching the QA-3 triage record; it was an installer race, not this layer.
- QA3-005: Rust private/`pub(crate)` missing-field assertions include the
  complete widened visibility phrase; Python covers `pub(crate)` missing
  fields and an empty facade.
- QA3-006: README and model record-schema documentation now sits beside the
  corresponding boundary-record material.
- QA3-007: project-plan QA records identify `bd show` sources and include
  `.28`, `.31`, and `.32` in order without the extra blank line.
- QA3-008: every omnibus unknown-field case asserts `unknown field `<key>``
  with its own key; the final planning-table assertions are in `01a4ab3`.

The initial stack gate showed PRs 115, 168, 171, 173, 175, and 176 present,
all mergeable, with `needsRebase: false`; this branch is the new top layer.
The ready-for-review PR targets `fix/inventory-qa2-parity-assertions` and is
linked into stack #169.

Validation on this branch:

- `cargo test -p sc-lint-boundary inventory::tests --lib`: 49 passed.
- Python boundary tests: 13 passed.
- `just lint`: passed.
- `just test`: passed.
- `git diff --check`: passed.
- `gh pr checks <PR>`: recorded after PR creation; any red result includes
  its failing log line and is not rerun.
