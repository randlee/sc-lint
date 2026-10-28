# ADR-020 — Phase L decisions for the atm-core findings

Status: Accepted — Rand, 2026-10-27 (drafted 2026-10-26 on `plans/phase-l`). Plan-review clarifications for sprints l-1 to l-9: Accepted — Rand, 2026-10-27. The Sprint l-10 clarification: Status: Accepted — Rand, 2026-10-27, together with REQ-CLI-025.

Related: ADR-006, ADR-007, ADR-010, ADR-013, ADR-015, ADR-016, ADR-017, REQ-SCB-017,
REQ-PRODUCT-004AA, REQ-PRODUCT-004C, REQ-PRODUCT-004D, REQ-PRODUCT-023, REQ-CLI-005C,
REQ-CLI-023.
Plan authority: `lint-phase-l` and sprints `lint-l-1` to `lint-l-10`.

## Intent

Record, before dispatch, every decision the phase L sprints would otherwise
make mid-sprint. Approval of this record gates dispatch only.

## Decisions

1. **PORT-006 and PORT-008 (narrows ADR-010 Decision 6, #641; this decision
   supersedes the `cfg(unix)`-only wording there).** An item gated by
   `#[cfg(unix)]`, `#[cfg(target_os = "linux"|"macos")]` or `#[cfg(not(windows))]`
   is a structural parity question exactly as `#[cfg(unix)]` is: PORT-006 leaf
   findings are suppressed inside it. PORT-008 does not report a read of
   `USER` when the same enclosing function also reads `USERNAME`. The gate
   scope is limited to these attribute forms: other `target_os` values and
   statement-level `#[cfg]` attributes are not honored.
