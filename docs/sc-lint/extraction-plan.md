# sc-lint Extraction and Migration Plan

This document records the planned extraction of remaining generic lint/view
tooling into `sc-lint`, with special focus on:

- migrating boundary inventory and manifest-policy logic from Python into
  `sc-lint-boundary`
- backporting reusable lint families that are first proven on `atm-core`

It also records the planned introduction of a top-level `sc-lint` command-line
entry point that provides one stable surface across mixed Rust/Python tool
implementations, plus the planned distribution of imported analyzer families
into narrow dedicated crates.

## Scope

This plan covers six current tooling areas and one proof-to-product migration
track:

1. boundary inventory and manifest-policy enforcement
   - current implementation: Python `lint_boundaries.py`
   - target: Rust `sc-lint-boundary`
2. line-count lint
   - current implementation: Python `check_line_counts.py`
   - target: standalone generic `sc-lint` Python utility
3. identity literal lint
   - current implementation: Python `check_test_identity_literals.py`
   - target: standalone generic `sc-lint` Python utility first
4. generic view plumbing
   - current implementation: Python `run_view.py`, `build_view_site.py`,
     `view_common.py`
   - target: standalone generic `sc-lint` Python utilities
5. selected view targets
   - current implementation: Python wrappers around inventory generation and
     external tooling
   - target: mixed extraction, depending on whether the view is orchestration
     or graph-derived analysis

It also covers one repo-architecture item:

6. top-level `sc-lint` CLI
   - target: one stable command-line entry point with command parsing,
     consistent output, config loading, and tool dispatch

7. postmortem lint families first proven on `atm-core`
   - current implementation split:
     - reusable analyzer rules proven in embedded `crates/sc-lint-*`
     - ATM-local policy lints kept in `.just/`
   - target:
     - migrate reusable analyzer rules into standalone `sc-lint`
     - keep ATM-local policy lints in the consumer repo unless only the
       underlying framework is extracted

## Goals

- consolidate generic lint and view tooling in `sc-lint`
- keep ATM-specific runtime policy out of the shared repo
- make the consumer-repo proving path explicit:
  - prove on `atm-core`
  - migrate only the reusable rule family or framework
- define where cross-target compile preflight belongs in the product so
  consumer repos can catch likely Windows/Linux drift earlier
- move boundary inventory logic closer to the Rust analyzer that already owns
  AST-sensitive boundary rules
- preserve parity during migration by keeping the Python implementation as a
  validation oracle until Rust output is proven stable
- provide a single end-user CLI surface even while some tools remain Python
  backed

## Non-Goals

- migrating ATM-specific daemon/test-runtime lints
- copying consumer-specific policy into `sc-lint` unchanged just because the
  underlying implementation shape is reusable
- forcing all current Python utilities into Rust
- rewriting report/site-generation plumbing into Rust without a correctness
  benefit
- creating tight coupling between specialized tool crates without an explicit
  design review

## Target Split

### Top-level CLI

Add a top-level `sc-lint` CLI crate that owns:

- command parsing
- config loading
- tool selection and dispatch
- consistent output formatting
- consistent exit-code behavior
- future cross-target preflight orchestration
- the canonical top-level machine-readable contract

The top-level CLI may call:

- Rust library APIs where available
- Python utilities or specialized binaries where migration is still in
  progress

This gives `sc-lint` one stable user-facing contract while backend
implementations continue to evolve.

### Crate isolation rule

Each specialized backend crate should remain self-contained.

Default rule:

- tool crates do not reference each other directly

Allowed shared dependencies:

- `sc-lint-directives`
- future shared support crates only after explicit design approval

This means, for example:

- `sc-lint-boundary` should not reach into another tool crate for business
  logic
- future line-count or portability crates should not depend on each other by
  default
- coordination belongs in the top-level `sc-lint` CLI, not in backend crate
  cross-calls

