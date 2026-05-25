---
id: C.8
title: Shell Invocation Portability
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.8 — Shell Invocation Portability

## Goal

- add one shared shell-invocation portability family to
  `sc-lint-portability`
- catch production command-launch patterns that assume a Unix shell exists
- keep shell portability separate from repo-local shell conventions and
  broader `cfg` parity analysis

## Hard Dependencies

- GitHub issue `#55` — shell invocation portability gap
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./adr/ADR-010-portability-scope-and-parity.md)
- [docs/sc-lint/sprint-C7.md](./sprint-C7.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/sprint-C8.md`

## Deliverables

- `sc-lint-portability` adds `PORT-009` for production shell-invocation
  portability drift, covering:
  - `Command::new("sh")`
  - `Command::new("bash")`
  - hardcoded `/bin/sh`
  - hardcoded `/bin/bash`
  - continues `REQ-PRODUCT-004AA` through the shared shell-portability
    follow-on owned by `sc-lint-portability`
- `crates/sc-lint-portability/src/lib.rs` extends `RuleId` with `Port009`
- `PORT-009` integrates into `collect_unix_portability_findings(...)` so it
  reuses the existing `unix_gated` propagation path instead of inventing a
  parallel suppression model; `collect_unix_portability_findings(...)`
  orchestrates the production walk and routes expression analysis through
  `visit_expr_for_unix_portability(...)`
- the shell-invocation detection seam is explicit and separates process-launch
  calls from string-literal path checks; `visit_expr_for_unix_portability(...)`
  must gain an `Expr::Call` arm that invokes `is_shell_command_call(...)`:

```rust
fn is_shell_command_call(expr_call: &ExprCall) -> bool {
    is_command_new_call(expr_call, "sh") || is_command_new_call(expr_call, "bash")
}

fn is_unix_shell_path_literal(value: &str) -> bool {
    matches!(value, "/bin/sh" | "/bin/bash")
}

fn visit_expr_for_unix_portability(
    expr: &Expr,
    unix_gated: bool,
    file_context: &FileContext,
    findings: &mut Vec<PortabilityFinding>,
) {
    match expr {
        Expr::Call(expr_call) if !unix_gated && is_shell_command_call(expr_call) => {
            // emit PORT-009
        }
        Expr::Lit(expr_lit) if !unix_gated && is_unix_shell_path_literal(/* literal text */) => {
            // emit PORT-009 for /bin/sh or /bin/bash literal paths
        }
        Expr::Block(expr_block) => { /* existing recursion */ }
        Expr::If(expr_if) => { /* existing recursion */ }
        Expr::Match(expr_match) => { /* existing recursion */ }
        _ => {}
    }
}
```

- `/bin/sh` and `/bin/bash` fire through the dedicated `PORT-009` literal
  helper above rather than through the generic Unix path-prefix matcher used
  by `PORT-006`; both detection modes share the same production visitor walk
- the shell-portability rule allows explicitly Unix-gated code paths to remain
  Unix-only rather than forcing fake parity into consumers that already expose
  an honest Unix boundary
- the new rule family stays consumer-neutral and does not encode repo-local
  shell conventions or project-specific command wrappers
- `crates/sc-lint-portability/README.md` documents the new shell-portability
  rule family and its expected remediation patterns

## Explicit Code Samples

```rust
pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("git status --short")
        .status()
}
```

```rust
#[cfg(unix)]
pub fn run_unix_hook() -> std::io::Result<std::process::ExitStatus> {
    std::process::Command::new("sh").arg("-c").arg("true").status()
}
```

## This Sprint Does Not Close

- production path-literal portability rules
- broad production environment-variable portability rules
- generic `cfg(unix)` / `cfg(windows)` structural parity checks

## Acceptance Criteria

- the sprint defines `PORT-009` in `sc-lint-portability` for consumer-neutral
  shell invocation portability checks
- the sprint explicitly covers both `Command::new("sh" | "bash")` and
  hardcoded `/bin/sh` or `/bin/bash` paths
- the sprint states that explicitly Unix-gated code paths are not false
  positives for this family, and names the existing `unix_gated` propagation
  path as the suppression mechanism
- the sprint names `visit_expr_for_unix_portability(...)` as the expression
  visitor seam for `is_shell_command_call(...)`
- the sprint states `C.7` as an ordering dependency because `C.8` extends the
  same expression-visitor seam added for the production env-portability family
- the sprint names the dedicated helper boundary for shell-path literals
  instead of leaving `/bin/sh` detection implicit in generic path matching
- the sprint keeps repo-local shell conventions and wrapper policy out of the
  shared product scope
- the sprint references GitHub issue `#55` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo test -p sc-lint-portability flags_command_new_sh_in_production_code`
- `cargo test -p sc-lint-portability flags_bin_bash_path_in_production_code`
- `cargo test -p sc-lint-portability passes_std_process_command_with_binary_name_in_production_code`
- `cargo test -p sc-lint-portability passes_cfg_unix_gated_shell_invocation_in_production_code`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
