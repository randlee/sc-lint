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

## REQ-RUNTIME-002: REQ-RUNTIME-002 panic rule and adoption

**Status:** Proposed  

### Requirement Statement

Definitions: REQ-RUNTIME-002 is this requirement's sc-lint identifier. `SCB-RUNTIME-008` is the runtime analyzer's panicking-call rule. Production code is source that the existing runtime test-scope rules classify as non-test. An opted-in package is a Cargo package listed in the repository-root configuration. The ADR-022 amendment is the phase-q ADR-022 amendment. The package list is the list of opted-in packages in the repository-root configuration. An analyzer's catalog-owned configuration keys are the analyzer-specific configuration keys that the analyzer publishes in its rule catalog. The generated runtime CLI references are the Markdown and HTML CLI reference documents for `sc-lint-runtime`. The generated runtime CLI references stay consistent with runtime CLI help when they match the runtime CLI help output.

Status: AWAITING RAND ACCEPTANCE. The package opt-in is an exception to REQ-RUNTIME-001's fail-by-default rule for new rules; the pending ADR-022 amendment specifies it. A recorded baseline is provisionally outside phase q pending Rand; no request for a baseline is on record.

**MUST statements:**

- `SCB-RUNTIME-008` MUST detect `.unwrap()`, `.expect(...)`, `panic!`, `unreachable!`, `todo!` and `unimplemented!` in production code of opted-in packages.
- `SCB-RUNTIME-008` MUST exempt test code under the existing runtime test-scope rules.
- A separate out-of-line test file MUST carry its own `#![cfg(test)]` to be exempt through that file's test marker.
- Suppression of `SCB-RUNTIME-008` MUST use the existing ADR-022 function/method allowance.
- An ADR-022 function/method allowance that suppresses `SCB-RUNTIME-008` MUST carry a nonblank reason.
- A nested function MUST carry its own allowance to be suppressed.
- The runtime analyzer MUST validate the package list through its catalog-owned configuration keys.
- Shared analyzer configuration MUST remain the single source of the package list.
- Runtime CLI help MUST include `SCB-RUNTIME-008`.
- The generated runtime CLI references MUST include `SCB-RUNTIME-008`.
- The generated runtime CLI references MUST stay consistent with runtime CLI help.

**MUST NOT statements:**

- `SCB-RUNTIME-008` MUST NOT run in Cargo packages that the repository-root configuration does not list.
- A parent-only `cfg(test)` attribute MUST NOT propagate into an out-of-line module file.
- `SCB-RUNTIME-008` MUST NOT claim macro expansion.
- `SCB-RUNTIME-008` MUST NOT claim type-aware proof.
- A phase-q sprint MUST NOT claim this requirement as accepted before Rand accepts it and the ADR-022 amendment.

### Rationale

Issue #931 asks for runtime detection of production panicking calls. Limiting the rule to opted-in packages leaves every other package and rule on its current defaults.

### Success Criteria

**Acceptance Criteria:**

- [ ] CLI fixture: in an opted-in package, each of the six call forms in production code with no allowance yields an `SCB-RUNTIME-008` finding in the `sc-lint lint sc-runtime` JSON report.
- [ ] CLI fixture: the same source in a package not listed in the repository-root configuration yields no `SCB-RUNTIME-008` finding.
- [ ] CLI fixture: the same calls in code that the existing runtime test-scope rules classify as test code yield no `SCB-RUNTIME-008` finding.
- [ ] CLI fixture: an out-of-line module file with its own `#![cfg(test)]` yields no finding. The same file without the marker, declared by a parent `#[cfg(test)] mod x;` in a production target, yields findings.
- [ ] CLI fixture: a function with `#[sc_lint(allow(SCB-RUNTIME-008), reason = "...")]` yields no finding in its body. A function nested inside it, without its own allowance, yields a finding.
- [ ] CLI fixture: an allowance for `SCB-RUNTIME-008` with a blank reason is reported as an error and does not suppress the finding.
- [ ] Configuration test: an invalid package list is rejected by the shared analyzer configuration loader through the runtime catalog's own configuration keys.
- [ ] Documentation check: `sc-lint-runtime --help` and the generated CLI references both list `SCB-RUNTIME-008`, and the generated reference matches the help output.
- [ ] Documentation review: the rule's documentation states that macro expansion and type-aware proof are not claimed.
- [ ] Phase-q review: no phase-q deliverable implements a recorded baseline unless Rand has decided to include one.
- [ ] Status check: this record stays Proposed until Rand's acceptance of it and of the phase-q ADR-022 amendment is recorded.

### Dependencies

**Related:**

- REQ-RUNTIME-003
- the phase-q ADR-022 amendment

