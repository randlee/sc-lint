# Changelog

All notable changes to sc-lint are documented here.

## [Unreleased]

## [0.8.5]

This release closes GitHub issues #640, #833, #834, #836, #837, #843 and
#844.

### Added

- Runtime rules reject thread `sleep_until` in production Tokio packages,
  Tokio timer sleeps, Runtime/Handle/futures `block_on`, and std Mutex guards
  held across `.await` (`SCB-RUNTIME-004`–`007`, #640, #838). Test code is exempt;
  function/method allows require a non-empty reason. `yield_now` stays legal.
- `SCB-RUNTIME-003` supports reason-required function/method allowances through
  the shared directive path. Intentional synchronous helpers can use
  `#[sc_lint_attributes::sc_lint(allow(SCB-RUNTIME-003), reason = "...")]`
  without suppressing sibling or nested functions (#836).
- Generated command-line references for `sc-lint`, `sc-lint-boundary`,
  `sc-lint-portability` and `sc-lint-runtime`, in Markdown and HTML, ship in
  the offline bundle; `sc-lint docs cli-reference` prints the index. Tests
  regenerate them from the real clap definitions and fail on drift.
- Every `sc-lint` command, option and target value has help text, and
  `-V` is a short form of `--version`. The `sc-lint ci` help says that
  in a source checkout it runs the source-maintainer profile, and that
  `sc-lint lint ci --consumer` selects the consumer CI profile.
- `sc-lint docs` accepts `README`, `README.md` and `overview` for the
  overview and `setup` as an alias of `installation`, so the
  `Docs: sc-lint docs setup` pointer in setup and compatibility errors
  resolves.
- The GitHub Action writes each failure as a `::error` annotation titled with
  its stable code, with the recovery text in the message.
- `python -m sc_lint.run_all`, `run_fmt` and `source_venv` answer `--help`;
  `print_help` lists only recipes that exist in the `Justfile`. The adoption
  installer's `--help` describes `--input`, `--dry-run`, the persisted input
  and its exit codes.
- Every crate on crates.io ships its `LICENSE` and a README that points to
  the primary `sc-lint` crate and the repository; each crate has
  crate-level rustdoc.
- The version-sync check has an opt-in `[version_sync.release_docs]` table
  (`enabled`, `files`, `patterns`) with findings `release_docs.version` and
  `release_docs.files`. sc-lint enables it for its own repository, so a stale
  version literal in the README, crate manifests or the offline docs bundle
  fails the check (lint-085.50).

### Changed

- The Python helpers and consumer bootstrap support Python 3.9 and newer
  (`requires-python = ">=3.9"`, matching the abi3-py39 wheels, GitHub #778).
  They parse TOML with the workspace `toml` crate through
  `sc_lint._native.toml_loads` instead of `tomllib`. A source checkout that
  has not built the native extension yet (before `just setup`) falls back to
  `tomllib`, which needs Python 3.11 or newer; with neither available, parsing
  raises `ImportError` with a recovery message. Windows ARM64 wheels still
  require Python 3.11.
- Runtime guard tracking handles diverging branches, match scrutinee ownership,
  rest/or patterns and nested reference bindings more accurately (#840).
- Document REQ-PRODUCT-004E as Accepted with mandatory-reason per-function
  opt-outs (#840).
- Runtime test exemption: a source file whose inner attributes mark it
  test-only (`#![cfg(test)]` or `#![cfg(all(test, ...))]`) is test code for
  every runtime rule, `SCB-RUNTIME-001` through `007`, exactly like a file
  under `tests/`. An out-of-line `#[cfg(test)] mod checks;` therefore needs
  `#![cfg(test)]` at the top of `checks.rs`; the earlier documented
  limitation that such files were linted as production code is removed
  (#840).
- Every runtime rule, `SCB-RUNTIME-001` through `007`, uses one test-scope
  classifier from `sc-lint-analyzer-support`, which recognizes `#[test]`,
  qualified test attributes such as `#[tokio::test]`, `#[cfg(test)]` and
  compound test cfgs such as `#[cfg(all(test, ...))]` (#840, lint-085.46).
- `SCB-RUNTIME-003` recovery text now recommends event-driven
  synchronization or `yield_now` instead of `tokio::time::sleep(..).await`,
  which `SCB-RUNTIME-005` now reports (#838).
- Runtime import aliases, including `self::`, `super::` and `crate::`
  imports, resolve per module and function scope for `SCB-RUNTIME-003`
  through `007`. A `use ...::sleep` in one module no longer leaks into a
  sibling module, so results no longer depend on item order, and statics
  imported through `crate::`, `super::` (including sibling modules such as
  `use super::a::M`), `self::` or a glob stay tracked for `SCB-RUNTIME-007`.
  `SCB-RUNTIME-003` also resolves renamed `self` imports such as
  `use std::thread::{self as thread}` (lint-085.39, lint-085.46,
  lint-085.52).
- The bootstrap usage lists each operation's own flags: `setup` accepts only
  `--dry-run`, and `--check` belongs to `upgrade`. `setup --check` is now a
  bootstrap usage error instead of being forwarded to `sc-lint setup`.
- Bootstrap `lint`, `test` and `exec` resolve the command's first word with
  `shutil.which` against PATH before running it. On Windows the child
  process otherwise searches the parent Python's directory first and can run
  a host interpreter instead of the managed venv tool (#841).
- `sc-lint init` requires `--just`.
- Files generated by `sc-lint init --just` start with
  `# Generated by sc-lint; user edits are preserved by future init runs.`,
  and the generated `sc-lint.toml` notes that its lint and test steps are
  user-owned.
- `sc-lint init --just` compares managed files after normalizing line
  endings, so a generated file checked out with CRLF (for example by Git on
  Windows) is recognised as current or version-only stale instead of a
  user-owned conflict. Content edits still make a file user-owned. The
  generated `sc-lint.toml` and `Justfile` assets are pinned to LF in
  `.gitattributes` (lint-085.55).
- A config whose `lint` profile has no steps now loads; `sc-lint lint ci
  --consumer` then fails with a recovery that asks for a
  `[[tool.sc-lint.lint]]` row, instead of the whole config being rejected.
- Command-line usage errors show the usage of the subcommand that failed and
  suggest `sc-lint <subcommand> --help`.
- Compatibility errors give specific recovery: a missing config suggests
  `sc-lint init --just`, a malformed config asks to correct the reported
  error. A `--root` path that does not exist is reported as such, with a
  recovery.
- An unknown `--rule-filter` selector error lists the valid selectors for
  that analyzer.
- **Behaviour-breaking:** portability site-allowance errors are reported
  together. An unsupported portability rule id (any `PORT-*` id other than
  `PORT-010`) is reported as `DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE` instead of
  `PORTABILITY.UNKNOWN_RULE`. Every site-allowance diagnostic across all
  scanned files is reported in one failure, sorted by path and line, as
  `<path>:<line>: <DIRECTIVE.SITE_ALLOW code and message>`; the
  `PORTABILITY.INVALID_SITE_ALLOW` prefix is gone. The exit code stays 3.
  Tooling that matches `PORTABILITY.UNKNOWN_RULE` or
  `PORTABILITY.INVALID_SITE_ALLOW` must match the `DIRECTIVE.SITE_ALLOW.*`
  codes instead (lint-085.52, lint-085.57).
- Site-allowance attributes that are not valid Rust attribute syntax report
  `DIRECTIVE.SITE_ALLOW.SYNTAX_ERROR`; `UNKNOWN_ELEMENT` now means only an
  unsupported element. `EMPTY_REASON`, `MALFORMED_RULE_ID` and `UNKNOWN_RULE`
  use one recovery text each (lint-085.52).
- **Behaviour-breaking:** a Python helper that fails to start (no stdout,
  error on stderr) is reported as a backend failure (exit 5) that carries the
  stderr and suggests `just setup`. It was a backend protocol error (exit 6).
  Scripts that test for exit 6 from `sc-lint view findings` or another
  Python-backed command must also accept 5.
- `sc-lint init --just` human output reports `init: <status>` (`current`,
  `would_create` or `created`) followed by `Created:` (`Would create:` under
  `--dry-run`), `Preserved:` and `Version-only stale:` file lists, instead of
  `init: ok`.
- `sc-lint docs --path` prints the bare path without the
  `Documentation root:` prefix, so the output can be used directly in
  scripts; with a guide name, the bare path precedes the guide text.
- The GitHub Action runs on Node 24. Its description and `operation` input
  state that `setup` only installs and preflights the release, and list the
  six supported runners.
- The adoption kit keeps its `skills/` and `agents/` at the plugin root, where
  Claude Code's plugin loader finds them, and still installs them under
  `.claude/` in consumers. The `sc-lint-version-adoption` skill ships in its
  plugin.
- The adoption kit writes `.sc-lint/install.json` in declaration order
  instead of sorting keys, and rejects `null` helper-policy values instead of
  rendering invalid TOML (#841).
- The adoption kit writes TOML strings, keys and names as Unicode scalar
  text and escapes DEL, so characters outside the Basic Multilingual Plane
  render as valid TOML instead of invalid surrogate-pair escapes
  (lint-085.39).
- Published wheels exclude `**/*.pyc` (#841).
- An `#[sc_lint(allow(...), reason = "...")]` runtime allowance on an item
  that is not a function or method is reported as an error instead of being
  silently ignored. A runtime allowance that names several unknown
  `SCB-RUNTIME-*` ids reports every one, not only the first (lint-085.46).

### Public Rust API

- New in `sc-lint-analyzer-support`: `AnalyzeOptions` (moved from the
  analyzer crates), `repo_relative_source_path`,
  `RepoRelativeSourcePathError`, `relative_path_under_root` and
  `is_test_only`. `sc-lint-portability` and `sc-lint-runtime` re-export
  `AnalyzeOptions`, so `sc_lint_portability::AnalyzeOptions` and
  `sc_lint_runtime::AnalyzeOptions` keep their paths and fields;
  `sc-lint-boundary` keeps its own `AnalyzeOptions` with the `rule` filter.
- New in `sc-lint-directives`: `is_sc_lint_attribute`, `unknown_owned_rules`,
  `unknown_rule`, the `#[non_exhaustive]` `DirectiveCode` (including
  `SyntaxError`), `DirectiveError::typed`, `DirectiveError::typed_code`,
  `DirectiveError::from_diagnostic` (maps a foreign `syn` error to
  `DIRECTIVE.SITE_ALLOW.SYNTAX_ERROR`), and `AttributeInput::parse_tokens` and
  `AttributeInput::parse_attribute`. Existing `DirectiveError` parse variants
  and constructors remain (lint-085.46, lint-085.52).
- `sc-lint-portability`: `PortabilityError::UnknownRule` and
  `PortabilityError::InvalidSiteAllow` are removed. Every site-allowance
  failure, one or many, is `PortabilityError::InvalidSiteAllows(SiteAllowErrors)`.
  New exported types: `SiteAllowDiagnostic { path, line, error }` and
  `SiteAllowErrors { diagnostics }` (lint-085.52, lint-085.57).
- `sc-lint-analyzer-support`: `PackageDependency.name` is the validated
  `PackageName` instead of `String` (`PackageTarget.name` is already
  `TargetName`). `PackageName` and `TargetName` keep `new`, `as_str`,
  `AsRef<str>` and `Display` but no longer implement `Deref`; `new` panics on
  empty input in release builds as well as debug builds, as documented
  (lint-085.46). The unused public
  `count_scanned_crates(root)` is removed;
  `count_scanned_crates_with_metadata` remains (lint-085.39).
- Rust library consumers that build `AnalyzeOptions` by struct literal or
  import it from an analyzer crate need no change. Consumers that read
  `PackageDependency.name` as `String`, rely on `Deref` from `PackageName` or
  `TargetName`, call `count_scanned_crates`, or match
  `PortabilityError::UnknownRule` or `PortabilityError::InvalidSiteAllow`
  must update; use `as_str()` and match `InvalidSiteAllows`. CLI and Python users need no configuration migration.

### Fixed

- Analyzer output draining is deterministic: a lingering grandchild process
  can no longer cause a lint step to miss output (#839, #843).
- Consumer adoption defects found in sc-compose (#841, #844):
  - Published Python wheels exclude repository-only contract, lifecycle,
    orchestration and release tests; release verification rejects leaked
    tests.
  - The reusable GitHub Action selects Linux and Windows ARM64 releases as
    well as the existing x86_64 and macOS targets.
  - Docs state that Python helper policies retain per-table fallback to
    `.just/lint-config.toml`; the native CLI and analyzers use
    `sc-lint.toml`.
- The adoption kit renders consumer helper-policy tables, logging/artifact
  settings and `default_lint_profile` from its managed input without drift
  (#841, #834).
- Bootstrap lint, test and exec commands put the managed venv scripts on
  child PATH after the archive bin directory and ahead of host interpreters
  (#841).
- The Python bootstrap's default managed binary directory matches the native
  installer on Windows: it follows `%APPDATA%` (Roaming), not `%LOCALAPPDATA%`,
  so a fresh install without `SC_LINT_INSTALL_DIR` finds the released binary
  (#833). The bootstrap reads the home directory only when neither
  `SC_LINT_INSTALL_DIR` nor `APPDATA` (Windows) or `XDG_DATA_HOME` supplies
  the directory, so a host with no home directory and an explicit override no
  longer fails with "Could not determine home directory" (lint-085.50). When
  no override, `APPDATA`, `XDG_DATA_HOME` or home directory is available, the
  bootstrap prints one `CLI.SC_LINT_INSTALL_PERMISSION_DENIED` line that
  suggests setting `SC_LINT_INSTALL_DIR` and exits 4, instead of a Python
  traceback (lint-085.58).
- `sc_lint.lint_common.rust_file_test_scope` classifies compound test cfgs such
  as `#[cfg(any(test, feature = "..."))]` and `#[cfg(all(test, ...))]` under
  `src/` as test scope; `cfg(not(test))` stays production (#837).
- Analyzer binaries no longer fail when stdout is closed early, for example
  when piped into `head`.
- `sc-lint init --just` classifies a `sc-lint.toml` generated by 0.8.4 and
  otherwise unedited as version-only stale: it is preserved and reported, not
  overwritten, instead of being reported as a user-owned conflict. A 0.8.4
  config with edited steps still conflicts and stays user-owned. Either line
  ending is accepted (lint-085.47, lint-085.51, lint-085.55).
- `sc-lint init --just` upgrades an unedited 0.8.4 integration without a
  conflict: a 0.8.4 `Justfile` with its old managed header is accepted as
  current, and an unedited 0.8.4 `.sc-lint/bootstrap.py` is rewritten with
  the current one (`--check` reports it as outdated). An edited `Justfile` or
  `bootstrap.py` stays user-owned and is reported as a conflict. A
  `sc-lint.toml` counts as version-only stale only when the version value is
  a well-formed minimum version, so a key inserted after it makes the file a
  user-owned conflict instead (lint-085.56).
- The Python bindings guide now says bootstrap `setup --check` is a usage
  error (exit 2), `setup --dry-run` is supported, and `upgrade --check` is the
  non-mutating probe (lint-085.47).

## [0.8.4] — 2026-11-01

### Added

- Lint rows accept `host = "linux" | "macos" | "windows"` to select the
  execution host. `[tool.sc-lint] default_lint_profile` selects the named
  profile for bare consumer lint. Lint steps and command test selectors accept
  `env_paths`, resolving relative values from the config file's directory
  (#822, #784).
- Release-time native verification runs the published archive binaries and
  pip-installed wheel on each manifest-declared host, checking SHA256 digests,
  architecture and versions. It inspects completed release runs and can also
  verify an explicitly selected tag (#820).

### Changed

- Upgrade Rust dependencies, including syn 3, toml 1.x, cargo_metadata 0.23,
  PyO3 0.30 and jsonschema 0.60. The dependency license allowlist now includes
  MIT-0 (#191, #818).
- Rust library consumers that exchange syn syntax nodes or toml tables with
  sc-lint helpers must align their direct dependencies with syn 3 and toml
  1.x. Consumers with a dependency license allowlist may need to admit MIT-0.
  CLI and Python users need no configuration migration for these upgrades.
- Configs using `host`, `default_lint_profile`, or `env_paths` require sc-lint
  0.8.4 or newer. Existing configs keep their current behavior.

### Fixed

- Windows release binaries statically link the MSVC runtime and no longer
  require an installed Visual C++ runtime.
- Future stable releases and prereleases upload only manifest-declared
  payloads and their checksums from dedicated staging directories. Checked-in
  release notes, templates, manifests and pin files are no longer uploaded as
  release assets; published immutable 0.8.3 assets are unchanged (#821).
- The adoption kit's `profiles.json` schema accepts the Rust consumer
  lint-step fields, including `host`, `env_paths`, `env`, `cwd`, and
  `after_all`, and validates test-layer entries against the supported
  test-step shape (#822).
  Environment-variable names must be non-empty and contain neither `=` nor NUL.

## [0.8.3] — 2026-11-01

Phase M ships the atm-core P1 tool fixes (#786, #720, #136, #385, #198,
#195; phase PR #815), and release artifacts now cover six platforms (#777,
#778; PR #780).

### Added

- Release archives and Python wheels cover Linux, macOS and Windows on both
  x86_64 and ARM64, with ARM64 entries in the Homebrew, winget and Scoop
  manifests. Managed setup, upgrade and the bootstrap select the new Linux and
  Windows ARM64 archives (#777, #778).
- Prereleases include Python wheels for every declared wheel target, including
  the three platforms previously published only in stable releases.
- PORT-010 recognizes same-name `cfg(not(unix))` siblings and Unix-only
  out-of-line modules, including their public items (#720, #786).
- PORT-010 can be allowed for one item with a required reason; suppressed
  findings appear in schema `0.2.0` reports as `allowed` entries, filtered by
  the same rule filter and exclusions as findings (#786).
- SCB-MANIFEST-004 resolves local module paths (#136), and SCB-CYCLE-001
  findings honor `exclude_paths_by_rule` (#385).
- Lint steps accept `env` and `cwd` (#198). Failed-step output includes both
  stdout and stderr, and when a log root is configured every executed step,
  passing steps included, writes both streams to its own file under
  `<log-root>/steps/`, named from the step name (#195).

### Changed

- `error.details.failed_steps[]` entries carry optional
  `step_log_destination` and `step_log_error` when that step's configured log
  could not be written. A log-write failure makes the step failed whether its
  child passed or failed; it preserves the child's exit code (null only if
  there was none) and keeps stdout and stderr in the report (#195).

### Breaking Changes

- The public Rust report API changes: `analyzer_report_schema()` now returns
  schema v2, and `analyzer_report_v1_schema()` exposes v1. Site allowances add
  public directive and error variants and an optional `FindingsReport.allowed`
  field. Downstream exhaustive enum matches and `FindingsReport` struct
  literals may require source changes; see the
  [ADR-011 compatibility assessment](docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md#compatibility-assessment--port-010-site-allowances-2026-10-31).

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

### Consumer Action Required

- The `line-counts` and `identity-literals` checks fail with
  `config.table_missing` when no supported config file defines their
  `line_counts` or `identities` table; add it to `sc-lint.toml` or
  `.just/lint-config.toml`.
- Repositories without a `[version_sync]` table fail version sync with
  `config.table_missing`; add the table and configure the checks you need.
- Unknown `[tool.sc-lint]` keys are rejected with `config.unknown_table`.
- External composition roots listed only in `allowed_dependencies` must move
  to `allowed_dependents`.
- Consumer profile failures report `profile.steps_failed` with
  `error.details.failed_steps[]` of `{index, name, exit_code, message}`
  instead of `{index, step, message}`.

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
