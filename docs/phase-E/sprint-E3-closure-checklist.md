# Sprint E.3 Closure Checklist

This worktree-local checklist records the two required closure-review passes for
Sprint E.3. It is an implementation audit artifact, not a replacement for the
sprint plan.

## Pass 1 — 2026-08-11

- [x] The product owns a canonical thin Just template and bootstrap asset.
  Existing coverage verifies the four public recipe names and excludes source
  implementation names.
- [x] Add `sc-lint init --just`, including idempotent ownership-aware rendering
  of `sc-lint.toml`, `Justfile`, and `.sc-lint/bootstrap`; add non-mutating
  `--check` and `--dry-run`, managed-file reporting, and README/conflict tests.
- [x] Add an explicit consumer execution path. `sc-lint lint ci` currently
  resolves the source checkout and its `.just` scripts, and `sc-lint test` is
  not a command. Consumer execution must instead load its explicit consumer
  configuration and run its configured lint or test profile.
- [x] Model and validate the consumer profile configuration at the config
  boundary, including an empty/malformed-profile recovery error.
- [x] Make consumer mode explicit in the generated command selection; no
  command may infer consumer/source behavior from directory names, Cargo
  manifests, or backend package names.
- [x] Add unit tests proving every configured lint/test member is executed and
  that any member failure fails the aggregate command. Add a missing-backend
  structured-error regression test.
- [x] Add an executable generated-fixture test for `just lint` and `just test`
  that proves the shared compatibility preflight happens before the product
  operation.
- [x] Bring the CLI requirements, architecture, contract, and ADR template
  spelling in line with the implemented command surface and configuration
  schema; update the sprint implementation record once all evidence is green.

## Pass 2 — 2026-08-11

- [x] The embedded sprint/ADR template had stale source-style `lint ci`
  spelling. Updated it to explicit `lint --consumer --config sc-lint.toml ci` and retained the
  shared preflight for every public recipe.
- [x] `init` accepted global `--config`/`--root` even though it always owns the
  canonical paths in the current consumer root. It now rejects those flags
  with a structured usage error; a regression test covers the boundary.
- [x] Added a direct command-context test proving source `lint ci` and
  consumer `lint --consumer --config sc-lint.toml ci` take distinct paths without directory-name
  detection.
- [x] Added a configuration test proving an empty consumer profile fails
  before any command can run.
- [x] Classified the optional consumer CI workflow as not closure-worthy for
  E.3: the Phase E plan assigns release-verified CI installation to E.6's
  reusable Action. ADR-012, CLI requirements, and the sprint non-goals now
  state that boundary explicitly.

All E.3-owned closure items are complete; final workspace validation remains.