2. **PORT-007 (#192).** Findings for the exact verbatim and device prefixes
   `\\?\` and `\\.\` are not reported. A reviewed-exception allowlist is not
   added in phase L.
3. **PORT-010 (#372).** A `#[cfg(unix)]` function inside a file declared in a
   `platform_adapters` pair is not reported. Other atm-core PORT-010 shapes
   (crate roots such as `crates/atm/src/main.rs`) are classified, not
   assumed fixed (see `lint-l-1`, L1-4 for the covered shape and L1-5 for fixtures
   of the others).
4. **Dependency policy (#643, #376).** An unexpected runtime dependent is
   reported as `SCB-DEPENDENCY-002` per REQ-SCB-017, using `allowed_dependents`
   unioned across every record of the owner package, dev edges separate. A stale
   `allowed_dependents` entry stays `SCB-DEPENDENCY-004`. Non-active record
   states (planned, retired, reserved_future) keep their current silent skip;
   changing that is outside phase L.
5. **Path-less exclusions (#385).** For findings with no path (SCB-CYCLE-002
   and 003), `exclude_paths_by_rule` resolves the path set from the source
   files of the finding's `node_ids`; the finding is excluded when any such
   file matches an exclusion. The shared `Finding` schema in `sc-lint-schema`
   is unchanged.
6. **Tokio sleep rule (#82, REQ-PRODUCT-004C as amended).** The rule is
   `SCB-RUNTIME-003` in `sc-lint-runtime`, reached by the existing
   `sc-lint lint sc-runtime`, and fails by default like `SCB-RUNTIME-001/002`;
   target kinds come from `cargo metadata`. It needs no `tokio` crate dependency: it reads
   package manifests and walks source. No `sc-lint-tokio` crate is created in
   phase L; the reserved boundary manifest stays reserved for Tokio rules
   that need Tokio dependencies or runtime semantics. A "Tokio package" is a
   workspace package declaring `tokio` in `[dependencies]` or
   `[build-dependencies]` (including target-specific tables and
   `package = "tokio"` renames). A production target is a `lib` or `bin`
   target; `#[cfg(test)]` items, `tests/`, `benches/` and `examples/` are
   excluded. The rule rejects `std::thread::sleep`, `thread::sleep` after
   `use std::thread`, and `sleep` imported from `std::thread` (including `as`
   aliases), and permits `tokio::time::sleep`.
   The rule honors the runtime analyzer's `exclude_paths_by_rule` like every
   other rule (REQ-PRODUCT-023: each exclusion carries a `reason`), so a
   consumer can stage the migration of existing sites. (Requested by atm-core
   review of 2026-10-27: three production sleep sites in tokio packages.)
7. **Consumer test selectors (#196).** REQ-CLI-023's sentence "test argument
   forwarding [is] not added" is superseded for consumer test profiles:
   selectors (one command or a group of selectors), `test.default`, positional
   forwarding after a selector, `env`, `cwd`, `platforms` and `--list`.
   `requires` and installer-side manifest validation are not added in phase L.
8. **Installer templates (#604, ADR-015).** `.j2` is reserved for sc-compose
   templates that carry front matter. Templates rendered by `install.py`'s own
   substitution use `.tmpl`.
9. **#198 (setup hook).** Won't-do. ADR-016's hook restriction is unchanged;
   `setup.pre` and `runner.python` are not added. Consumers use
   `sc_lint.setup_tools`; direct manifest, shear, and spell checks use the
   managed invocation `.sc-lint/venv/bin/python -m sc_lint.<module>`.

10. **Stable codes (l-4, l-6, l-8).** New codes use the existing error envelope
    (`kind`, `code`, `cause`, `recovery`; REQ-CLI-005C) and are recorded in the
    error table of `docs/sc-lint/cli-contract.md` by the sprint that adds them:

    | Code | Kind | Cause names | Recovery text |
    |---|---|---|---|
    | `config.boundaries_dir_missing` | `config` | the repo root searched | create `boundaries/` or do not run the boundary family |
    | `config.table_missing` | `config` | the table and files searched | add the table to `sc-lint.toml` or `.just/lint-config.toml` |
    | `profile.invalid_after_all` | `config` | the step or group selector and file | remove `after_all` from that selector |
    | `test.unknown_selector` | `config` | the selector name and file | use a name from `sc-lint test --list` |
    | `test.group_cycle` | `config` | the selector cycle path | break the cycle in `[tool.sc-lint.test]` |
    | `profile.steps_failed` | `backend_failure` | `failed_steps[]`: `index`, `name`, `exit_code` in declaration order | fix the failing steps and rerun |
    | `backend_payload_drift` | `backend_protocol` | the failing JSON key path | upgrade or reinstall the wheel so the payload matches the envelope schema |

    Codes are typed constants in the owning crate and are validated where
    config is parsed, never matched as free strings at call sites.

11. **Delegation edges (#204, supersedes part of the 2026-10-10 #204 amendment).**
    A method-to-method expression edge between two distinct graph nodes is a
    delegation edge, and `SCB-CYCLE-002/003` exclude it, when the callee cannot
    reach the caller through the owner's expression edges. The earlier name and
    visibility conditions are dropped: a trait-impl method calling a same-named
    inherent method is a call between two distinct nodes (Rust resolves
    `self.m()` to the inherent method), and a public method calling a different
    public method is not recursion. A method that resolves to its own node
    stays a self-loop. Method-call expressions create edges only when the
    receiver is exactly `self` (through parentheses, `&` and `*`); a receiver
    that is a field, local or call result creates none.
12. **`[version_sync.python]` (#137, REQ-CLI-003).** Keys: `cargo_lock` (bool,
    default `true`; `false` omits the Cargo.lock check) and `python_manifests`
    (array of repo-relative `pyproject.toml` paths, default empty; each static
    `[project].version` must equal the workspace version). The
    `[version_sync.winget]` and `[version_sync.release_wiring]` subsections keep
    the keys already implemented (`manifest_glob`, `package_version_field`,
    `manifest_version_field`, `installer_url_field`; `file`,
    `required_fragments`), which match atm-core's `.just/lint-config.toml`.
    A missing `Cargo.lock` is reported as skipped, not as a failure. These key
    names are chosen here because no earlier record defines them. sc-lint owns
    the workspace, crate, lockfile, and Python package-version checks; it does
    not check sibling dependency version pins in `pyproject.toml`. atm-core
    keeps that dependency-pin check in its own tooling.

## Plan-review clarifications (Accepted — Rand, 2026-10-27)

These clarifications were written by the sprint design fixes for plan review round 2, after the decisions above were approved on 2026-10-27. They narrow or make precise the approved decisions and change none of them, except where a clarification says otherwise. Sprints l-1 to l-9: Accepted — Rand, 2026-10-27 ("approve the adr"). Sprint l-10: Status: Accepted — Rand, 2026-10-27, together with REQ-CLI-025. Numbering is per sprint.

### Sprint l-1
Clarifications to decisions 1 to 3 (recorded by `lint-l-1`):

1. Decision 1 widens only the leaf-suppression predicate `attr_is_platform_cfg`. Because PORT-006/007/008/009 share that gate, the added cfgs (`target_os = "linux"|"macos"`, `not(windows)`, and `all`/`any` lists containing one of them) silence PORT-007, PORT-008 and PORT-009 exactly as `cfg(unix)` does. PORT-004 and PORT-010 do not treat the added cfgs as unix-gated, and PORT-010 impl-method parity still runs for impls under them. `target_os = "unix"` stays accepted as before. `all`/`any` use the existing any-child rule. The only PORT-010 difference is that an impl with `#[cfg(unix)]` methods inside a `#[cfg(windows)]` ancestor module is now checked.
2. Decision 1's USERNAME rule: "same enclosing function" is the function or impl method whose body textually contains an `env::var`/`env::var_os` read of the literal `USERNAME` (closures and nested items inside that body included, case-sensitive). `const` and `static` initialisers are never covered.
3. Decision 2: the exemption is `starts_with` of the post-unescape literal value against `\\?\` or `\\.\`; `\\server\share` and `\\?x` are still reported.
4. Decision 3: the exemption covers private (inherited visibility) fn and mod items and private inherent-impl methods in the files of both modules of a `platform_adapters` pair, at any inline-module depth. `pub`, `pub(crate)`, `pub(super)`, `pub(in ..)` items and trait-impl methods keep the parity check; impls nested in fn bodies are not exempt. The crate-root `main.rs` shape (`#[cfg(unix)]` fn with a `#[cfg(not(unix))]` sibling) is classified by fixture as still reported, not fixed.


### Sprint l-2
Clarifications to decision 4 (sprint l-2):
4.1 The external composition-root check of the inventory loader (SCB-BOUNDARY-004, kind `unapproved_composition_root`) tests the record's `dependencies.allowed_dependents`, not `allowed_dependencies`, as REQ-SCB-020's "dependent-policy checks" requires. It replaces the old test (no union). This is a behaviour change for records that listed an external root's package only under `allowed_dependencies`: they must move it to `allowed_dependents`. sc-lint's own `boundaries/` records are unaffected. REQ-SCB-020 is amended with one sentence to say so.
4.2 The #376/#643 aggregation (union of `allowed_dependents` across the records of an owner, dev fallback per record, normal dominating dev, one 002/004 per owner/dependent/category) is already implemented in `package_policy.rs`; phase L adds tests only. The `SCB-DEPENDENCY-002` message and finding shape are unchanged; the 002 message names the first record of the owner.
4.3 Atm-core parity for the manifest allowlist (#136) is limited to the record-lock comparison of the record keys `allowed_dependencies` and `allowed_dev_dependencies` against atm-core `collect_manifest_dependency_allowlist_violations` at randlee/atm-core commit f4746d500. Parity is by `(RuleId, kind)` and finding count, not message text. The four-case oracle is recorded in `crates/sc-lint-boundary/tests/issue136.rs`; the cases agree with the current implementation.


### Sprint l-3
Clarifications to ADR-020 from sprint l-3:
1. Decision 5: global `exclude_paths` also apply to path-less SCB-CYCLE-002/003 findings through the same any-of-N match as `exclude_paths_by_rule` (a finding is excluded when any source file of its `node_ids` matches either list).
2. Decision 5: a node whose file cannot be made repo-relative (not under the root, or absent from the node-to-file map) contributes no path; a finding with no resolvable path is never excluded by path.
3. Decision 11: method-call receivers other than `self` (typed locals, fields, call results) and calls inside macro invocations create no edge; trait default bodies (no owner type) create none either. A trait-impl method resolving to its own node stays an SCB-CYCLE-003 self-loop.


### Sprint l-4
Clarifications to ADR-020 from l-4:
13. Repo root (decision 7 area, REQ-CLI-003): the nearest ancestor of the discovery start holding `sc-lint.toml` or a `Cargo.toml` with a `[workspace]` table is the repo root; nearest wins regardless of marker kind; package-only manifests never match. Only `lint sc-boundary` requires `boundaries/`.
14. Per-table resolution: candidates are the explicit `--config` file, `sc-lint.toml`, `.just/lint-config.toml`, in that order; the first existing file holding the table wins, and a file lacking the table never stops the search. An explicit `--config` file that does not exist is a hard error (`explicit config file not found: <path>`, kind config, no code), not a skipped candidate. A key present with a non-table value, or an unparsable candidate reached during the search, is a hard error of kind config. Rust forwards `--config` only when the user passed it.
15. Behaviour change: `line-counts` and `identity-literals` now fail with `config.table_missing` when their table (`line_counts`, `identities`) is in none of the files; previously a missing table meant defaults. This repository's `.just/lint-config.toml` gains empty tables to keep its behaviour. `lint_common.load_lint_config` stays as a permanently-deprecated shim.
16. `config.table_missing` crosses the Rust adapter through a closed lookup (`ConfigCode::from_wire`); an unrecognised `details.code` stays in `details` and the error keeps `CLI.CONFIG_ERROR`. The cause equals the Python message (table and files searched, repo-relative posix paths, absolute when outside the repo). `error.suggested_action` carries the decision 10 recovery text.


### Sprint l-5
Clarifications to decision 8 (l-5):
1. Upgrade of installed consumers: renaming the three templates changes the files the kit copies to `plugins/sc-lint/templates/`. The existing retirement logic removes the old `.j2` copies when unmodified (digest match) and reports `conflict: ... was modified outside sc-lint` (exit 2, no writes) when a consumer edited one; `--dry-run` lists them as deletions and exits 1. No installer code change; tests cover the three cases.
2. `tests/adoption/golden/` (rendered Justfile, sc-lint.toml, sc-lint.yml) is a permanent byte-exact regression baseline, stored with `-text` in .gitattributes and compared as bytes.
3. The `.j2`/`.tmpl` guard covers git-tracked files under packages/sc-lint-adoption and skips only when git is absent or ROOT is not the top level of a git work tree.

### Sprint l-6
Clarifications from l-6 (no new codes beyond decision 10):
1. Decision 7, coexistence. `[tool.sc-lint.test]` is exactly one of two forms per file (TOML forbids both): the existing `[[tool.sc-lint.test]]` row array (unchanged, each row is a command selector named after the row, bare `sc-lint test` and `all` run every row) or a table with `default` and `selectors.<name>` (a `command` argv or a `group` of selector names, with `env`, `cwd`, `platforms`, `after_all`). In the table form there is no implicit `all`; a bare `sc-lint test` runs `default`, and without `default` it fails `test.unknown_selector`. Table-form `--list` and iteration are sorted by selector name; group members keep their listed order. A command reached twice through groups runs once. Arguments after the selector are appended verbatim to a command selector's argv; arguments to a group selector are a usage error. A selector whose `platforms` excludes the host is reported as a skipped row (`reason: "platform_mismatch"`), not run and not a failure. `requires` remains out of scope.
2. Decision 10, `profile.invalid_after_all` also covers a non-boolean `after_all` value, and applies to lint and test steps and selectors. `profile.steps_failed` is `kind: backend_failure` (exit 5) and carries `error.details.failed_steps[]` with `index` (zero-based position in the selected list, platform-skipped rows included), `name`, `exit_code` (number, null when the process did not start or panicked) and `message` (the step error message plus its cause). Under `--fail-fast` it also carries `error.details.skipped_steps[]` of `{index, name, reason: "fail-fast"}`; this is an additional detail key, not a new code. The built-in lint plan uses the same code and shape, replacing its previous `{index, step, message}` payload.
3. REQ-CLI-023 as amended, executor behaviour: a step is in the serialized cargo lane iff the file stem of its argv[0] is `cargo` (case-insensitive); every other step has its own worker and there is no worker cap; a panicking step is a failed step (never aborts the run); `--fail-fast` skips only not-yet-started cargo-lane steps and `after_all` steps, never cancels or skips running or Parallel steps. A missing `cargo xwin` is preflighted before any step runs and stays a capability error. A consumer failure reports `profile.steps_failed` instead of the first step's own error code (a missing backend no longer surfaces `CLI.SC_LINT_BACKEND_NOT_FOUND`; the spawn error is in the step `message`).
4. The `Stable codes` table in docs/sc-lint/cli-contract.md receives the four rows `profile.invalid_after_all`, `test.unknown_selector`, `test.group_cycle`, `profile.steps_failed` from l-6 with the decision 10 recovery text verbatim.


### Sprint l-7
ADR-020 decision 6 clarifications (sprint l-7):
1. Narrowing of ADR-007: the ADR-007 follow-up "keep Tokio-specific rules out of `sc-lint-runtime` unless a later ADR changes that split" is narrowed by decision 6 to: a Tokio-aware rule that needs no Tokio crate dependency and no Tokio runtime semantics (SCB-RUNTIME-003) lives in `sc-lint-runtime`; `sc-lint-tokio` remains reserved for rules that need Tokio dependencies or runtime semantics. The sprint l-7 runtime docs record this.
2. Rule-neutral support crate (REQ-PRODUCT-004D, ADR-013): `sc-lint-analyzer-support` adds `WorkspaceMetadata::workspace_packages()` exposing dependencies (real name, rename, kind, target platform) and targets (name, kind, src_path). "Declares tokio" is evaluated in `sc-lint-runtime` only (dependency `name == "tokio"` with kind Normal or Build, so Development, `tokio-util` and `tokio-stream` never trigger).
3. Target kind mapping from cargo's `target.kind` array, by precedence: bin, example, test, bench, custom-build, then any library kind (lib, rlib, dylib, cdylib, staticlib, proc-macro) = Lib, else Other. Production = Lib or Bin. Because file discovery attributes each `src/` file to the first target, the rule does not use the attributed target: a file is checked when its package has a Lib or Bin target and the file is not the exact `src_path` of a non-production target.
4. Test scope for rule 003 is decided by attributes only: `#[cfg(test)]`, `#[cfg(all(.., test, ..))]` (any `all` member requiring test), `#[test]` and `#[tokio::test]`-style `<crate>::test` attributes are exempt; `cfg(any(test, ..))` and `cfg(not(test))` are checked; a module merely named `tests` is checked.
5. Import resolution is flattened per file and only the `std` root is recognised; macros are not expanded; out-of-line `#[cfg(test)] mod x;` files are scanned as production (same as SCB-RUNTIME-001/002).
6. `exclude_paths_by_rule` in the runtime analyzer is the existing plain-array form plus the optional single table-level `reason`; decision 6's "each exclusion carries a `reason`" is not enforced by the parser and L7 adds no enforcement. The reported message path is absolute; exclusion patterns match the repo-relative `/`-separated path.

### Sprint l-8
Clarifications for decision 10 (`backend_payload_drift`) and REQ-PRODUCT-002DA (l-8):
1. "Published schema" in REQ-PRODUCT-002DA means the `sc-lint-python-v1` payload shape documented in cli-contract.md ("Python adapter backend") and encoded in `sc_lint_schema::validate_adapter_payload`. No JSON Schema file or other artifact is published. Rust backends are typed structs and are not runtime-validated.
2. The validator lives in `sc-lint-schema` (which gains `serde_json` as a normal dependency, already a workspace dependency; Cargo.lock unchanged). It requires `adapter_schema`, `ok`, `diagnostics` (array of strings); on success `data` (object) and `summary` (string) and no `error`; on failure `error{kind,message}` (strings), optional `error.details` (object) and `error.suggested_action` (string), optional `summary`, and no `data`. Unknown extra keys are allowed. The first drift in that fixed order is reported.
3. In `parse_adapter_output` the check runs after the existing unknown-`adapter_schema` check (not drift) and before the exit-code and verdict checks, so a drifted payload fails as `backend_payload_drift` even when the process exit code is non-zero.
4. Column mapping for the decision 10 row: Code -> `CliError::code()` (`backend_payload_drift`, typed constant in `python_adapter.rs`); Kind -> `backend_protocol`; Cause names -> `CliError::cause` (key path text) plus `details.key_path`; Recovery -> `CliError::suggested_action`.
5. `sc-lint-schema` may hold pure shape validation of the transport payloads it defines; it still performs no analysis and decides no policy (boundary record, architecture.md and requirements.md updated accordingly).

### Sprint l-9
Clarifications to ADR-020 decision 12 (l-9):
1. The stable finding `check` ids of `sc_lint.check_version_sync` are: `crate_version`, `internal_path_dep`, `cargo_lock`, `python.version`, `python.manifest_missing`, `winget.manifest_glob`, `winget.package_version`, `winget.manifest_version`, `winget.installer_url`, `winget.missing_field`, `release_wiring.file_missing`, `release_wiring.fragment`. A finding has exactly `check`, `file`, `expected`, `actual`.
2. `--json` uses the Python adapter envelope; `data` is `{ok, findings[], skipped[], workspace_version}`. With findings the envelope `ok` stays true, `data.ok` is false and the exit code is 1. Exit codes: 0 clean, 1 findings or unexpected error, 3 config error.
3. Config failures of the check other than a missing table (unreadable or unparsable Cargo.toml, Cargo.lock or listed pyproject, missing workspace version, no member manifests, non-array `python_manifests`, non-bool `cargo_lock`, non-table subtables, absolute or `..` paths in `python_manifests`, `manifest_glob` or `file`) are errors of kind `config` without a `code`; they are not stable codes and decision 10's table is unchanged. Only a missing `[version_sync]` table carries `config.table_missing`.
4. A listed pyproject whose `[project].version` is `dynamic` yields a `python.version` finding with actual `dynamic`; absent yields `absent`; a non-string yields `invalid`. A listed but missing pyproject yields `python.manifest_missing`. A missing `Cargo.lock` is reported in `data.skipped[]` as `{check: "cargo_lock", file: "Cargo.lock", reason: "missing"}` and `cargo_lock = false` omits both the check and the entry.
5. Because `[version_sync]` is now resolved per table, a repo without that table (an empty `[version_sync]` suffices) fails the check, including the built-in `version` plan step, with `config.table_missing`. sc-lint's own `.just/lint-config.toml` adds `[version_sync.python]` with `cargo_lock = true`.
6. `lint_common.load_lint_config` stays a permanently-deprecated shim; l-9 does not edit lint_common.py.

### Sprint l-10 (#647, added at the user's direction after the sprint designs above; governed by REQ-CLI-025)
1. The cargo-shear wrapper treats a path reported in `unlinked_files` as consumed, and not a finding, when a `.pass("<glob>")` or `.compile_fail("<glob>")` call with a string-literal glob in a Rust file of the same package resolves to a regular file at that path, relative to the package directory. No configuration key is added. Consumed takes precedence over `allowed_unlinked_files` and produces no downgrade line. Ordinary unlinked-source detection is unchanged.
2. Only plain and raw string literals are interpreted. Non-literal, empty, absolute, drive-letter, backslash and `..` globs are skipped, and a glob Python cannot evaluate is skipped without error; those fixtures stay reported. Matching is textual, so a comment or string that names a harness call can hide an unlinked report only for a path cargo-shear already reported and that the glob resolves to (accepted over-approximation, documented).
3. Harness files are the `*.rs` files of the same package, scanned in sorted package-relative path order, skipping any `target` directory and the reported unlinked files themselves (a fixture cannot consume itself). The first harness by sorted path then line wins and is named in the note `shear note: <crate_dir>/<path> consumed by trybuild harness <harness>:<line> [unlinked_files]`; the root package omits the `<crate_dir>/` prefix. A package unknown to the workspace metadata consumes nothing.
