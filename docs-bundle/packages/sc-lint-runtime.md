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
sc-lint lint ci
```

The analyzer reads Rust source under the configured root and emits normalized
findings. The top-level product owns command dispatch and CI aggregation.

## Runtime rules

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

A function/method may declare a reasoned exception:

```rust
#[sc_lint_attributes::sc_lint(allow(SCB-RUNTIME-005), reason = "Protocol timer boundary")]
async fn timer() {
    tokio::time::sleep(duration).await;
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
blocks, explicit drop aliases and branches; temporaries do not stay live.
They do not expand macros or perform Rust type/borrow checking. Known false
negatives include indirect calls, reexports/crate renames, type aliases,
external return types, unrecognized receivers, and guards moved through other
bindings/containers/captures. File-flattened imports/struct names and unproved
control-flow exits can cause conservative false positives. Out-of-line
`#[cfg(test)]` parent-module context is not propagated to a separate file.

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
