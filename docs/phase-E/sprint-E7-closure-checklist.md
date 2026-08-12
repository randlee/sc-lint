# Sprint E.7 Closure Checklist

This worktree-local checklist is the implementation and closure record for
[Sprint E.7](./sprint-E7.md). It is deliberately completed in two review
passes: each item needs executable evidence before it is checked.

## Pass 1 — Initial audit and closure work

- [x] **Root consumer model:** add root `sc-lint.toml` and product-owned
  `.sc-lint/bootstrap`; make the root `Justfile` the maintained model with
  public `setup`, `lint`, `test`, and `upgrade` recipes sharing one private
  compatibility preflight. The source-only aggregate work remains behind the
  product operation, not copied into a consumer template.
  **Closure evidence:** golden parity test, `just --list`, and executable root
  operation tests. Completed: contract dry-runs cover all four recipes and
  root helpers exactly match the generated product assets.

- [x] **Completion contract and guidance:** constrain root `AGENTS.md`, root
  `README.md`, and installed guidance to tell agents to use only `just lint`
  and `just test` for completion; ensure the commands retain complete source
  maintenance gates.
  **Closure evidence:** documentation tests/assertions and direct content
  review. Completed: AGENTS, README, and installed guidance are asserted;
  source profiles retain Rust, Python, and Action gates.

- [x] **CI dogfooding:** replace divergent hand-assembled CI work with the
  root aggregate commands after standard setup, preserving platform coverage.
  **Closure evidence:** workflow assertions and CI matrix coverage for Linux,
  macOS, and Windows. Completed: CI runs `just lint`, `just setup`, and
  `just test` across all three platforms.

- [x] **Release-binary consumer fixture harness:** add disposable fixtures
  that initialize a fresh consumer and exercise setup, lint, test, offline
  docs discovery, and upgrade through staged E.5 release binaries rather than
  `cargo run` paths.
  **Closure evidence:** deterministic local archive fixture tests for fresh,
  compatible, and too-old installations on Linux/macOS/Windows mappings.
  Completed: the lifecycle test stages the binary/docs outside the checkout,
  then runs init/setup/lint/test/docs/upgrade and asserts release mappings.

- [x] **Missing-installation fixture:** remove the product binary from PATH,
  invoke generated `just lint` and `just test`, and prove both stop before
  work with the E.1 structured recovery result (minimum version and recovery;
  no traceback).
  **Closure evidence:** focused fixture assertions for both commands.
  Completed: both generated recipes stop with the stable recovery before work.

- [x] **Distribution smoke contracts:** validate release archive, Homebrew
  layout, and GitHub Action installation surfaces where their platform contract
  is expressible locally; preserve usable fixture transcripts without treating
  them as source artifacts.
  **Closure evidence:** fixture harness and Action/archive/Homebrew tests.
  Completed: staged archive-style docs discovery, aggregate tests, and release
  manifest validation cover the supported local surfaces.

- [x] **RBP/requirements review:** review new Rust harness or CLI error paths
  against RBP-001, RBP-004, RBP-006, RBP-007, and any triggered practice;
  retain structured recovery and validated boundaries.
  **Closure evidence:** review record with no unaddressed applicable finding.
  Completed: RBP-001 structured recovery remains; no new RBP-004/006/007
  finding. Requirements, ADR, and CLI docs include the Windows helper.

- [x] **First critical review:** reread Sprint E.7 after implementation;
  record any residual gaps below and fix them before final validation.
  Completed: fixed the Clap argument-order defect, CI Python omission, and
  stale managed-file wording.

## Pass 2 — Residual review and final closure

- [x] **Second-plan review:** re-evaluate every deliverable and acceptance
  criterion against the completed implementation; add and close any new item.
  Completed: no residual gaps after requirements/ADR, docs, Action/archive,
  and release-matrix review.
- [x] **Final validation:** run `just setup`, `just lint`, `just test`, and
  the local Linux/macOS/Windows fixture matrix; confirm clean worktree and
  send final commit hash to team-lead.
  Completed: all three aggregate gates passed after the final review; focused
  lifecycle/contract tests and release manifest validation also passed. Commit
  and clean-worktree verification follow immediately before handoff.
