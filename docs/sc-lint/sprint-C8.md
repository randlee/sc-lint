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
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `crates/sc-lint-portability/src/lib.rs`
- `crates/sc-lint-portability/src/portability.rs`
- `crates/sc-lint-portability/src/tests.rs`
- `crates/sc-lint-portability/README.md`
- `docs/sc-lint/sprint-C8.md`

## Deliverables

- `sc-lint-portability` adds `PORT-009` for production shell-invocation
  portability drift, covering:
  - `Command::new("sh")`
  - `Command::new("bash")`
  - hardcoded `/bin/sh`
  - hardcoded `/bin/bash`
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
  positives for this family
- the sprint keeps repo-local shell conventions and wrapper policy out of the
  shared product scope
- the sprint references GitHub issue `#55` in its hard dependencies

## Required Validation

- `cargo test -p sc-lint-portability`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
