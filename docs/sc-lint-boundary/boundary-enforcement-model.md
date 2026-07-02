# Boundary Enforcement Model

Related ADR:
- [`../sc-lint/adr/ADR-004-structured-boundary-definitions.md`](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [`../sc-lint/adr/ADR-006-ai-first-cli-contract.md`](../sc-lint/adr/ADR-006-ai-first-cli-contract.md)

Related requirements:
- [`./requirements.md`](./requirements.md)

## Purpose

This document records the planned boundary-inventory enforcement models in
`sc-lint-boundary`, including:

- planning-aware inventory-parity checks for documented boundary items
- fail-only package dependency policy for direct workspace package edges

Current implementation note:

- A.7 adds Rust-native manifest-policy checks and parity tests against
  `.just/lint_manifests.py`
- that work does not replace the inventory-parity model described here
- manifest policy remains separate from boundary-inventory package dependency
  enforcement
- package dependency fields already exist in boundary TOML, but direct
  workspace package-edge enforcement is still the planned `D.1` follow-on
- planning-aware missing-item enforcement remains a later stage built on the
  A.6 loader foundation

The goal is:

- hard gate architectural drift
- preserve visibility for planned future work
- avoid using indefinite warning states as a suppression system

## Problem

The current boundary tooling validates and enforces many runtime and structural
rules, but it does not yet act as a completion guard for "documented boundary
item exists in code".

Current implementation note for caller policy:

- `SCB-CALLER-001` is the exact-symbol, exact-caller allowlist rule for
  boundary records
- it uses the same TOML inventory source as the rest of `sc-lint-boundary`
- it is a fail-only rule; there is no warning mode for unapproved callers
- `references.scope = "outside_owner_crate"` exempts owner-crate callers but
  still enforces the allowlist for external callers

That gap leaves room for two bad outcomes:

- documented boundary items that never land in code
- ad hoc exceptions for future-sprint items that should remain visible and
  traceable

## Core Requirement

Boundary lint should compare the structured boundary inventory against the code
graph and classify missing documented items by planning status:

- missing and planned in the current or a future sprint: `WARN`
- missing and unplanned: `ERROR`
- missing and overdue: `ERROR`

This model applies at item-key granularity, not just per boundary record. One
boundary may therefore contain:

- satisfied items
- warning-level planned gaps
- error-level unscheduled or overdue gaps

## Named Caller Policy

Named caller policy is separate from inventory-parity enforcement.

Use `[callers].approved` when a boundary symbol is intentionally callable from
outside the owner crate, but only by a curated set of named external callers.

Canonical example:

```toml
[callers]
approved = [
  { symbol = "restricted::run", callers = ["app::allowed::Facade"] },
]
```

Operational rules:

- `symbol` is an exact boundary-owned symbol path, such as `restricted::run`
- each `callers` entry is an exact caller identity validated at inventory load
- unknown fields, malformed paths, duplicate symbols, duplicate callers, and
  empty caller lists fail inventory loading immediately
- `SCB-CALLER-001` emits one failure per unapproved external caller identity

This differs from `references.forbidden`:

- use `references.forbidden` when the dependency edge should not exist at all
- use `[callers].approved` when the edge is allowed, but only from named
  external owners

## Package Dependency Policy

Package dependency policy is also separate from inventory-parity enforcement.

Use `[dependencies]` when a boundary record needs to constrain which workspace
packages may directly depend on the owner package or be depended on by the
owner package.

Canonical example:

```toml
[dependencies]
allowed_dependents = ["sc-lint"]
allowed_dependencies = ["sc-lint-directives", "sc-lint-schema"]
forbidden_edges = [
  "sc-lint-boundary -> sc-lint-attributes",
  "sc-lint-boundary -> sc-observability",
]
```

Operational rules:

- `allowed_dependencies` is the complete allowlist of direct outgoing
  workspace-member dependencies for the owner package
- `allowed_dependents` is the complete allowlist of direct incoming
  workspace-member dependents for the owner package
- `allowed_dependents = []` means no external workspace package may directly
  depend on that owner package
- each `forbidden_edges` row is one exact denied direct edge in
  `package-a -> package-b` form
- malformed edge strings, duplicate edges, duplicate package names, and unknown
  fields fail inventory loading immediately
- `SCB-DEPENDENCY-001` reports direct outgoing workspace edges not present in
  `allowed_dependencies`
- `SCB-DEPENDENCY-002` reports direct incoming workspace edges not present in
  `allowed_dependents`
- `SCB-DEPENDENCY-003` reports exact forbidden direct edges even when the same
  edge would otherwise pass owner/dependent allowlists

Scope limits for the first rule family:

- direct workspace-member edges only
- no transitive reachability enforcement yet
- no third-party crates.io or git dependency policy yet

This differs from manifest policy:

- use package dependency policy for architectural dependency seams between
  workspace packages
- use manifest policy for workspace-field inheritance and internal path
  dependency version alignment
- do not merge package dependency policy into the manifest-policy rule family

## Inventory-Parity Scope

Initial parity checks should verify that the following documented items resolve
in code:

- `implementation.module`
- `implementation.type`
- `public.facade`
- `composition.root`

Later extensions may include other required architecture module families once
they are represented in structured form.

For release `0.1.x`, repo-local automation/profile orchestration surfaces are
out of parity scope unless they are first promoted into explicit structured
boundary records. CLI contract types recorded under
`BOUNDARY-ScLintCli.composition.root.*` are in scope.

Reserved future analyzer crates may also exist in structured boundary records
before they are scheduled. They remain out of parity scope until a future
sprint assigns planned items for their implementation/public contract fields.

## Required Planning Inputs

The warning model is only valid when the planning metadata is complete enough to
evaluate mechanically.

Each planned item must include:

- stable item key
- owning boundary id
- scheduled sprint
- tracking id
- escalation condition

If any of those are missing or malformed, the linter should treat the item as
an error, not a warning.

The item key must be derived mechanically from the canonical boundary record
path, not invented as freeform prose. Recommended shape:

- `<boundary_id>.<section>.<field>[.<subfield>]`

Examples:

- `BOUNDARY-ScLintCli.public.facade`
- `BOUNDARY-ScLintCli.implementation.type`
- `BOUNDARY-ScLintCli.composition.root.CliError`

## Finding Classes

### Error

Use an error when:

- the documented item is missing from code
- and there is no structured planning mapping for that item

This is immediate architectural drift and should fail the lint.

### Warning

Use a warning when:

- the documented item is missing from code
- and it is explicitly scheduled for the current or a future sprint

This warning should remain visible in lint output and logs, not be silently
suppressed.

### Warning auto-escalation

Warnings must automatically escalate to errors when the planned delivery point
is older than the active sprint and the item still does not resolve.

This prevents:

- permanent warning debt
- stale plan references becoming de facto suppressions

### Current-sprint semantics

For release planning, `[planning].current_sprint` means:

- the currently active, in-progress sprint
- not the most recently completed sprint

That means a planned-but-missing item with:

- `scheduled_sprint == current_sprint`

is still warning-eligible while the sprint is active.

An item becomes overdue only when:

- `scheduled_sprint < current_sprint`

Repositories that want a harder interpretation must advance
`[planning].current_sprint` when the sprint closes rather than reusing the
closed sprint label for post-closeout validation.

## Structured Planning Mapping

Warning eligibility must come from structured data, not freeform prose.

Each planned-but-missing item should map to:

- item key
- scheduled sprint
- tracking id
- escalation condition

The mapping must be machine-readable so the linter can:

- decide warn vs error deterministically
- explain the reason for warning
- auto-escalate at the correct point

Recommended source:

- TOML-backed planning metadata checked into the repo

Not acceptable as the long-term source:

- prose-only notes
- inline comments in Markdown boundary docs
- freeform allowlists

## Recommended Data Shape

The enforcement model should assume TOML-backed boundary records and TOML-backed
planning metadata in:

- `boundaries/planning.toml`

Example direction:

```toml
[planning]
current_sprint = "A.14"

[planned_items."BOUNDARY-ScLintCli.implementation.type"]
scheduled_sprint = "A.14"
tracking_id = "SCB-CHANGE-1234"
expires_when = "sprint_before_current"
```

This shape is illustrative, not final. The `A.N` notation is only an example;
quarter-based labels such as `Q.3` or release-phase labels such as `R.7` are
equally valid if the repository defines a machine-parsable sprint ordering.

The important point is that:

- the item key is stable
- the scheduling metadata is structured
- the linter can evaluate it without parsing prose

Current implementation boundary:

- A.6 now provides the Rust-native TOML loader, schema validation, and
  duplicate handling foundation for this model
- planning-aware missing-item rule emission (`SCB-INVENTORY-001` through
  `SCB-INVENTORY-003`) remains the next enforcement stage on top of that
  loader foundation

## Sprint Evaluation Rule

The linter must have one deterministic source for "current sprint" when it
evaluates overdue warnings.

Recommended source:

- `boundaries/planning.toml`
- specifically `[planning].current_sprint`

The source must be explicit and testable. It must not rely on a human manually
interpreting the current sprint at review time.

If the current-sprint source is missing, malformed, or cannot be parsed, the
linter must classify planned-but-missing items as errors rather than warnings.

Sprint comparison must use parsed sprint ordering, not lexical string
comparison. For example, `A.10` must compare greater than `A.9`.

For release `0.1.x`, the supported escalation condition should be:

- `sprint_before_current`

Meaning:

- warn when `scheduled_sprint == current_sprint`
- error when `scheduled_sprint < current_sprint`

## Finding Content

Both warnings and errors should include:

- stable rule id
- exact missing item
- owning document / boundary record
- scheduled sprint, when present
- tracking id, when present

This makes the output useful to:

- QA
- planning reviewers
- future implementation sprints

Required rule families:

- `SCB-INVENTORY-001`
  - missing documented item with no valid planning mapping
- `SCB-INVENTORY-002`
  - missing documented item scheduled for the current or a future sprint
- `SCB-INVENTORY-003`
  - missing documented item whose planned sprint is before the current sprint

## Rule Behavior

Recommended behavior:

1. Parse the structured boundary inventory.
2. Build or load the code graph.
3. Resolve each required documented item into the graph.
4. If the item exists:
   - pass silently
5. If the item does not exist:
   - check structured planning metadata
   - classify as warning or error
6. If the item is planned but overdue:
   - promote warning to error

## Dual-Loader Behavior During TOML Migration

During the Markdown + TOML transition, the parity model should operate on the
shared internal boundary model, but the planning/enforcement feature itself
should still be TOML-first.

Default behavior should be:

- TOML planning metadata is authoritative
- `boundaries/planning.toml` is the default authoritative planning-metadata
  file
- duplicate boundary definitions across sources are errors unless explicitly in
  an equivalence-test migration mode
- duplicate item keys in the planning metadata are errors

The equivalence-test migration mode should be test-only and disabled in normal
developer lint runs and CI.

## Testing Requirements

At minimum, the implementation should ship with:

- positive test: documented item resolves in code
- warning test: documented item missing but scheduled in a future sprint
- overdue test: documented item missing and scheduled in current/past sprint
- error test: documented item missing with no planning mapping
- malformed-planning test:
  - missing sprint
  - missing tracking id
  - invalid item key
  - item key that points at no known boundary path
- malformed-current-sprint-source test
- sprint-ordering test:
  - `A.9` vs `A.10`
  - current sprint vs future sprint
- duplicate-planning-entry test
- mixed-boundary test where one record contains pass/warn/error items together
- dual-loader test where TOML and Markdown coexist without duplicate authority
- duplicate-source test where the same boundary id appears in both sources and
  fails
- equivalence-test-mode test showing duplicate-source migration checks are only
  legal in explicit migration fixtures, not normal lint mode

## Relationship To TOML Migration

This model should be implemented against TOML-backed boundary data.

The model is possible with Markdown compatibility during transition, but it
should not be designed around Markdown parsing as the long-term source.

New boundary-enforcement features in this area should be TOML-first.

## Recommendation

Implement the warn/error model as the next boundary-enforcement planning item,
with these constraints:

- structured mapping only
- no freeform warning exceptions
- overdue warnings must escalate automatically
- implementation should land on TOML-backed boundary data rather than extending
  Markdown-only parsing further
