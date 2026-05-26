---
id: C.6
title: Production Path-Literal Portability Parity
status: planned
branch: feature/sprint-C6
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/sprint-C6
target: develop
---

# Sprint C.6 — Production Path-Literal Portability Parity

## Goal

- extend shared path-literal portability linting from test-only scope into
  production code
- add Windows-path parity to the existing Unix-path portability family
- keep path-literal detection separate from broader structural `cfg` parity

## Hard Dependencies

- GitHub issue `#53` — production path portability gap
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/phase-C/phase-C-plan.md](./phase-C-plan.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](../sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)
- `RuleId` in `crates/sc-lint-portability/src/lib.rs` must carry
  `#[non_exhaustive]` before this sprint runs; otherwise each `Port00x`
  variant addition becomes a semver-breaking change under ADR-011.

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/source_scan.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/phase-C/phase-C-plan.md`
- `docs/phase-C/sprint-C6.md`

## Deliverables

- `sc-lint-portability` adds one production-scope path-literal rule pair:
  - `PORT-006` hardcoded Unix-only absolute path literals in production code
  - `PORT-007` hardcoded Windows-only absolute path literals in production
    code
  - closes `REQ-PRODUCT-004AA` for production path-literal expansion
  - closes `REQ-PRODUCT-004AB` for Windows-path parity in the shared crate
- `crates/sc-lint-portability/src/lib.rs` extends `RuleId` with `Port006` and
  `Port007`
- `crates/sc-lint-portability/src/portability.rs` extends
  `collect_unix_portability_findings(...)` with production path-literal leaf
  checks, while `PortabilityCollector` remains limited to scanning test-scope
  items and continues to own `PORT-001`, `PORT-002`, and `PORT-003`
- the production path-literal rule seam is explicit and uses the existing
  production walk rather than silently broadening the test-only collector;
  `visit_item_for_unix_portability(...)` is the integration seam, delegates
  into `visit_block_for_unix_portability(...)` and then
  `visit_expr_for_unix_portability(...)` for the inner expression walk, and
  calls the new helper only when `scope == ScopeKind::NonTest` and
  `unix_gated == false`:

```rust
fn collect_production_path_literal_findings(
    item: &Item,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    // Inspect literals reachable from the current production item and emit
    // PORT-006 / PORT-007 findings for hardcoded absolute paths.
}

fn visit_item_for_unix_portability(
    item: &Item,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    // Existing scope + unix_gated propagation stays in place here.
    if scope == ScopeKind::NonTest && !unix_gated {
        collect_production_path_literal_findings(item, file_context, findings);
    }
    visit_expr_for_unix_portability(/* recurse into the item's expressions */);
}
```

- `PORT-006` reuses the existing Unix absolute-path matching approach for
  production literals, while `PORT-007` adds explicit Windows-path detection
  for both:
  - drive-letter absolute paths such as `C:\ProgramData\sc-lint\cache.json`
  - UNC paths such as `\\server\share\sc-lint\cache.json`
- `PORT-007` uses a dedicated built-in helper instead of a repo-configured
  allowlist key:

```rust
fn is_windows_path_literal(value: &str) -> bool {
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    let unc_absolute = value.starts_with("\\\\");
    drive_absolute || unc_absolute
}
```

- explicit `#[cfg(unix)]` production items do not emit `PORT-006`; those
  intentional Unix-only boundaries stay under `C.9` structural parity review
  instead of double-firing leaf and parity findings for the same branch
- rule messages point callers toward platform-aware path sources or explicit
  platform-gated abstractions rather than hardcoded OS-specific literals
- `crates/sc-lint-portability/README.md` documents the new production
  path-literal rules and their intended portable alternatives

## Explicit Code Samples

```rust
pub fn unix_socket_dir() -> std::path::PathBuf {
    std::path::PathBuf::from("/var/run/sc-lint")
}

pub fn windows_cache_file() -> std::path::PathBuf {
    std::path::PathBuf::from(r"C:\ProgramData\sc-lint\cache.json")
}
```

```rust
pub fn cache_dir() -> std::path::PathBuf {
    dirs::cache_dir().expect("cache directory")
}
```

## This Sprint Does Not Close

- broader environment-variable portability rules
- shell invocation portability rules
- generic production `cfg(unix)` / `cfg(windows)` parity checks outside the
  path-literal family

## Acceptance Criteria

- the sprint defines `PORT-006` and `PORT-007` as `sc-lint-portability` owned
  production rules rather than extending `PORT-001` beyond test scope
- the sprint explicitly covers both Unix-only and Windows-only absolute path
  literal patterns in production code
- the sprint names the concrete implementation seam in
  `visit_item_for_unix_portability(...)`, states that
  `collect_production_path_literal_findings(...)` and
  `visit_expr_for_unix_portability(...)` are both called from that seam, and
  keeps `PortabilityCollector` test-scope-only
- the sprint defines `PORT-007` detection for both drive-letter absolute paths
  and UNC paths instead of leaving Windows matching implicit
- the sprint names at least one platform-aware alternative path source for
  remediation guidance
- the sprint keeps structural `cfg(unix)` companion analysis out of scope and
  assigns that closure to `C.9`, including the rule that explicit
  `#[cfg(unix)]` items are not `PORT-006` findings
- the sprint references GitHub issue `#53` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo test -p sc-lint-portability flags_hardcoded_unix_path_in_production_code`
- `cargo test -p sc-lint-portability flags_hardcoded_windows_path_in_production_code`
- `cargo test -p sc-lint-portability passes_dirs_cache_dir_in_production_code`
- `cargo test -p sc-lint-portability passes_cfg_unix_gated_unix_path_in_production_code`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
