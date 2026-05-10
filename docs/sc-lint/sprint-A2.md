# Sprint A.2 — Profiles And Windows Preflight

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.2"
worktree: <repo-root>
branch: feature/sprint-A2
status: completed
estimated_scope: M
```

## Goal

Implement product-level `fast`, `full`, and `ci` lint profiles and add
capability-driven `xwin` Windows preflight support without changing CI parity
semantics.

## Scope Summary

This sprint operationalizes the profile and `xwin` strategy that is already
documented. It must produce a stable developer-facing command model that
distinguishes:

- `sc-lint lint fast`
- `sc-lint lint full`
- `sc-lint lint ci`
- `sc-lint ci`
- `sc-lint check xwin`
- `sc-lint clippy xwin`

## Governing Requirements

- `REQ-PRODUCT-006C`
- `REQ-PRODUCT-006D`
- `REQ-PRODUCT-006E`
- `REQ-PRODUCT-012A`
- `REQ-PRODUCT-012B`
- `REQ-PRODUCT-012C`
- `REQ-PRODUCT-012D`
- `REQ-PRODUCT-012E`
- `REQ-PRODUCT-016A`
- `REQ-CLI-007`
- `REQ-CLI-007B`
- `REQ-CLI-007C`
- `REQ-CLI-007D`
- `REQ-CLI-007E`
- `REQ-CLI-008B`
- `REQ-CLI-008C`
- `REQ-CLI-014`
- `REQ-CLI-015`
- `REQ-LOG-004`
- `REQ-LOG-005`

## Governing ADRs

- `docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md`
- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`

## Governing Boundaries

- `BOUNDARY-ScLintCli`

## Prerequisites

- Sprint A.1b complete with a functioning top-level CLI crate
- one top-level delegated command already normalized through the CLI

## Hard Dependencies

- profile semantics must remain defined at the product level, not only in
  `Justfile`
- `xwin` support must remain optional local enhancement and must not redefine
  what `ci` means

## Non-Goals

- replacing real Windows CI with cross-target preflight
- adding `xwin` to the `ci` lint profile
- porting Python utilities to Rust

## Primary Targets

- `crates/sc-lint/`
- `Justfile`
- `.just/run_lint.py`
- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/sc-lint/roadmap.md`
- `docs/requirements.md`
- `boundaries/planning.toml`

## Sub-Tasks

1. Implement `LintProfile` and `OutputMode`
   Development work:
   - add `LintProfile::{Fast, Full, Ci}`
   - add `OutputMode::{Human, Json}`
   - define stable membership and dispatch rules for each profile
   - define stable output-mode selection rules for human and `--json` paths
   - expose `sc-lint lint fast|full|ci`
   Required tests:
   - profile parsing tests
   - profile membership tests
   - output-mode parsing and serialization tests
   Required doc or boundary updates:
   - update CLI/profile docs if the implemented subcommand shape differs
   - keep `boundaries/planning.toml` and `docs/sc-lint/cli-contract.md`
     aligned with the implemented `OutputMode`

2. Implement top-level `sc-lint ci`
   Development work:
   - define `sc-lint ci` as lint + tests
   - keep `sc-lint lint ci` lint-only
   Required tests:
   - tests proving `sc-lint ci` and `sc-lint lint ci` differ only by the test
     execution layer
   Required doc or boundary updates:
   - keep `REQ-PRODUCT-016A` traceability aligned

3. Implement `xwin` capability detection
   Development work:
   - detect `cargo xwin`
   - expose `sc-lint check xwin`
   - expose `sc-lint clippy xwin`
   - ensure `full` includes `xwin` only when installed
   - ensure `fast` remains strictly zero-network/low-latency without `xwin`
   - keep `xwin check` as an explicit command rather than default `fast`
     membership until a later policy change is approved
   Required tests:
   - capability-present tests
   - capability-absent tests
   - profile-behavior tests for with/without `xwin`
   Required doc or boundary updates:
   - update profile docs if capability behavior narrows further

4. Plan `xwin` preflight logging
   Development work:
   - log one entry event for `sc-lint check xwin` and `sc-lint clippy xwin`
     including the effective target/config selection
   - log one completion event with verdict and elapsed time in ms
   - log one error event per `CliError` on capability or preflight failure
   Required tests:
   - doc review for entry/exit/error event consistency with
     `docs/sc-lint/logging.md`
   Required doc or boundary updates:
   - keep the logging design aligned with the `xwin` command path

5. Document rule-disable behavior
   Development work:
   - define how rules are disabled in the new CLI (source vs config)
   - document the disable policy for the first shipped backends
   Required tests:
   - doc review for consistency
   Required doc or boundary updates:
   - ensure initial per-tool guides (even if draft) reflect this policy

6. Align repo-local wrappers with CLI semantics
   Development work:
   - ensure `just` wrappers call the intended `sc-lint` profile commands
   - keep CI semantics explicit and independent from `xwin`
   Required tests:
   - `just lint` profile mapping checks
   Required doc or boundary updates:
   - update local development gate documentation if wrapper names change

## Split Recommendation

Keep A.2 together. Profiles and `xwin` capability behavior are tightly coupled;
splitting them would create temporary contradictions about what `fast`, `full`,
and `ci` actually mean.

## Acceptance Criteria

- `sc-lint lint fast`, `full`, and `ci` exist and are documented
- `OutputMode::{Human, Json}` is implemented and documented for the top-level
  CLI
- `sc-lint ci` exists and includes tests
- `sc-lint check xwin` and `sc-lint clippy xwin` exist when `cargo xwin` is installed
- the `full` profile includes `xwin` when present, but `fast` and `ci` remain independent from `xwin`
- `xwin` preflight command paths log entry, completion, and error events
  through the standard CLI event pattern
- real Windows CI remains the authoritative release gate
- the cross-target preflight strategy document exists and is approved before
  A.2 closes

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`

## Required Document Updates

- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/sc-lint/README.md`
- `docs/sc-lint/roadmap.md`
- `docs/project-plan.md`
- `docs/requirements.md`

## Risks And Watchouts

- do not let capability absence break unrelated local lint flows
- do not make `xwin` part of the `ci` lint profile
- do not bury profile semantics inside wrapper scripts where the product docs
  cannot govern them