### Test Strategy

**Test Types:**

- CLI integration test
- Configuration unit test
- Documentation check

## REQ-RUNTIME-003: REQ-RUNTIME-003 example target coverage

**Status:** Proposed  

### Requirement Statement

Definitions: REQ-RUNTIME-003 is this requirement's sc-lint identifier. Example source is the source of a supported Cargo example target, including its nested modules. Rules 001 to 008 are `SCB-RUNTIME-001` to `SCB-RUNTIME-008`. The runtime analyzer is `sc-lint-runtime`, the analyzer run by `sc-lint lint sc-runtime`. An example-only file is a file that an example target contains and no lib or bin target contains. A target-aware context records, for each source file, its package, its Cargo target and that target's kind (lib, bin, example, test, bench, build script or other). A crate-private context is visible only inside the runtime analyzer crate. A separate context is one apart from the existing source-discovery entry points. The 003/004 sleep scope is the runtime analyzer's selection of the files that rules 003 and 004 visit. A file's target classification is the target kind that the target-aware context records for that file. A file is attributed deterministically when repeated runs over the same workspace attribute it to the same example target. Shared source discovery is the source-file discovery that the shared analyzer support library provides to the boundary, portability and runtime analyzers. The existing example exclusion is that shared source discovery walks only each package's `src/` and `tests/` directories, so it does not discover example source outside them. The existing source-discovery entry points are the shared scanner's public functions `discover_source_files` and `discover_source_files_with_metadata`. Their defaults are their current behaviour of discovering Rust files only under each workspace package's `src/` and `tests/` directories.

Status: AWAITING RAND ACCEPTANCE. Rand must accept or revise this requirement before dispatch.

**MUST statements:**

- The runtime analyzer MUST analyze supported Cargo example source for rule 001.
- The runtime analyzer MUST analyze supported Cargo example source for rule 002.
- The runtime analyzer MUST analyze supported Cargo example source for rule 005.
- The runtime analyzer MUST analyze supported Cargo example source for rule 006.
- The runtime analyzer MUST analyze supported Cargo example source for rule 007.
- The runtime analyzer MUST apply rule 008 to example source once rule 008 is integrated and the example's package is opted in.
- The runtime README MUST document the 003/004 example exclusion.
- The runtime rule catalog MUST document the 003/004 example exclusion.
- The runtime analyzer MUST discover example source through a separate crate-private target-aware context.
- The 003/004 sleep scope MUST use that target classification, including for nested example modules.
- Each example-only file MUST be attributed deterministically to its example target.

**MUST NOT statements:**

- Rules 003 and 004 MUST NOT report findings in example source.
- Shared source discovery MUST NOT change boundary's or portability's existing example exclusion unless a separate approved requirement changes it.
- Existing source-discovery entry points MUST NOT change their signatures.
- Existing source-discovery entry points MUST NOT change their defaults.
- Existing lib/bin target attribution MUST NOT change.

### Rationale

Issue #935: the shared source scanner walks only package `src/` and `tests/`, so example source outside those directories is not analyzed while `scanned_crates` still counts its package. Rules 003 and 004 deliberately exclude examples. No analyzer report schema change is required.

### Success Criteria

**Acceptance Criteria:**

- [ ] CLI fixture: an example-only `Condvar::wait` yields an `SCB-RUNTIME-001` finding with the example path. Fixtures for rules 002, 005, 006 and 007 in example source each yield that rule's finding.
- [ ] CLI fixture (after rule 008 is integrated): an opted-in package's example source yields `SCB-RUNTIME-008` findings, and a non-opted-in package's example source yields none.
- [ ] CLI fixture: blocking sleep and `sleep_until` in example source of a Tokio package, including a nested example module, yield no `SCB-RUNTIME-003` or `SCB-RUNTIME-004` finding.
- [ ] Documentation review: the runtime README and rule catalog state the 003/004 example exclusion.
- [ ] Schema test: the runtime report validates against the existing analyzer report schema artifacts, unchanged.
- [ ] Regression tests: boundary and portability results on a workspace with example targets are the same before and after the change.
- [ ] API check: the signatures and default behaviour of the existing source-discovery entry points are unchanged, and the example-discovery context is crate-private to the runtime analyzer.
- [ ] Regression test: findings in existing lib/bin source keep the same target attribution as before the change.
- [ ] Determinism test: repeated runs attribute each example-only file to the same example target.
- [ ] Status check: this record stays Proposed until Rand's acceptance is recorded.

### Dependencies

**Related:**

- REQ-RUNTIME-002
- the phase-q ADR-022 amendment

### Test Strategy

**Test Types:**

- CLI integration test
- Cross-analyzer regression test
- Schema test
