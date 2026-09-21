---
sprint: lint-spx.9
bead: lint-spx.9
epic: lint-spx
status: planned
branch: docs/release-notes-0.6.0-boundary
worktree: /Users/randlee/github/sc-lint-worktrees/docs/release-notes-0.6.0-boundary
pr_target: develop
closure_type: contract
adrs: [ADR-004]
requirements: []
---

# lint-spx.9 — RELEASE-NOTES-0.6.0.md: boundary fixes and graph schema 0.2.0

Own stack rooted on `develop` (`gh stack link --base develop`). Docs only:
`release/RELEASE-NOTES-0.6.0.md` and this record. `requirements` is empty
because no requirement is implemented; say so in the closeout.

## Source of facts

Every sentence added must be traceable to one of: a merged PR listed below
(`gh pr view <n> --json title,body`), a sprint doc under
`docs/plans/release-0.6.0/`, or the code at `origin/develop`. No sentence
about behaviour that you did not read in the diff or a sprint doc.

Merged to `develop` on 2026-09-20 since the 2026-08-30 note:

- boundary inventory stack: #115, #168, #171, #173, #175, #176, #177, #178
- trait identity stack: #159, #160, #163, #164
- installer test redesign: #180
- CI on every PR target: #182
- orchestration tooling (`.claude/**` only, not user-facing): #166, #167,
  #170, #174, #179, #181 — one line under a "Repository tooling" heading at
  most; do not describe skills.

## Deliverables

1. `## Release`: date refreshed to the day you commit; keep the other fields.
2. `## Summary`: one added paragraph: boundary-model fixes and graph schema
   `0.2.0`.
3. `## Major Changes`, new bullets (one each, with PR numbers):
   - reference impl owners without graph identity collisions (#159);
   - trait methods distinguished from inherent methods and qualified forwarding
     edges (#160, #164);
   - boundary graph `schema_version` `0.1.0` → `0.2.0`
     (`crates/sc-lint-boundary/src/lib.rs:42`, #163);
   - structured boundary inventory: strict forbidden-edge deserialization,
     required `planning.toml`, `owner_crate_path` check, preserved edge parse
     causes, Python validator parity (#115, #168, #171, #173, #175–#178).
4. `## Migration Notes`: the schema `0.2.0` rebaseline: what a consumer with a
   committed `0.1.0` graph must do. Read #163's diff and the boundary docs for
   the actual procedure; if none is documented, write the exact command
   sequence you verified against the code and cite `file:line`.
5. `## Validation`: replace the Phase G sentence with the CI facts of the
   merges: `develop` head SHA, and `gh run list --branch develop --limit 1`
   status.
6. `## Follow-Up Items`: keep; add the installer ETXTBSY redesign (#180) as a
   test-only change under a `## Test Infrastructure` or similar heading rather
   than Major Changes.
7. Do not edit the `## Included Crates` block (generated).

## Acceptance criteria

- `gh stack view --json` first and last; summary in the closeout.
- PR ready for review (never draft), base `develop`, in its own stack.
- `just lint`, `git diff --check` exit 0; `gh pr checks <pr>` counts + run id.
- `git diff --name-only` lists only the two files.
- Closeout: one line per deliverable with the fact source (PR number or
  `file:line`). `status: complete`.
- Completion message, then wait for the lead's reply before closing.
