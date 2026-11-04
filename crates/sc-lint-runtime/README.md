# `sc-lint-runtime`

`sc-lint-runtime` is the Rust analyzer used by the
[`sc-lint` product](https://crates.io/crates/sc-lint) for runtime and
concurrency checks. The `sc-runtime` name is the analyzer target accepted by
`sc-lint lint`.

Current crate release: **0.8.5**. Install the product CLI with
`brew install randlee/tap/sc-lint` or from
[GitHub Releases](https://github.com/randlee/sc-lint/releases). To use this
crate as a Rust library, run `cargo add sc-lint-runtime@0.8`. See the
[repository](https://github.com/randlee/sc-lint) and
[API documentation](https://docs.rs/sc-lint-runtime/0.8.5).

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

Rule details appear in the JSON `findings` array. For example, this is the
message produced for `SCB-RUNTIME-003` by the 0.8.5 binary:

```text
/private/tmp/sc-lint-docs-085-A-sample/src/lib.rs:2: SCB-RUNTIME-003 std::thread::sleep blocks the thread in Tokio package `docs-sample`; use event-driven synchronization or `yield_now` instead
```

## Invocation

Primary product surface:

```bash
sc-lint lint sc-runtime
sc-lint --json lint sc-runtime
```

There is no dedicated repo-local `just lint sc-runtime` wrapper. In the
sc-lint source repository, `just lint` runs the complete repository lint gate.

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

Representative text result from the 0.8.5 binary on a one-crate workspace
with no findings:

```text
sc-lint-runtime 0.8.5 status=pass scanned_crates=1 findings=0
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
/private/tmp/sc-lint-docs-085-A-sample/src/lib.rs:5: SCB-RUNTIME-001 bare Condvar::wait(...) in non-test production code; use wait_timeout(...) or wait_timeout_while(...) and inspect the WaitTimeoutResult
```

Discarding the timeout result is reported as `SCB-RUNTIME-002`; the JSON
finding message includes the source path and line.

The backend `sc-lint-runtime analyze` returns exit code `0` after a completed
analysis even when the report status is `fail`. The top-level
`sc-lint lint sc-runtime` command returns exit code `10` for a configured lint
failure.

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
`try_lock`, `unwrap`/`expect`/`?`. Temporary guards are released at the end
of their expression; lock-result match temporaries remain live through the arm.
Nested
block exit and explicit `drop` (including `std::mem::drop` aliases) release a
binding; branch analysis retains a guard if a continuing branch can keep it alive.

The tracker also recognizes `Arc::clone`/`Rc::clone` of Mutex owners,
file-local typed static Mutexes, `lock().unwrap_or_else(|e| e.into_inner())`,
constructors reached through globs (`use std::sync::*`, `Mutex::new`), and
`Ok(g) = m.lock()` patterns in if-let, let-else, match and while-let.
Pattern bindings live only within their scope; explicit drops release them.
Match arms track guards owned by `Ok(g)`, poisoned guards owned by `Err(p)`,
and unbound scrutinee guards in `Ok(_)` or fallback arms. A guard returned by
`match m.lock() { Ok(g) => g, Err(p) => p.into_inner() }` stays live in its
receiving local binding until dropped or the scope ends.
A visible `break` inside macro tokens (including `tokio::select!`) keeps a
loop exit reachable when joining guard states. If/match joins
exclude diverging branches (`return`, `break`, `continue`, built-in
`panic!`/`unreachable!`/`todo!`/`unimplemented!`, and statically known `!` calls).
A branch that reaches the following statement with a live guard still fails.

This is not type checking or borrow checking. Known false negatives include
macro-generated calls, indirect function pointers, reexports and Cargo crate
renames, type aliases, types returned by other functions, unrecognized receiver
fields/chains, and guards moved through other bindings, containers or deferred
captures. `use` bindings resolve by lexical module and function scope, so an
import in one module does not change findings in another; struct names are
flattened per file, and local name shadowing can cause false positives. Cross-function ownership transfer and
control-flow exits are not proved; consumed guards or branch/loop exits may
produce conservative findings. In particular, `consume(g); other().await`
and `loop { drop(g); break; } other().await` may still report rule 007;
interprocedural ownership and loop/labeled-block exit guard transfer are not proved.
Rule 003's legacy resolver does not recognize renamed `self` imports such as
`use std::thread::{self as thread}`; use a direct module alias instead.
For raw `try_lock()` result-variant arms, `Ok` patterns retain the guard,
including `Ok(_)` and `Ok(..)` scrutinees. Explicit
`Err(TryLockError::Poisoned(p))` bindings are also tracked. Ownership in unrefined
`Err(p)` and wildcard fallback arms is a known limitation; those arms are not
assumed to hold a guard. Move bindings of an entire scrutinee use the binding
name, so `drop(name)` releases the tracked resource. Blocking lock-result
rest and or-patterns retain their scrutinee resources. Dropping a nested
`ref` binding does not release the scrutinee-held guard.
Known `!` declarations are matched by unambiguous file-local names.
Unresolved callable return types are not assumed to diverge. Other `cfg`
conditions are not evaluated.

Runtime source discovery scans Rust files under production target source roots.
It does not resolve Rust module declarations when deciding whether a file is
test-only. For an out-of-line `#[cfg(test)] mod checks;` declared in
`src/lib.rs`, put the inner attribute `#![cfg(test)]` at the top of
`src/checks.rs` so that the separate file is classified as test code:

```rust
// src/checks.rs
#![cfg(test)]

#[test]
fn checks_behavior() {
    assert_eq!(2 + 2, 4);
}
```

The file-level marker makes the whole file test code for every runtime rule,
`SCB-RUNTIME-001` through `SCB-RUNTIME-007`, exactly like a file under
`tests/`. A parent module's `cfg(test)` attribute alone does not propagate into
the separate file.

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

- [logging.md](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/logging.md)
- [cli-contract.md](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/cli-contract.md)