### Move to Rust / dedicated analyzer crates

The following logic should migrate into `sc-lint-boundary`:

- TOML boundary record loading
- boundary schema validation
- duplicate-id and duplicate-source validation
- owner/package/path validation
- manifest dependency ownership rules
- manifest section rules
- inventory-parity checks
- future boundary metadata checks that depend on the canonical boundary model

The following reusable analyzer families should migrate into dedicated crates:

- `sc-lint-portability`
  - `PORT-001` hardcoded Unix-only absolute paths in test code
  - `PORT-002` direct `dirs::home_dir()` without configured override check
  - `PORT-003` `std::env::set_var()` in test code
  - `PORT-004` ungated `std::os::unix` imports in production code
  - `PORT-005` `cfg_attr(not(unix), allow(dead_code))` portability suppressors
- `sc-lint-runtime`
  - `SCB-RUNTIME-001` bare production `Condvar::wait(...)`
  - `SCB-RUNTIME-002` discarded `wait_timeout*` results in production code

Planned later crate:

- `sc-lint-tokio`
  - Tokio-specific runtime rules only
  - not part of the current import scope

### Stay in Python

The following should remain Python-based unless later correctness pressure
proves otherwise:

- line-count lint
- generic identity literal/content scan
- generic view plumbing
- wrappers around external tools used for report generation

Current ATM-local policy lints that should not migrate unchanged:

- duplicate semantic string-literal policy in non-test production Rust code
- fixed `thread::sleep(...)` test-hygiene policy
- triage Turtle aggregate/branch consistency policy

### Mixed / later decision

Selected view targets should move only when they are naturally derived from the
Rust analyzer graph, for example:

- boundary graph or inventory views sourced directly from `sc-lint-boundary`
- future graph-db/export-backed visualizations

Views that are mostly orchestration should remain Python:

- site assembly
- HTML/XHTML collation
- external-tool artifact wrapping

## Planned Phases

### Phase 0: Add the top-level `sc-lint` CLI

Create a top-level command-line crate before the remaining extraction work
sprawls across more entry points.

Required work:

- create `sc-lint` CLI crate
- define top-level command structure
- define shared output/exit-code conventions
- define shared machine-readable success/failure conventions
- define the canonical top-level `--json` mode and backend translation policy
- define config-loading behavior
- implement dispatch to existing backends

Initial expected shape:

- `sc-lint lint <tool>`
- `sc-lint view <tool>`
- `sc-lint version`
- `sc-lint ci`

Release-1 note:

- `lint` owns the primary crate-mapped backend targets
- `view` remains a narrower grouped surface whose targets are documented
  individually as they become stable

Initial expected profile shape:

- `sc-lint lint fast`
- `sc-lint lint full`
- `sc-lint lint ci`

Initial expected `xwin` shape when capability is present:

- `sc-lint check xwin`
- `sc-lint clippy xwin`

Deliverable:

- one stable CLI entry point for the repo
- one explicit machine-readable contract for non-interactive command families
- backend implementations remain self-contained behind the dispatcher

### Phase 0.5: Define cross-target preflight support

Decide how `sc-lint` should orchestrate cross-target compile checks that can
surface likely platform drift before CI.

Required work:

- document the supported targets and prerequisites
- document `cargo xwin` as the initial Windows preflight mechanism if
  consumer-repo validation continues to hold
- define explicit `xwin`-aware command shapes rather than burying the feature
  only inside generic lint-target names
- measure developer-cost impact before adding any cross-target check to a
  default gate
- record the expected split between:
  - `xwin check` as the likely first promotion candidate
  - `xwin clippy` as a stronger follow-up path that may stay non-default
- record the profile policy:
  - `fast` excludes `xwin` to preserve low-latency local feedback
  - `full` includes `xwin check` and `xwin clippy` when available
  - `ci` excludes `xwin` because real Windows CI already exists

Deliverable:

