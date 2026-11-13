# sc-lint-runtime

## Purpose and ownership

`sc-lint-runtime` is the AST-sensitive analyzer for runtime safety patterns,
including blocking waits, detached work, and supervision-sensitive execution.
It ships a backend binary and library.

## Intended users

Rust service and library maintainers use it through `sc-lint lint sc-runtime`;
runtime owners review each finding against the service's shutdown and
concurrency model.

## Configuration and commands

```sh
sc-lint lint sc-runtime
sc-lint lint ci --consumer
```

The analyzer reads Rust source under the configured root and emits normalized
findings. The top-level product runs configured commands; the consumer owns
CI orchestration.

## Runtime rules

`SCB-RUNTIME-008` detects production `.unwrap()`, `.expect(...)`, `panic!`, `unreachable!`, `todo!`, and `unimplemented!` in opted-in packages. Matching is syntactic, so methods with these names are detected regardless of receiver type. Macro bodies are not expanded, and type-aware proof is not performed.

Runtime rules 001, 002, and 005–007 analyze supported Cargo example targets, including nested modules. Rules 003 and 004 exclude example targets.

- `SCB-RUNTIME-001`: Bare `Condvar::wait(...)` in production code; use a
  timeout and inspect the result.
- `SCB-RUNTIME-002`: Discarded `Condvar::wait_timeout*` result; inspect the
  returned timeout result before proceeding.
- `SCB-RUNTIME-003`: Blocking `std::thread::sleep` in production Tokio
  packages; use event-driven synchronization or `yield_now` instead.

- `SCB-RUNTIME-004`: `std::thread::sleep_until`, with exactly the same
  production lib/bin Tokio-package scope as 003 (normal/build dependency).
- `SCB-RUNTIME-005`: `tokio::time::sleep` and `sleep_until`, all non-test source.
- `SCB-RUNTIME-006`: Runtime/Handle `block_on` and
  `futures::executor::block_on`, all non-test source.
- `SCB-RUNTIME-007`: A bound std Mutex guard live at `.await` in an async fn
  or async block, all non-test source.

Rules 005–007 do not require a declared Tokio dependency. `yield_now` is legal.
Test files/targets and inline `cfg(test)`, `#[test]` and framework test
attributes are exempt. Prefer event-driven synchronization over sleeps.

A function/method may declare a reasoned exception. `SCB-RUNTIME-003` uses
the same allowance path as rules 004–007; the allowance covers only that body,
so sibling and nested functions still need their own reasoned exceptions:

```rust
#[sc_lint_attributes::sc_lint(allow(SCB-RUNTIME-003), reason = "Bounded synchronous compatibility polling")]
fn bounded_sync_poll(poll_interval: std::time::Duration) {
    std::thread::sleep(poll_interval);
}
```

The unqualified `sc_lint` attribute works too; use the `sc-lint-attributes`
no-op macro when compiling. A non-empty reason is mandatory; missing/blank
reasons produce `DIRECTIVE.SITE_ALLOW.MISSING_REASON`/
`DIRECTIVE.SITE_ALLOW.EMPTY_REASON`. Nested functions require their own allow;
module/impl allows do not exempt members. Runtime configuration still supports
`exclude_rules`, `fail_rules`, `exclude_paths_by_rule` and `--rule-filter`.

Checks resolve grouped/renamed/glob use aliases, explicit receiver types,
recognized constructors, and typed self fields. Guard liveness follows lexical
blocks, explicit drop aliases and branches. Temporaries end with their
expression; lock-result match scrutinees stay live through each arm.
The tracker recognizes `Arc::clone`/`Rc::clone`, typed file-local static
Mutexes, poison recovery through `lock().unwrap_or_else(|e| e.into_inner())`,
glob-qualified `Mutex::new`/`Runtime::new`, and `Ok(g) = m.lock()` patterns
in if-let, let-else, match and while-let. Guards stay within their binding
scope and explicit drops release them. Match arms track `Ok(g)` guards,
`Err(p)` poison errors that own guards, and unbound scrutinee resources in
`Ok(_)` or fallback arms. A guard returned by
`match m.lock() { Ok(g) => g, Err(p) => p.into_inner() }` stays live in its
receiving local binding. Visible `break` tokens inside macros
(including `tokio::select!`) keep loop exits reachable for guard-state joins. Diverging return/break/continue branches,
built-in panic/unreachable/todo/unimplemented macros and statically known `!`
calls do not contribute to an if/match guard join.

They do not expand macros or perform Rust type/borrow checking. Known false
negatives include indirect calls, reexports/crate renames, type aliases,
external return types, unrecognized receivers, and guards moved through other
bindings/containers/captures. Imports resolve by lexical module and function
scope. File-flattened type/static names and unproved control-flow exits can
cause conservative false positives. In particular,
`consume(g); other().await` and `loop { drop(g); break; } other().await` may
still report 007: interprocedural moves and loop/labeled-block exit guard transfer are not
proved.
For raw `try_lock()` result-variant arms, `Ok` patterns retain the guard,
including `Ok(_)` and `Ok(..)` scrutinees. Explicit
`Err(TryLockError::Poisoned(p))` bindings are also tracked. Ownership in unrefined
`Err(p)` and wildcard fallback arms is a known limitation; those arms are not
assumed to hold a guard. Move bindings of an entire scrutinee use the binding
name, so `drop(name)` releases the tracked resource. Blocking lock-result
rest and or-patterns retain their scrutinee resources. Dropping a nested
`ref` binding does not release the scrutinee-held guard.
Known `!` declarations are matched by unambiguous file-local names.
Unresolved callable return types are not assumed to diverge. For an out-of-line
`#[cfg(test)] mod checks;` in `src/lib.rs`, put `#![cfg(test)]` at the top of
`src/checks.rs`:

```rust
// src/checks.rs
#![cfg(test)]

#[test]
fn checks_behavior() {
    assert_eq!(2 + 2, 4);
}
```

The file-level marker makes the whole file test code for every runtime rule,
`SCB-RUNTIME-001` through `SCB-RUNTIME-008`, exactly like a file under
`tests/`. The parent module's `cfg(test)` attribute alone does not propagate
into it.

## Finding interpretation and CI

A finding marks a discarded or unbounded runtime operation and includes the
source location and rule context. Prefer supervised regions, explicit timeout
handling, and observed results; document any intentional exception.

## Common failures

Bare waits, discarded timeout results, and detached tasks commonly fail the
profile. Inspect the exact operation and preserve error context while fixing
the ownership or timeout boundary.

## Related packages

Use [sc-lint-portability](./sc-lint-portability.md) for cross-platform concerns,
[sc-lint-boundary](./sc-lint-boundary.md) for architecture, and
[sc-lint-schema](./sc-lint-schema.md) for output types.
