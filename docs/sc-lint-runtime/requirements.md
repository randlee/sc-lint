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
  imports and does not expand macros. A separate test-module file must carry
  its own inner `#![cfg(test)]` attribute to be classified as test code; the
  marker then exempts the whole file from every runtime rule, 001–007.
- `SCB-RUNTIME-003` exclusions use the runtime analyzer table:

  ```toml
  [tool.sc-lint.analyzers.runtime]
  reason = "staged migration to tokio::time::sleep"

  [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
  "SCB-RUNTIME-003" = ["crates/example/src/lib.rs"]
  ```

- known limitations of the sleep pass:
  - calls are matched by name, not by scope: a local item or binding that reuses
    an imported name, such as a local `sleep`, is not shadow-checked
  - macros are not expanded, so a sleep call produced by macro expansion is not
    seen
  - an out-of-line module's parent `#[cfg(test)] mod x;` attribute is not
    propagated; put `#![cfg(test)]` at the top of the separate file
- the crate must keep its machine contract stable through the top-level CLI
  normalization surface

## Stable consumer interfaces

Rule IDs and this analyzer's JSON output are stable consumer contracts
(`REQ-PRODUCT-002`, [ADR-011](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)).

## Related Docs

- [architecture.md](./architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)

## REQ-RUNTIME-001 — Pending Rand acceptance (0.8.5, #640)

- `SCB-RUNTIME-004` rejects `std::thread::sleep_until` with exactly the
  `SCB-RUNTIME-003` scope: production lib/bin source in workspace packages with
  a normal or build Tokio dependency. Dev-only Tokio does not enable either.
- `SCB-RUNTIME-005` rejects Tokio time `sleep` and `sleep_until`;
  `SCB-RUNTIME-006` rejects Runtime/Handle `block_on` and
  `futures::executor::block_on`; `SCB-RUNTIME-007` rejects a std Mutex guard
  bound in an async fn/block and live at a later `.await`.
  These three rules cover all source classified as non-test, regardless of a
  Tokio dependency. `yield_now` remains legal.
- Existing ScopeKind classification and the sleep pass's test-attribute
  handling exempt test targets/files, `tests/`, inline `cfg(test)` (including
  `all(test, ...)`), bare and framework test attributes. A separate test-module
  file whose first item is the inner attribute `#![cfg(test)]` is test code
  for every runtime rule, 001–007, exactly like a file under `tests/`.
  Put that marker at the top of the separate file; the parent module's
  `#[cfg(test)] mod checks;` does not propagate into it.
  Marker approval receipt: Rand, 2026-11-03: "Putting #![cfg(test)] at the top
  of the separate file is acceptable." This receipt accepts only the marker;
  REQ-RUNTIME-001 remains Pending.
- Syntactic grouped/renamed/glob `use` aliases are recognized. Qualified
  calls and identifiable typed/inferred Runtime/Handle and Mutex receivers are
  checked. Macro expansion, Rust type/borrow checking, reexports/crate renames,
  indirect calls and arbitrary interprocedural guard transfers are not claimed.
  The README records the precise approximations and false negatives.
- Function/method `#[sc_lint(allow(SCB-RUNTIME-00N), reason = "...")]`
  and its qualified `sc_lint_attributes::sc_lint` form suppress only listed
  rules in that body. Nested functions need their own allowance. Parse through
  `sc-lint-directives`; missing/blank reasons and unknown runtime IDs are
  reported, rather than silently suppressing findings.
- All new rules register in the runtime catalog, fail by default, and honor
  common analyzer configuration. The CLI exits 10 for unsuppressed findings,
  0 for test/exempt/allowed fixtures, and non-zero for malformed allowances.
  Tests drive source/AST inputs and never execute the prohibited operations.

## REQ-RUNTIME-002 — Proposed panic rule and adoption

Status: **AWAITING RAND ACCEPTANCE**. Before dispatch, accept or revise this
requirement and the proposed ADR-022 phase-q amendment; until then no phase-q
sprint may claim this as an accepted requirement.

`SCB-RUNTIME-008` detects production `.unwrap()`, `.expect(...)`, `panic!`,
`unreachable!`, `todo!`, and `unimplemented!` in opted-in Cargo packages. It
uses the existing runtime test-scope rules and existing ADR-022 function/method
allowance with a nonblank reason; a separate out-of-line test file carries its
own `#![cfg(test)]`, and parent-only cfg does not propagate. Nested functions
require their own allowance. Macro expansion and type-aware proof are not
claimed. The rule is off for packages not listed in the repository-root
configuration. The proposed opt-in exception to REQ-RUNTIME-001's new-rule
default is specified by the pending ADR-022 amendment. The runtime analyzer
validates its package list through its catalog-owned configuration keys; shared
analyzer configuration remains the single source. A recorded baseline is
provisionally outside phase q pending Rand; no request for a baseline is on
record. Runtime CLI help and its generated references include the new rule ID
and stay consistent.

## REQ-RUNTIME-003 — Proposed example target coverage

Status: **AWAITING RAND ACCEPTANCE**. Before dispatch, accept or revise this
requirement before dispatch.

Runtime analyzes supported Cargo example source for rules 001, 002, 005,
006 and 007, and for 008 after that rule is integrated and its package is
opted in. Rules 003 and 004 retain their deliberate exclusion from examples.
The 003/004 exclusion is documented in the runtime README and rule catalog;
no analyzer report schema change is required. Shared source discovery keeps
boundary and portability's existing example exclusion unless a separate approved requirement changes it.
Existing source-discovery entry points retain their signatures and defaults;
runtime uses a separate crate-private target-aware context for example discovery. The 003/004
sleep scope uses that target classification, including nested example modules.
Existing lib/bin target attribution remains stable; example-only files are
attributed deterministically to their example target.