- one documented cross-target preflight path
- no ambiguity about the difference between cross-target compile checks and
  authoritative native-platform CI validation
- no ambiguity about the difference between:
  - `sc-lint lint ci`
  - `sc-lint ci`

### Phase 1: Extract generic Python utilities

Move the non-boundary generic Python tools into `sc-lint`:

- line-count lint
- identity literal lint
- generic view plumbing

Required work:

- rename ATM-specific file/function terminology to consumer-neutral names
- define repo config inputs in `sc-lint`
- add standalone fixture tests
- keep repo wrappers thin in the consumer repo
- expose extracted utilities through the top-level `sc-lint` CLI
- normalize Python utility machine output through one adapter schema
  (`sc-lint-python-v1`) before the top-level CLI wraps results into
  `CommandEnvelope<T>` or `CliError`

Deliverable:

- `sc-lint` owns the generic Python utilities
- consumer repos call them through local wrappers or direct `sc-lint`
  invocation
- the initial extracted command surfaces are:
  - `sc-lint lint line-counts`
  - `sc-lint lint identity-literals`
  - `sc-lint view findings`

Phase 1 note on identity literals:

- the current `atm-core` role-name gate is a consumer policy, not a shared
  default
- if extracted later, `sc-lint` should expose a configurable literal-policy
  framework rather than hardcoding ATM role names

### Phase 1.5: Add `sc-lint-portability`

Create the dedicated portability analyzer crate and move all shared
portability rules into it.

Required work:

- create `sc-lint-portability`
- move the existing portability implementation out of
  `crates/sc-lint-boundary/src/portability.rs`
- keep `PORT-001/002/003` with the same rule ids under `sc-lint-portability`
- port `PORT-004` and `PORT-005` into `sc-lint-portability`
- retarget the current portability wrapper surface to the new crate once it
  exists
- add rule documentation and tests in `sc-lint`
- keep the consumer repo (`atm-core`) as the first validation target after
  the backport

Deliverable:

- `sc-lint-portability` owns the shared portability rule family
- the existing portability code path has moved out of `sc-lint-boundary`
- consumer repos can consume the shared portability family without copying
  ATM-local policy

### Phase 1.6: Add `sc-lint-runtime`

Create the dedicated std runtime/concurrency analyzer crate and move the
shared runtime rules into it.

Required work:

- create `sc-lint-runtime`
- port `SCB-RUNTIME-001` and `SCB-RUNTIME-002` into `sc-lint-runtime`
- add rule documentation and tests in `sc-lint`
- keep the consumer repo (`atm-core`) as the first validation target after
  the backport

Deliverable:

- `sc-lint-runtime` owns the shared std runtime rule family
- the imported runtime families land in a dedicated analyzer crate rather than
  widening `sc-lint-boundary`
- consumer repos can consume those rules without copying ATM-local policy
- release-1 CLI exposure remains `sc-lint lint sc-runtime` through delegated
  backend execution and top-level output normalization

### Phase 2: Introduce Rust boundary inventory loader

Add boundary inventory loading and schema validation to `sc-lint-boundary`.

Required work:

- load canonical TOML boundary records
- validate schema and required fields
- detect duplicate ids and duplicate authoritative records
- emit stable findings for schema/inventory failures
- expose the Rust inventory loader through the top-level `sc-lint` CLI

Deliverable:

- Rust can load and validate boundary inventory without Python assistance

### Phase 3: Add manifest-policy enforcement to Rust

Port the generic manifest-policy portion of `lint_boundaries.py` into
`sc-lint-boundary`.

Required work:

- model dependency ownership rules
- model manifest section rules
- validate `Cargo.toml` edges against boundary/config policy
- preserve stable finding categories across the migration
- route execution through the top-level `sc-lint` CLI without adding direct
  dependencies on unrelated backend crates

Deliverable:

- Rust owns boundary inventory plus manifest-policy enforcement

