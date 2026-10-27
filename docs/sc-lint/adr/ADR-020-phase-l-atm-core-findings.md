# ADR-020 — Phase L decisions for the atm-core findings

Status: Pending — Rand approval required (drafted 2026-10-26 on `plans/phase-l`).

Related: ADR-010, ADR-013, ADR-015, ADR-016, ADR-017, REQ-SCB-017, REQ-PRODUCT-004C,
REQ-PRODUCT-004D, REQ-CLI-023.
Plan authority: `lint-phase-l` and sprints `lint-l-1` to `lint-l-9`.

## Intent

Record, before dispatch, every decision the phase L sprints would otherwise
make mid-sprint. Approval of this record gates dispatch only.

## Decisions

1. **PORT-006 and PORT-008 (ADR-010 Decision 6, #641).** An item gated by
   `#[cfg(unix)]`, `#[cfg(target_os = "linux"|"macos")]` or `#[cfg(not(windows))]`
   is a structural parity question exactly as `#[cfg(unix)]` is: PORT-006 leaf
   findings are suppressed inside it. PORT-008 does not report a read of
   `USER` when the same enclosing function also reads `USERNAME`.
2. **PORT-007 (#192).** Findings for the exact verbatim and device prefixes
   `\\?\` and `\\.\` are not reported. A reviewed-exception allowlist is not
   added in phase L.
3. **PORT-010 (#372).** A `#[cfg(unix)]` function inside a file declared in a
   `platform_adapters` pair is not reported. Other atm-core PORT-010 shapes
   (crate roots such as `crates/atm/src/main.rs`) are classified, not
   assumed fixed (see `lint-l-1`, L1-4).
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
   `sc-lint lint sc-runtime`. It needs no `tokio` crate dependency: it reads
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
7. **Consumer test selectors (#196).** REQ-CLI-023's sentence "test argument
   forwarding [is] not added" is superseded for consumer test profiles:
   selectors (one command or a group of selectors), `test.default`, positional
   forwarding after a selector, `env`, `cwd`, `platforms` and `--list`.
   `requires` and installer-side manifest validation are not added in phase L.
8. **Installer templates (#604, ADR-015).** `.j2` is reserved for sc-compose
   templates that carry front matter. Templates rendered by `install.py`'s own
   substitution use `.tmpl`.
9. **#198 (setup hook).** Won't-do. ADR-016's hook restriction is unchanged;
   consumers use `sc_lint.setup_tools`.
