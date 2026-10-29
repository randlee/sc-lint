# Changelog

All notable changes to sc-lint are documented here.

## [Unreleased]

### Added

- Release archives and Python wheels cover Linux, macOS and Windows on both x86_64 and ARM64. Managed setup and upgrade select the new Linux and Windows ARM64 archives.
- l-1: Portability checks now account for additional platform `cfg` forms, avoid
  reporting a `USER` read when the same function reads `USERNAME`, ignore
  Windows namespace path prefixes, and exempt private helpers in paired
  platform-adapter files.
- l-2: Boundary external composition-root checks authorize dependents using
  each record's `allowed_dependents` policy only. Consumers that listed an
  external composition root only in `allowed_dependencies` must move it to
  `allowed_dependents`.
- l-3: Cycle analysis recognizes `self` method-call edges and delegation between
  distinct methods; path exclusions also apply to path-less cycle findings.
- l-4: Repository-root discovery recognizes workspace manifests, and Python lint
  tables resolve independently across the supported config files. The
  `line-counts` and `identity-literals` checks now fail with
  `config.table_missing` when no supported config file defines their
  `line_counts` or `identities` table. Consumers running either check must add
  its table to
  `sc-lint.toml` or `.just/lint-config.toml`.
- l-5: Adoption template files rendered by the installer use `.tmpl`; upgrades
  retire unmodified legacy `.j2` files and report modified files as conflicts.
- l-7: Tokio-dependent packages are checked for blocking `std::thread::sleep` calls
  in production targets by `SCB-RUNTIME-003`.
- l-8: CLI contract checks now document unsupported interface checks, pin the
  command-family JSON envelope, and reject malformed Python adapter payloads
  with `backend_payload_drift`.
- l-8: The CLI rejects unknown `[tool.sc-lint]` configuration keys with
  `config.unknown_table`; supported keys are documented in the CLI contract.
- l-9: Version-sync checks support Python manifests and configurable Winget and
  release-wiring checks. Repositories without a `[version_sync]` table now fail
  with `config.table_missing`; consumers using version sync must add the table
  and configure the checks they need.
- l-10: Cargo-shear no longer reports Rust fixtures consumed by trybuild
  `.pass()` or `.compile_fail()` harnesses as unlinked files.
