# sc-lint Extraction and Migration Plan

This document records the planned extraction of remaining generic lint/view
tooling into `sc-lint`, with special focus on migrating boundary inventory and
manifest-policy logic from Python into `sc-lint-boundary`.

It also records the planned introduction of a top-level `sc-lint` command-line
entry point that provides one stable surface across mixed Rust/Python tool
implementations.

## Scope

This plan covers five current tooling areas:

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

## Goals

- consolidate generic lint and view tooling in `sc-lint`
- keep ATM-specific runtime policy out of the shared repo
- move boundary inventory logic closer to the Rust analyzer that already owns
  AST-sensitive boundary rules
- preserve parity during migration by keeping the Python implementation as a
  validation oracle until Rust output is proven stable
- provide a single end-user CLI surface even while some tools remain Python
  backed

## Non-Goals

- migrating ATM-specific daemon/test-runtime lints
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

### Move to Rust

The following logic should migrate into `sc-lint-boundary`:

- TOML boundary record loading
- boundary schema validation
- duplicate-id and duplicate-source validation
- owner/package/path validation
- manifest dependency ownership rules
- manifest section rules
- inventory-parity checks
- future boundary metadata checks that depend on the canonical boundary model

### Stay in Python

The following should remain Python-based unless later correctness pressure
proves otherwise:

- line-count lint
- generic identity literal/content scan
- generic view plumbing
- wrappers around external tools used for report generation

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
- define config-loading behavior
- implement dispatch to existing backends

Initial expected shape:

- `sc-lint lint <tool>`
- `sc-lint view <tool>`
- `sc-lint version`

Deliverable:

- one stable CLI entry point for the repo
- backend implementations remain self-contained behind the dispatcher

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

Deliverable:

- `sc-lint` owns the generic Python utilities
- consumer repos call them through local wrappers or direct `sc-lint`
  invocation

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

## Config Work Needed

Before extraction completes, `sc-lint` needs a consumer-neutral config contract
for the extracted Python utilities and the Rust boundary loader.

Expected config surfaces:

- boundary discovery and policy config
- line-count thresholds and exclusions
- identity literal forbidden values and exemptions
- view output directories and enabled targets
- top-level CLI discovery and dispatch config where needed

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
