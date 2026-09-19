---
sprint: lint-spx.11
bead: lint-spx.11
epic: lint-spx
status: planned
branch: fix/installer-exec-fixture-race
worktree: /Users/randlee/github/sc-lint-worktrees/fix/installer-exec-fixture-race
pr_target: develop
closure_type: contract
adrs: []
requirements: []
---

# lint-spx.11 — Installer tests: remove the write-then-exec race (ETXTBSY)

Own stack rooted on `develop` (bottom layer, `gh stack init` / `gh stack link
--base develop`). Gates release bead `lint-spx.10`. `adrs` and `requirements`
are empty because this sprint changes test fixtures only; if deliverable 2
finds a production path, stop and report before changing it.

## Failure

`installer::tests::setup_and_upgrade_command_dispatch_covers_all_installation_states_on_every_platform`
panicked at `crates/sc-lint/src/installer.rs:1118` with
`probe copied native CLI: "Text file busy (os error 26)"` on
`Test (ubuntu-latest)`: run 35467775425 (PR #166), PR #164 @ e5ebb50, and run
35473395473 job 105978334178 (PR #176).

## Root cause

`fs::copy` (`installer.rs:1117`) and `write_probe` (`:1324`, `File::create`)
open a write fd on a file inside the multi-threaded test process. A
`Command::spawn` on another test thread forks while that fd is open; the child
holds the inherited fd until its own `exec`. An `exec` of the written file in
that window fails with ETXTBSY. Rust opens files `O_CLOEXEC`, which closes the
fd at `exec`, not at `fork`, so the window exists.

## Design rule

The test process never holds a write fd to a file that is later executed.

Forbidden: retry on ETXTBSY, sleeps, `serial_test` or any serialization
attribute, `--test-threads=1`, a global test mutex, CI reruns.

## Deliverables

1. `:1117` — do not copy the built CLI. Place it with `fs::hard_link` (same
   filesystem: create the install dir under the directory that holds the built
   binary, or under `CARGO_TARGET_TMPDIR`), falling back to a symlink on Unix
   only if the installer under test accepts a symlinked managed binary. On
   Windows the race does not exist (no fork); keep one code path if it works on
   all three hosts, otherwise `cfg`-split with a comment stating why.
2. `write_probe` (`:1324`) and its 7 call sites (`:986`, `:1027`, `:1028`,
   `:1167`, `:1176`, `:1303`): the script is written by a child process, not by
   the test process (for example `sh -c 'cat > "$1" && chmod 755 "$1"'` with
   the text on stdin), so no write fd to it ever exists in the test process.
   Then inventory every path by which a test-written or installer-written file
   is executed in the same process, including production code reached from
   tests (the installer copying a payload into the install dir and then
   probing its version). List each path with `file:line` in the closeout and
   state for each why it cannot hit ETXTBSY. If a production path can, stop and
   report it to the lead; do not change production code in this sprint.
3. Search the workspace for the same pattern outside `installer.rs`
   (`fs::copy`, `File::create`, `fs::write` followed by `Command` on the same
   path in test code, Rust and Python). Fix each hit by the same rule or list
   it as not affected with the reason.
4. Regression evidence, not a new flaky test: a stress run on Linux that
   reproduces the failure on `origin/develop` and not on this branch. Record
   the command, the iteration count and both results in the closeout (for
   example 200 iterations of `cargo test -p sc-lint installer::tests` with the
   default thread count). If it cannot be reproduced on `develop` on the
   available host, write "not reproduced" with the command and count; do not
   claim a before/after.
5. A comment at each fixed site stating the rule in one sentence.

## Acceptance criteria

- First and last step: `gh stack view --json` from this worktree; summary in
  the closeout.
- PR opened ready for review (never draft), base `develop`, in its own stack.
- `just lint`, `just test`, `git diff --check` exit 0; `gh pr checks <pr>`
  counts with run id. Do not re-run a red check; report its log line.
- `grep` evidence in the closeout: no forbidden mechanism was added.
- Closeout states only reproducible facts: SHAs, exit codes, counts, run ids.
  `status: complete`.
- Send the completion message and wait for the lead's reply before closing.

## Out of scope

- Production installer changes (report only). The skill stack #172 and the
  #115 stack #169.
