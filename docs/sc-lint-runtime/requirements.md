# `sc-lint-runtime` Requirements

These requirements define the crate-local rule ownership for `sc-lint-runtime`.

## Purpose

`sc-lint-runtime` is the shared analyzer crate for std
runtime/concurrency-correctness rules.

## Scope Rules

- generic std runtime and liveness rules land here
- `SCB-RUNTIME-003` rejects blocking `std::thread::sleep` in production lib/bin
  targets of workspace packages declaring Tokio. It honors the runtime
  exclusion settings, including `exclude_paths_by_rule`.
- ADR-020 decision 6 narrows ADR-007: Tokio-aware rules needing neither a Tokio
  crate dependency nor Tokio runtime semantics may live here; `sc-lint-tokio`
  remains reserved for rules that need either. The sleep pass uses flattened
  imports, does not expand macros, and treats out-of-line `#[cfg(test)]` module
  files as production files.
- `SCB-RUNTIME-003` exclusions use the runtime analyzer table:

  ```toml
  [tool.sc-lint.analyzers.runtime]
  reason = "staged migration to tokio::time::sleep"

  [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
  "SCB-RUNTIME-003" = ["crates/example/src/lib.rs"]
  ```

- known limitations of the sleep pass:
  - `use` imports are flattened across the whole file, including imports inside
    function bodies
  - calls are matched by name, not by scope: a local item or binding that reuses
    an imported name, such as a local `sleep`, is not shadow-checked
  - macros are not expanded, so a sleep call produced by macro expansion is not
    seen
  - out-of-line `#[cfg(test)] mod x;` files are production files
- the crate must keep its machine contract stable through the top-level CLI
  normalization surface

## Stable consumer interfaces

Rule IDs and this analyzer's JSON output are stable consumer contracts
(`REQ-PRODUCT-002`, [ADR-011](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)).

## Related Docs

- [architecture.md](./architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)