### Phase 4: Parity validation window

Keep the Python boundary implementation and run it as a parity validator
against the Rust implementation.

Required work:

- define parity fixtures
- compare Python vs Rust findings on:
  - valid boundary inventory
  - invalid boundary inventory
  - manifest-policy failures
  - mixed-source migration repos where still relevant
- fail parity tests on mismatch
- ensure parity runs can be triggered from the top-level CLI or equivalent test
  harness without violating backend crate isolation

Deliverable:

- confidence that the Rust migration preserved current enforcement behavior

### Phase 5: Promote Rust to primary boundary engine

Once parity is stable:

- make `sc-lint-boundary` the primary boundary implementation
- keep Python available as reference validation for at least one release cycle
- then decide whether to:
  - retain Python as a long-term oracle, or
  - deprecate it once the Rust implementation is fully trusted

Deliverable:

- one primary boundary engine in Rust
- Python retained temporarily for migration safety

## Testing Requirements

### Boundary migration to Rust

At minimum, add:

- positive TOML boundary load tests
- malformed TOML tests
- missing required field tests
- duplicate `boundary_id` tests
- duplicate authoritative source tests
- manifest section rule positive/negative tests
- dependency ownership positive/negative tests
- parity tests against the Python implementation

### Generic Python utilities

At minimum, add:

- line-count threshold tests
- exclusion/allowlist tests
- identity literal positive/negative tests
- view plumbing tests for artifact collation and output layout
- selected view target tests using standalone fixture repos where possible

### Backported postmortem analyzer families

At minimum, add:

- positive and negative tests for:
  - ungated `std::os::unix` imports
  - `cfg_attr(not(unix), allow(dead_code))` suppressors
  - bare `Condvar::wait(...)`
  - discarded `wait_timeout*` results
- consumer-repo validation runs proving the standalone `sc-lint` copy matches
  the `atm-core` proving behavior

### Cross-target preflight

At minimum, add:

- one documented target matrix for preflight support
- fixture or sample validations proving the chosen command surfaces expected
  `cfg`/import drift
- timing measurements so the project can decide whether the check belongs in a
  default local path or an explicit pre-push path
- initial validation data for:
  - `cargo xwin check --target x86_64-pc-windows-msvc`
  - `cargo xwin clippy --target x86_64-pc-windows-msvc -- -D warnings`

## Config Work Needed

Before extraction completes, `sc-lint` needs a consumer-neutral config contract
for the extracted Python utilities and the Rust boundary loader.

Expected config surfaces:

- boundary discovery and policy config
- line-count thresholds and exclusions
- identity literal forbidden values and exemptions
- view output directories and enabled targets
- top-level CLI discovery and dispatch config where needed
- any future configurable consumer-policy framework extracted from ATM-local
  literal/content lints

## CLI Requirements

The top-level `sc-lint` CLI should provide:

- command parsing
- consistent text and machine-readable output
- stable exit-code behavior across delegated tools
- repo-root discovery
- config loading before dispatch
- dispatch to self-contained backend tools

It should not require backend tool crates to depend on each other just to share
one entry point.

## ATM-Specific Items That Stay Out

The following should remain in the consumer repo and not be extracted into
`sc-lint`:

- daemon-singleton/no-spawn lint
- consumer-specific boundary inventories
- runtime/test-policy rules tied to one application architecture
- ATM role-name duplication policy as currently expressed
- ATM triage-record consistency policy as currently expressed

## Completion Criteria

This migration is complete when:

- a top-level `sc-lint` CLI exists and is the preferred user-facing entry
  point
- `sc-lint-boundary` owns boundary inventory + manifest-policy enforcement
- Python boundary logic remains available for parity validation during the
  transition window
- line-count, identity literal, and generic view plumbing are owned by
  `sc-lint`
- ATM-specific lints remain only in the consumer repo
- the `sc-lint` project plan and docs describe the split clearly
