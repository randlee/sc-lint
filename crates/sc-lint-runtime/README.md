# `sc-runtime` User Guide

`sc-runtime` is the shipped runtime-analysis tool for std
concurrency/liveness checks.

## Purpose

Use `sc-runtime` to detect unsafe wait patterns in production Rust code.

Current rule families:

- `SCB-RUNTIME-001`
  - bare `Condvar::wait(...)` in production code
- `SCB-RUNTIME-002`
  - discarded `Condvar::wait_timeout*` result
- `SCB-RUNTIME-003`
  - blocking `std::thread::sleep` in production Tokio packages; use
    event-driven synchronization or `yield_now`

- `SCB-RUNTIME-004`
  - `std::thread::sleep_until` in the same production Tokio-package scope as
    rule 003 (normal/build Tokio dependency, production lib/bin targets)
- `SCB-RUNTIME-005`
  - `tokio::time::sleep` and `tokio::time::sleep_until` in all non-test source
- `SCB-RUNTIME-006`
  - `tokio::runtime::Runtime::block_on`, `Handle::block_on`, and
    `futures::executor::block_on` in all non-test source
- `SCB-RUNTIME-007`
  - a bound `std::sync::Mutex` guard live at `.await` inside an async function
    or async block, in all non-test source

Rules 005–007 do not require a declared Tokio dependency. `yield_now` remains
legal. Prefer event-driven synchronization; a timer is not a replacement for
another forbidden sleep. The checks inspect syntax, not a running service.

Sample `SCB-RUNTIME-003` finding:

```text
SCB-RUNTIME-003 std::thread::sleep blocks the thread in Tokio package `example`; use event-driven synchronization or `yield_now` instead
```

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

## Configuration and local allows

The runtime analyzer supports the shared `enabled`, `exclude_rules`,
`fail_rules`, `exclude_paths_by_rule`, and `--rule-filter` configuration.
Every rule fails by default. A function or method can allow listed rules only
with a non-empty reason, using the existing shared directive syntax:

```rust
#[sc_lint_attributes::sc_lint(
    allow(SCB-RUNTIME-005),
    reason = "Compatibility protocol requires this timer boundary"
)]
async fn protocol_timer() {
    tokio::time::sleep(duration).await;
}
```

The unqualified `#[sc_lint(...)]` form also works. The attribute macro comes
from `sc-lint-attributes` and is a no-op on every OS. An allowance applies to
that function/method's body and only its listed IDs; nested functions need
their own allowance. Module/impl allowances do not authorize their members.
Suppressed sites are omitted from findings; their reason remains in source.
A missing or blank reason is an analyzer error with
`DIRECTIVE.SITE_ALLOW.MISSING_REASON` or `DIRECTIVE.SITE_ALLOW.EMPTY_REASON`
and recovery guidance. Unknown runtime rule IDs are also rejected.

Test targets and `tests/` files, inline `#[cfg(test)]` (including
`cfg(all(test, ...))`), `#[test]`, and qualified test attributes such as
`#[tokio::test]` are exempt. Fixtures in the test suite are parsed and analyzed;
none of their sleep/blocking calls are executed.

## Syntactic coverage and limits

Grouped, renamed and glob `use` imports are resolved syntactically, as in rule
003. Method `block_on` is recognized for explicitly typed Runtime/Handle
parameters, inferred constructors/builders/handles, and typed `self` fields.
Mutex tracking recognizes typed Mutex/Arc<Mutex> parameters, local constructors,
typed `self` fields and explicit MutexGuard bindings, including `lock`/
`try_lock`, `unwrap`/`expect`/`?`. Temporary guards are not retained. Nested
block exit and explicit `drop` (including `std::mem::drop` aliases) release a
binding; branch analysis retains a guard if a continuing branch can keep it alive.

The tracker also recognizes `Arc::clone`/`Rc::clone` of Mutex owners,
file-local typed static Mutexes, `lock().unwrap_or_else(|e| e.into_inner())`,
constructors reached through globs (`use std::sync::*`, `Mutex::new`), and
`if let Ok(g) = m.lock()` bindings scoped to the then-body. If/match joins
exclude diverging branches (`return`, `break`, `continue`, built-in
`panic!`/`unreachable!`/`todo!`/`unimplemented!`, and statically known `!` calls).
A branch that reaches the following statement with a live guard still fails.

This is not type checking or borrow checking. Known false negatives include
macro-generated calls, indirect function pointers, reexports and Cargo crate
renames, type aliases, types returned by other functions, unrecognized receiver
fields/chains, and guards moved through other bindings, containers or deferred
captures. `use` bindings and struct names are flattened per file; local name
shadowing can cause false positives. Cross-function ownership transfer and
control-flow exits are not proved; consumed guards or branch/loop exits may
produce conservative findings. In particular, `consume(g); other().await`
and `loop { drop(g); break; } other().await` may still report rule 007;
interprocedural ownership and loop/labeled-block exit guard transfer are not proved.
Known `!` declarations are matched by unambiguous file-local names.
Unresolved callable return types are not assumed to diverge. Other `cfg`
conditions are not evaluated.

Runtime source discovery scans Rust files under production target source roots.
It does not resolve Rust module declarations when deciding whether a file is
test-only. An out-of-line `#[cfg(test)] mod checks;` declared in `src/lib.rs`
therefore does not mark `src/checks.rs` as a test file; blocking sleeps in that
file may be reported as production code. Keep test modules inline or place
them under a Cargo test target until module-aware source classification is
available.

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