- Consumer test selectors: `[tool.sc-lint.test]` may be a table of named
  `command` or `group` selectors with `default`, `env`, `cwd`, `platforms` and
  `after_all`; `sc-lint test [<selector> [args...]]` forwards arguments to the
  selected command and `sc-lint test --list` lists the declared selectors
  ([using sc-lint](docs-bundle/using-sc-lint.md#test-selectors)).
- `--fail-fast` for `sc-lint test` and `sc-lint lint ci --consumer` (and the
  bootstrap's `lint`/`test` operations) stops starting cargo-lane and
  `after_all` steps after the first failure.
- `after_all = true` on lint rows, test rows and command selectors runs that
  step after every other step.
- Stable codes `profile.invalid_after_all`, `test.unknown_selector`,
  `test.group_cycle` and `profile.steps_failed`
  ([CLI contract](docs/sc-lint/cli-contract.md#stable-codes)).

### Changed

- Consumer lint and test profiles run independent steps in parallel (steps
  whose `argv[0]` is `cargo` stay serialized in declaration order) and fail
  slow: every step runs and every failed step is reported. This supersedes the
  0.8.0 statement that consumer profiles remain sequential and fail-fast.
- The built-in plan and consumer profiles now fail with code
  `profile.steps_failed` and `error.details.failed_steps[]` of
  `{index, name, exit_code, message}` (zero-based `index`) instead of
  `{index, step, message}`. A consumer failure no longer returns the first
  step's own error, and a missing consumer backend now reports
  `profile.steps_failed` with the spawn error in `message`.

## [0.8.2] — 2026-10-29

Republish of 0.8.1 under a new version number. The 0.8.1 Windows binaries
differed between the prerelease and release builds, so no version number is
shared by two builds. There are no source changes from 0.8.1; 0.8.1 reached
GitHub and crates.io only.

## [0.8.1] — 2026-10-28

Phase L closes the atm-core findings for the 0.8.0 release. Its implementation
and review fixes are carried by the phase integration PR #716.

### Added

- Consumer test selectors support named commands and groups, a default
  selector, per-selector environment and working directory, platform filters,
  forwarded arguments, and `sc-lint test --list` (#196).
- `SCB-RUNTIME-003` reports blocking `std::thread::sleep` in production
  targets of workspace packages that declare Tokio (#82).
- Rust analyzer reports are validated against the bundled
  `analyzer-report-v1` JSON Schema, and Python adapter payloads are checked
  against the `sc-lint-python-v1` transport contract (#586).
- Cargo-shear recognizes trybuild `.pass()` and `.compile_fail()` fixtures as
  consumed files (#647).

### Changed

- Consumer lint and test steps run concurrently and report all failures in
  declaration order. Cargo steps remain serialized; `--fail-fast` stops
  starting eligible steps after a failure (#195).
- Boundary dependency checks use the union of each owner's
  `allowed_dependents`; path-less cycle findings can be excluded using their
  source files. Composition-root, delegation, and per-rule path exclusion
  behavior is corrected (#642, #643, #376, #204, #385; follow-up PRs #717,
  #718, and #722).
- Version-sync checks support Python manifests and configurable Winget and
  release-wiring fields (#137).
- Installer-rendered adoption templates use `.tmpl`; front-matter `.j2`
  remains reserved for sc-compose templates (#604).

### Fixed

- Portability checks honor additional platform `cfg` forms, recognize
  `USERNAME` fallback, ignore Windows namespace path prefixes, and avoid
  reporting private helpers in paired adapter files (#641, #192, #372;
  follow-up PRs #719 and #724).
- GitHub Action inputs accept hyphenated names such as `working-directory`
  and `config-path` (#669; follow-up PRs #721 and #725).
- The reviewed `Clone` default is preserved in cycle analysis, and
  timing-sensitive CI tests use controlled synchronization (follow-up PRs
  #728, #729, and #731).

## [0.8.0] — 2026-10-19

### Added

- Boundary `allowed_dependencies`, `allowed_dev_dependencies`, and
  `allowed_feature_dependencies` accept a single trailing `*` prefix pattern
  (for example `"codex-*"`).
- New `SCB-FEATURE-001` rule (in the `dependencies` family) enforces feature
  independence, with a new optional `allowed_feature_dependencies` field.
- Analyzer `exclude_paths_by_rule` maps exact rule IDs to workspace-relative
  globs and drops only that rule's findings at matching paths; optional and
  inactive when not declared
  ([configuration](docs-bundle/configuration.md)).
- Portability `platform_adapters` declares explicit Unix/Windows module pairs,
  with a review reason, for equivalent implementations whose names differ;
  optional and inactive when not declared
  ([sc-lint-portability](docs-bundle/packages/sc-lint-portability.md)).
- Boundary accepts the optional manifest dependency policy families
  `manifest_dependency_allowlists`, `global_dependency_ownership` and
  `manifest_section_rules`, validates inventory policy semantically, and
  reports stale dependent permissions; inactive when not declared
  ([sc-lint-boundary](docs-bundle/packages/sc-lint-boundary.md#dependency-and-manifest-policies)).
- Consumer named lint profiles (`[tool.sc-lint.lint_profiles]`) and
  `--platform-scope` (`host` by default, or `all`, `linux`, `macos`,
  `windows`) select which declared lint steps run; the adoption kit renders
  platform-aware profiles and CI resolution
  ([using sc-lint](docs-bundle/using-sc-lint.md#profiles-platform-scope-and-declared-tools)).
- `.sc-lint/bootstrap.py exec -- <command>` runs a command in the managed
  product environment, preserving its arguments and exit status; it does not
  run setup or select a profile
  ([Just setup](docs-bundle/just-setup.md#select-lint-steps-and-run-managed-commands)).
- `just setup` provisions only the cargo tools (exact versions) and rustup
  targets a consumer declares under `tools` in `.sc-lint/install.json`; no
  product default chooses tools, and lint never installs them
  ([using sc-lint](docs-bundle/using-sc-lint.md#profiles-platform-scope-and-declared-tools)).

### Changed

- Adoption uses one Python bootstrap, `.sc-lint/bootstrap.py`, identical on
  Linux, macOS and Windows. Recipes run
  `<python> .sc-lint/bootstrap.py <operation> --config sc-lint.toml`
  (`python` on Windows, `python3` elsewhere); on Windows `just` runs recipes
  with `cmd.exe /c`. pwsh is no longer used by recipes, actions or tests
  (ADR-012, ADR-016).
- The built-in source lint plan runs independent steps concurrently and
  continues through all planned steps after failures, reporting every failure
  by plan index in plan order. Consumer-configured lint and test profiles
  remain sequential, declaration-ordered, and fail-fast.
  [Superseded in 0.8.1: consumer profiles run steps in parallel and fail
  slow by default, with opt-in `--fail-fast`; see Unreleased.]

### Compatibility

- `.sc-lint/bootstrap` and `.sc-lint/bootstrap.ps1` are no longer shipped.
  Upgrading the adoption kit removes them when they are unmodified (their
  digest matches the kit's install manifest); a modified copy is reported as
  a conflict (exit 2) and left in place. The bootstrap requires Python 3.11 or
  newer.
- Existing boundary records retain their results; feature independence is opt-in.
  Older binaries reject records declaring `allowed_feature_dependencies`, so
  consumers adopting it must pin a release that supports the field.
- Tightened boundary inventory validation and stale-entry diagnostics can
  reject previously accepted invalid declarations, including
  `SCB-DEPENDENCY-004`, `SCB-BOUNDARY-004`, and `SCB-MANIFEST-006`. Review the
  [boundary inventory and stale-policy recovery steps](docs-bundle/packages/sc-lint-boundary.md#recovering-inventory-and-stale-policy-findings)
  before upgrading.
- Previously accepted path-like cargo-shear
  `allowed_unused_dependencies` keys are rejected with
  `SC_LINT_CARGO_SHEAR_CONFIG_INVALID`. Use the canonical key
  `"{package}.{dependency_key}"` (the dependency rename alias when present,
  otherwise the package name), or `"workspace.{dependency_key}"` only for a
  virtual-root workspace dependency. See the cargo-shear migration guidance
  in [troubleshooting](docs-bundle/troubleshooting.md#cargo-shear-exception-migration).
- The never-constructed public `CountScannedCrates` variants were removed from
  `sc_lint_portability::PortabilityError` and `sc_lint_runtime::RuntimeError`
  in `2a53e91`.

### Fixed

- The cargo-deny lint step retries a DNS resolution failure up to three times
  before reporting it.
- The boundary inventory now rejects nested `planning.toml` files instead of
  silently ignoring planning metadata outside `boundaries/planning.toml`.
- Portability and runtime analysis now report an error when a finding's source
  path cannot be made repository-relative, including paths outside the
  repository root, instead of silently skipping that finding.

## [0.7.3] — 2026-10-12

Fixes found after publishing 0.7.2.

### Fixed

- A lone `sc-lint` now rejects a stale or foreign `sc-lint-*` backend found on
  `PATH` with `BACKEND_EXEC_FAILURE` and the `sc-lint setup` remedy instead of
  running it. The backends gain `--version` so the launcher can check them
  (#407).
- Text output reports `fail` when the verdict is failing, so the rendered
  result matches the exit status and JSON output (#405).

## [0.7.2] — 2026-10-12

Consumer installation and boundary fixes found while adopting 0.7.1.

### Fixed

- Managed setup installs and repairs the complete release bundle (the
  `sc-lint` launcher, its analyzer companions and the documentation bundle)
  instead of the launcher alone, so an older companion on `PATH` is no longer
  selected. This fixes `sc-lint lint sc-runtime` rejecting `--config` (#373).
- Setup checks each managed companion's identity and health with bounded
  startup probes; a missing, corrupt, renamed or hanging companion is repaired
  or rejected, and a failed upgrade keeps the previous complete install
  (#373).
- The boundary analyzer resolves a crate named in a record or dependency edge
  by Cargo package name, then by a unique workspace member directory, then by
  a unique library target name, so records and `forbidden_edges` that use the
  directory name are no longer silently skipped (#375).

### Changed

- An unresolved boundary package reference (an `owner_package`, dependent or
  edge name that matches no Cargo workspace package, member directory or
  library target) now fails with `CLI.CONFIG_ERROR` ("unknown workspace
  package reference ..."). 0.7.1 skipped such entries silently. Consumers must
  name a Cargo workspace package, member directory or library target (#375).

## [0.7.1] — 2026-10-12

Consumer-adoption fixes found while adopting 0.7.0.

### Added

- cargo-shear: `[cargo_shear.allowed_unused_dependencies]` in
  `.just/lint-config.toml` downgrades reviewed unused dependencies, keyed
  `<package>.<dependency>` with a required reason. These compose with the
  existing unlinked/empty-file allowlists, also when `--root` is run from
  another directory (#346).

### Fixed

- The built-in `ci` and `full` lint profiles now run the runtime analyzer
  (#344).
- The packaged pytest runner runs only its bundled tests and no longer
  hard-codes the sc-lint repository's own test paths (#345).
- cargo-shear: crate-root file paths are evaluated, and output sections the
  wrapper cannot parse fail rather than pass (#346).
- identity-literals detects literals split across `concat!`, including
  distinct repeated occurrences and trailing comments (#347).
- The adoption installer rejects unresolved template variables and
  substitutes placeholders written without spaces (#348).
- Python tool runners no longer crash writing captured output to a console
  with a restrictive encoding, such as Windows; only characters the console
  cannot encode are replaced (#349).
- The adoption bootstrap treats an incompatible sc-lint on `PATH` that it did
  not install as unavailable and installs the pinned release; a missing
  managed path no longer breaks the probe (#367).

## [0.7.0] — 2026-10-11

### Added

- Analyzer tables (`[tool.sc-lint.analyzers.<name>]`): every analyzer takes
  `enabled`, `reason`, `exclude_rules`, `fail_rules` and `exclude_paths`;
  portability also takes `config_home_env`, `unix_path_prefixes` and the
  existing `targets` (#201, #202).
- `sc-lint lint sc-boundary|sc-portability|sc-runtime --rule-filter` (#201).
- `unix_path_prefixes` extends the always-enabled built-in portability prefix
  list for PORT-001 and PORT-006; configuration adds detection without
  suppressing defaults.

### Changed

- A key an analyzer table does not define is now a configuration error
  (exit 3) naming the analyzer and the key. Existing kit-rendered configs,
  including `targets`, are unaffected.
- The boundary, portability, and runtime lint commands now exit 10 for a
  lint-fail verdict while retaining their diagnostic output.
- Consumer Justfiles, including `just test`, remain consumer-owned; adoption
  adds its recommended test recipe only when one does not already exist.

## [0.6.0] — 2026-08-30 (cut; unpublished)

### Features

- **Adoption kit** (G.1) — `packages/sc-lint-adoption` installs a
  drift-detectable consumer end state (`install.py --dry-run` exit 1 on drift,
  exit 2 on user-owned conflicts); managed `.sc-lint/install.json`; kit-provided
  `setup-sc-lint` GitHub Action; bootstrap `setup --check/--dry-run` never
  fetches a release.
- **Adoption skill** (G.2) — `sc-lint-adoption` plugin skill, `sc-lint-adopter`
  agent, durable evals, marketplace entry, and `docs/sc-lint/adoption.md`
  (`just test-*` → layers migration, offline docs via `sc-lint docs --path`).
- **Identity-literal Unicode fix** (G.3c).
- Phase F archived (G.0, tag `archive/phase-F`).

### Version

Workspace bumped from `0.5.0` → `0.6.0`; adoption plugin `0.6.0`.

- **Python wheel runtime** (G.3a) — the repository helper scripts ship as the
  `sc-lint` PyPI distribution; the bootstrap provisions `.sc-lint/venv` and
  the CLI runs helpers as `python -m sc_lint.<module>`.
- **Self-contained release** (G.3b) — `full`/`ci` lint profiles invoke only
  archive binaries or wheel helpers (closes #84); `sc-lint version --json`
  reports `self_contained`; CI job `release-smoke` runs the kit recipes from a
  staged archive on every OS. Deferred to the G.1 → G.3b reconciliation
  (after `sprint/G.1-adoption-kit` lands): the kit-copy `cmp` CI check for
  `.sc-lint/bootstrap*` and `just test-adoption`.

### Removed

- The legacy `.just/lint-config.toml` repo-config fallback in the CLI and in
  `sc-lint-portability`; `sc-lint.toml` is the only repo configuration file.

## [0.5.0] — 2026-08-12

### Features

- **Compatibility contract and recovery** (E.1) — consumer repositories declare
  a SemVer minimum in `sc-lint.toml`; `sc-lint` exposes a structured version
  probe and compatibility preflight with documented recovery diagnostics.
- **Safe install and upgrade engine** (E.2) — checksum-verified release
  installation, atomic replacement, rollback, and idempotent setup/upgrade
  operations let consumers recover missing or old installations.
- **Consumer CLI and Just integration** (E.3) — `sc-lint init --just` creates
  the product-owned bootstrap and canonical `just setup`, `just lint`,
  `just test`, and `just upgrade` interface.
- **Distributed operator documentation** (E.4/E.5) — the installed release
  includes an offline documentation bundle, package guides, help discovery,
  and release/Homebrew distribution support.
- **Reusable GitHub Action** (E.6) — a versioned Action installs verified
  release artifacts and runs the consumer contract in CI.
- **Dogfood consumer contract fixtures** (E.7) — the root repository is the
  executable model, with cross-platform release-binary fixtures covering
  setup, lint, test, docs discovery, and safe upgrade migration.
- **New shared analyzer support crate** — `sc-lint-analyzer-support` is
  published for the first time in this release, providing the shared
  `source_scan.rs` and `render.rs` implementation extracted to close the
  Phase E `ARCH-001` duplication finding.

### Changes

- **Consumer adoption boundary** — `just` is the memorable consumer interface;
  installed `sc-lint` owns consumer behavior, diagnostics, and documentation,
  while `cargo run` remains source-checkout development machinery.
- **Release and CI delivery** — Phase E validates consumer delivery across
  Ubuntu, macOS, and Windows, including archive, Homebrew, and GitHub Action
  distribution paths where supported.

### Version

Workspace bumped from `0.4.0` → `0.5.0`. `[workspace.package].version`, the
four pinned internal `[workspace.dependencies]` versions (`sc-lint-boundary`,
`sc-lint-directives`, `sc-lint-schema`, and `sc-lint-analyzer-support`), and `Cargo.lock`
are all synchronized at `0.5.0`.

---

## [0.4.0] — 2026-07-02

### Features

- **Named-caller allowlist enforcement** (`SCB-CALLER-001`) — `BoundaryRecord` now accepts a `CallersSection` with `ApprovedCaller` entries; `analyze_named_callers` and `caller_is_exempt` enforce the allowlist at analysis time
- **Observability boundary policy** (`ADR-009`) — `emit_*` wrapper functions are forbidden in non-CLI crates; `CLI-LAYER-OWNS-LOGGER-INITIALIZATION` boundary rule enforced across the workspace
- **Portability scope and parity** (`ADR-010`) — `sc-lint-portability` designated shared owner for Windows-path, env-var, and shell portability lint rules
- **Full Homebrew toolset** — `sc-lint.rb` primary formula installs all four binaries (`sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, `sc-lint-runtime`); legacy `sc-lint-boundary.rb` maintained for tap compatibility
- **QA process hardening** — triage-first dispatch flow; `scripts/find_todos.py` and `scripts/triage_carry_forward.py` added; oxigraph subprocess mocked in tests
- **Package-level dependency policy enforcement** (`SCB-DEPENDENCY-001/002/003`, closes #73) — boundary TOML `[dependencies]` sections (`allowed_dependencies`, `allowed_dependents`, `forbidden_edges`) are now parsed into validated types (`RawDependenciesSection`, `ForbiddenPackageEdge`, `PackageDependencyPolicy`) at inventory load and mechanically enforced against direct workspace package edges via a new `sc-lint-boundary::package_policy` analyzer and `RuleFilter::Dependencies`; malformed policy records fail inventory loading with actionable `CLI.CONFIG_ERROR` diagnostics

### Changes

- **Security dependency update** — bumped `anyhow` from `1.0.102` to
  `1.0.103` to clear `RUSTSEC-2026-0190` in the `cargo-deny` advisories gate
- **Workspace version inheritance** — all seven crates converted from `version = "..."` to `version.workspace = true`; `[workspace.package] version` is the single source of truth; the three internal `[workspace.dependencies]` path-dependency version pins (`sc-lint-boundary`, `sc-lint-directives`, `sc-lint-schema`) are kept in sync by hand alongside it
- **Standalone binary dispatch** — `run_delegated_backend()` now resolves tool binaries as a sibling to `current_exe()` with PATH fallback; removes build-time `env!("CARGO_MANIFEST_DIR")` dependency that prevented Homebrew installs from running `sc-lint lint sc-portability` and `sc-lint lint sc-runtime`
- **Release gate version check** — `scripts/release_gate.sh` now verifies the release version input matches `[workspace.package].version` in `Cargo.toml` via `tomllib`; `gate-and-tag` CI job pins Python 3.11 via `setup-python`
- **Version consistency regression test** — `crates/sc-lint/src/tests.rs` now asserts every workspace member crate's resolved version matches `[workspace.package].version`, catching an accidental per-crate version override before publish

### Version

Workspace bumped from `0.3.1` (unreleased, last tagged release `v0.3.0`) → `0.4.0`. `[workspace.package].version`, the three pinned internal `[workspace.dependencies]` versions, and `Cargo.lock` are all synchronized at `0.4.0`.

---

## [0.1.0] — 2026-05-11

Initial release.

### Features

- **`sc-lint` CLI** — top-level composition root; `lint` subcommand dispatches to backend analyzers; structured JSON output via `sc-lint-schema`
- **`sc-lint-boundary`** — AST-sensitive boundary analyzer; loads `boundary.toml` inventory; carries dependency-policy metadata such as `forbidden_edges` for later enforcement; `emit_*` rules and `banned_imports`; named-caller groundwork
- **`sc-lint-portability`** — AST-sensitive portability analyzer; Windows-path, env-var, and shell-portability lint rules
- **`sc-lint-runtime`** — AST-sensitive runtime/concurrency analyzer
- **`sc-lint-schema`** — shared machine output schema types used by all analyzer crates
- **`sc-lint-attributes`** — proc-macro crate for source-level declarative lint suppression attributes
- **`sc-lint-directives`** — shared directive parser for proc-macro and analyzer crates
- **Homebrew tap** — `update-homebrew` CI job rewrites `sc-lint.rb` and `sc-lint-boundary.rb` after each release
- **Release workflow** — cross-platform binary builds (macOS intel/arm, Linux x86\_64, Windows); `cargo publish` in dependency order; `publish-artifacts.toml` manifest drives all release steps
