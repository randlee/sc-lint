---
id: C.7
title: Broad Environment-Variable Portability
status: completed
branch: feature/sprint-C7
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/sprint-C7
target: develop
---

# Sprint C.7 — Broad Environment-Variable Portability

## Goal

- add one shared production-scope environment-variable portability family to
  `sc-lint-portability`
- catch Unix-centric home/user/config lookups that bypass platform-neutral
  abstractions
- keep the new production env family distinct from the existing home-dir
  wrapper and test-only `set_var` rules: `PORT-008` is production-scope,
  `PORT-002` remains test-scope only, and `PORT-003` remains test-scope only

## Hard Dependencies

- GitHub issue `#54` — broad env-var portability gap
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](../sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- [docs/phase-C/sprint-C6.md](./sprint-C6.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/phase-C/phase-C-plan.md`
- `docs/phase-C/sprint-C7.md`

## Deliverables

- `sc-lint-portability` adds `PORT-008` for production use of Unix-centric
  environment variables without a platform-neutral abstraction:
  - `HOME`
  - `USER`
  - `XDG_*`
  - continues `REQ-PRODUCT-004AA` through the production env-portability
    follow-on owned by `sc-lint-portability`
- `crates/sc-lint-portability/src/lib.rs` extends `RuleId` with `Port008`
- `PORT-008` fires on direct ungated production lookups of `std::env::var`
  or `std::env::var_os` for `HOME`, `USER`, and `XDG_*`; it does not add a
  new repo-configured allowlist or "approved abstraction" config key
- the production env-portability seam is explicit and leaves
  `#[cfg(unix)]`-gated items to `C.9` structural parity instead of adding a
  separate suppression model; `visit_expr_for_unix_portability(expr,
  inherited_scope: ScopeKind, inherited_unix_gated: bool, file_context,
  findings)` is the expression visitor seam, and the actual `PORT-008`
  dispatch happens in `ProductionPathLiteralVisitor::visit_expr_call(...)`
  through `production_env_portability_variable(...)`. This builds directly on
  the `visit_item_for_unix_portability(...) ->
  visit_expr_for_unix_portability(...)` call chain established in `C.6`:

```rust
fn production_env_portability_variable(expr_call: &ExprCall) -> Option<String> {
    let variable_name = extract_env_var_lookup(expr_call)?;
    if variable_name == "HOME"
        || variable_name == "USER"
        || variable_name.starts_with("XDG_")
    {
        return Some(variable_name);
    }
    None
}

fn visit_expr_for_unix_portability(
    expr: &Expr,
    inherited_scope: ScopeKind,
    inherited_unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    let mut visitor = ProductionPathLiteralVisitor::new(file_context, findings);
    visitor.visit_expr(expr);
}

impl<'ast> Visit<'ast> for ProductionPathLiteralVisitor<'_, '_> {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Some(variable_name) = production_env_portability_variable(node) {
            // emit PORT-008 for HOME / USER / XDG_* here
        }
        syn::visit::visit_expr_call(self, node);
    }
}
```

- the rule text distinguishes this family from:
  - `PORT-002` configured home-dir wrapper enforcement
  - `PORT-003` test-only `std::env::set_var()` mutation
- `crates/sc-lint-portability/README.md` documents the new production
  env-portability rule and the intended abstraction strategy

## This Sprint Establishes

- `visit_expr_for_unix_portability(expr, inherited_scope: ScopeKind,
  inherited_unix_gated: bool, file_context, findings)`, as extended by `C.7`,
  continues to be the expression-walk seam inherited from `C.6`
- `ProductionPathLiteralVisitor::visit_expr_call(node: &ExprCall)`, as
  extended by `C.7`, serves as the concrete `Expr::Call` dispatch seam and the
  integration base for sprint `C.8`
- sprint `C.8` extends this same `ProductionPathLiteralVisitor::visit_expr_call`
  seam for shell-invocation portability and therefore depends on this sprint
  completing first

## Explicit Code Samples

```rust
pub fn config_root() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    std::path::PathBuf::from(home).join(".config").join("sc-lint")
}
```

```rust
pub fn data_root() -> std::path::PathBuf {
    dirs::data_dir().expect("data directory")
}
```

## This Sprint Does Not Close

- production path-literal portability rules
- shell invocation portability rules
- generic `cfg(unix)` / `cfg(windows)` structural parity checks

## Acceptance Criteria

- the sprint defines `PORT-008` as a production env-portability rule in
  `sc-lint-portability`
- the sprint explicitly names `HOME`, `USER`, and `XDG_*` as the initial
  covered variable family
- the sprint states the suppression contract explicitly: `PORT-008` is not
  config-driven, fires on ungated production lookups, and leaves
  `#[cfg(unix)]`-gated items to `C.9`
- the sprint names `visit_expr_for_unix_portability(...)` as the expression
  visitor seam for `production_env_portability_variable(...)`
- the sprint states `C.6` as an ordering dependency because `C.7` extends the
  same `visit_item_for_unix_portability(...) -> visit_expr_for_unix_portability(...)`
  production call chain
- the sprint keeps `PORT-002` and `PORT-003` semantics distinct instead of
  silently broadening either existing rule
- the sprint names a platform-neutral remediation path for production callers
- the sprint references GitHub issue `#54` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo test -p sc-lint-portability flags_home_env_lookup_in_production_code`
- `cargo test -p sc-lint-portability flags_user_env_lookup_in_production_code`
- `cargo test -p sc-lint-portability flags_xdg_config_home_lookup_in_production_code`
- `cargo test -p sc-lint-portability passes_dirs_data_dir_in_production_code`
- `cargo test -p sc-lint-portability passes_cfg_unix_gated_home_lookup_in_production_code`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
