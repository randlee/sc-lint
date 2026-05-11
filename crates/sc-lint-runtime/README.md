# `sc-runtime` User Guide

`sc-runtime` is the shipped runtime-analysis tool for std
concurrency/liveness checks.

## Purpose

Use `sc-runtime` to detect unsafe wait patterns in production Rust code.

Current rule families:

- `SCB-RUNTIME-001`
  - bare `Condvar::wait(...)` in production code
- `SCB-RUNTIME-002`
  - discarded `wait_timeout*` result in production code

## Invocation

Primary product surface:

```bash
sc-lint lint sc-runtime
sc-lint --json lint sc-runtime
```

There is no dedicated repo-local `just lint sc-runtime` wrapper in the current
release line. Use the top-level CLI or the backend-local command directly.

Backend-local path:

```bash
cargo run -p sc-lint-runtime -- analyze --root . --format text
cargo run -p sc-lint-runtime -- analyze --root . --format json
```

## Representative Pass

Production code that inspects the `WaitTimeoutResult`:

```rust
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub fn wait_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
    let guard = state.lock().expect("lock");
    let (_guard, wait) = condvar
        .wait_timeout(guard, Duration::from_secs(1))
        .expect("wait");
    if wait.timed_out() {
        return;
    }
}
```

Representative text result:

```text
sc-lint-runtime: PASS
scanned crates: 1
findings: 0
```

## Representative Fail

Bare wait with no timeout inspection:

```rust
use std::sync::{Condvar, Mutex};

pub fn block(condvar: &Condvar, state: &Mutex<bool>) {
    let guard = state.lock().expect("lock");
    let _guard = condvar.wait(guard).expect("wait");
}
```

Representative finding:

```text
SCB-RUNTIME-001 bare Condvar::wait(...) in non-test production code; use wait_timeout(...) or wait_timeout_while(...) and inspect the WaitTimeoutResult
```

Discarded timeout result:

```text
SCB-RUNTIME-002 wait_timeout* result discarded in non-test production code; inspect the returned WaitTimeoutResult before proceeding
```

## Disable Model

There is currently no approved rule-disable path in the shipped
`sc-lint lint sc-runtime` surface.

Important limits:

- no top-level CLI rule-disable flags
- no documented backend-specific allowlist or source-annotation suppression
  path in release `0.1.x`

If a future exception model is added, it must be backend-owned and documented
explicitly before users rely on it.

## Output And Logging

Canonical machine mode:

```bash
sc-lint --json lint sc-runtime
```

The top-level envelope is the shared CLI contract:

- success:
  - `ok: true`
  - `command: "lint.sc-runtime"`
  - findings payload under `data`
- failure:
  - `ok: false`
  - `command: "lint.sc-runtime"`
  - stable `CliError` object under `error`

Structured logs for this tool use:

- service name:
  - `sc-runtime`
- command identity:
  - `lint.sc-runtime`

Read the command lifecycle through:

- `cli.command.started`
- `cli.command.completed`
  - includes `summary` and `elapsed_ms`
- `cli.command.error`
  - includes stable error code and `CliError.kind`

Delegated/backend normalization records also appear:

- `cli.dispatch.started`
- `cli.dispatch.normalized`
  - includes the delegated tool name and normalized finding count

See:

- [logging.md](../../docs/sc-lint/logging.md)
- [cli-contract.md](../../docs/sc-lint/cli-contract.md)
