# `sc-lint-runtime` Architecture

This document records the crate-local architecture summary for
`sc-lint-runtime`.

## Role

`sc-lint-runtime` owns AST-sensitive std runtime and concurrency analysis and
its backend machine contract.

## Authoritative Architecture Sources

- [../architecture.md](../architecture.md)
- [../../crates/sc-lint-runtime/README.md](../../crates/sc-lint-runtime/README.md)

## Boundary Rules

- the crate remains separate from `sc-lint-portability` and
  `sc-lint-boundary`
- generic std concurrency rules stay here
- `sleep.rs` owns the `SCB-RUNTIME-003` AST pass and its production-target and
  Tokio-package policy. ADR-020 decision 6 narrows ADR-007: rules requiring no
  Tokio dependency or runtime semantics may live here; `sc-lint-tokio` remains
  reserved for rules that do.
- `SleepScope` (in `runtime.rs`) decides which files the sleep pass visits. It
  keeps workspace packages that declare a normal or build `tokio` dependency and
  have a lib or bin target. Source files of the package's other targets (tests,
  examples, benches, build scripts) are excluded, and test files are skipped.
  The runtime analyzer table's `exclude_paths_by_rule` entries filter all
  runtime rule IDs (001–007) by path. Rules 003 and 004 additionally share
  the production Tokio-package eligibility described here.

## `SCB-RUNTIME-003` Scope (ATM-QA-008)

The accepted L7 design uses a package-scoped `SleepScope` in `runtime.rs`:
`excluded_roots` maps each eligible `PackageName` to its excluded `PathBuf`s, and
`includes` checks whether a file is in scope. Eligibility is limited to workspace
packages declaring a normal or build `tokio` dependency and having a lib or bin
target. Files belonging to tests, examples, benches, or build scripts are
excluded; test files are skipped as well.

`analyze_runtime_liveness` builds the scope once before parallel traversal.
`RuntimeCollector` continues to collect the existing 001/002 rules; after that
walk, the analyzer separately calls `sleep::find_blocking_sleeps` when the
scope includes the file. Keeping the package-keyed excluded roots and the
sleep pass outside `RuntimeCollector` is the accepted alternative for
ATM-QA-008, recorded in L7 design section B. It preserves the same
production-file behavior while keeping the new rule's eligibility separate
from the existing collector.

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)

## Runtime practice checks (0.8.5)

`practices.rs` owns the 004–007 syntax walk and lexical guard state. Its
`bindings` module resolves file-local use aliases and recognized receiver
types using the runtime-owned `api.rs` canonical registry. `imports.rs` walks
use trees for both practices and the legacy sleep pass; their adapters preserve
the distinct alias/glob recognition contracts. `scope.rs` calls analyzer support's shared test-attribute recognition and
scope classification while preserving rule 003's lack
of name-based test exemption. The `allowances` module parses the shared
reasoned directive model through `sc-lint-directives`. It has no Tokio crate dependency or runtime executor.
Rule 004 reuses `SleepScope` unchanged; 005–007 cover all non-test source.
Function/async-region boundaries isolate guard state, block boundaries remove
locals, explicit drops release guards, and branch states conservatively join.

Allowances validate before rule filtering and apply only to the annotated
function/method, with nested functions forming a fresh boundary. Root test
files and inline test scopes remain exempt. A file whose inner attributes mark
it test-only (`#![cfg(test)]`) is test code for every runtime rule, 001–007:
`runtime.rs` returns no findings for it before the 001/002 collector, the 003
sleep pass and the 004–007 practices run, exactly as for a file under
`tests/`. Existing 001–003 IDs/report shape are preserved; 003's recovery now suggests event-driven synchronization or
yielding because rule 005 rejects Tokio timers as well.

Branch-tail reachability excludes return/break/continue, built-in diverging
macros and statically known never-returning calls from if/match guard joins.
An internal four-variant PracticeRule makes conversion into runtime findings
exhaustive, without an unreachable fallback. Shared directive helpers own
attribute-path matching and analyzer-owned unknown-rule checks. Runtime
allowances cache parsed attributes and aggregate located errors in source order.

Lock-result Ok patterns introduce scoped guard bindings in if-let, let-else,
match and while-let. Match arms also track poison errors and unbound
scrutinee resources, and direct guard-valued match results transfer into
receiving local bindings. `PoisonError::into_inner` transfers ownership from
the error binding to its guard result. Break tokens inside opaque macro bodies conservatively
keep a loop's exit reachable; macro bodies are not otherwise expanded.
