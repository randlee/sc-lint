# `sc-lint` Requirements

These requirements define the planned behavior for the boundary-definition,
package dependency policy, and inventory-parity work in the `sc-lint` tool
family.

## Canonical Boundary Source

- `REQ-SCB-001`
  Canonical machine-readable boundary definitions must live in standalone TOML
  files under the `boundaries/` directory.

## Dual-Loader Transition

Amendment (phase L, #586, Accepted — Rand, 2026-10-26): the TOML
transition is complete and the loader is TOML-only (`inventory/mod.rs`). The
Markdown-embedded loader and its cross-source checks below are retired; they
are kept as history. `REQ-SCB-002` to `REQ-SCB-005` are marked retired and the
item-key parity wording of `REQ-SCB-007` applies to TOML records only.
Plan authority: `lint-l-8`.

- `REQ-SCB-002` **(retired)**
  During migration, the loader must accept both Markdown-embedded boundary
  records and TOML boundary records while mapping both sources into one shared
  internal boundary model.

- `REQ-SCB-003` **(retired)**
  During the dual-loader phase, duplicate `boundary_id` authority across
  sources is a hard error by default.

- `REQ-SCB-004` **(retired)**
  During the dual-loader phase, conflicting definitions for the same
  `boundary_id` across Markdown and TOML are a hard error.

- `REQ-SCB-005` **(retired)**
  Any migration equivalence mode that permits duplicate-source fixtures must be
  test-only and disabled in default developer runs and CI.

## TOML-First Evolution

- `REQ-SCB-006`
  Once TOML loading exists, new boundary-enforcement features that depend on
  boundary metadata must be implemented against TOML-backed data first.

## Package Dependency Policy

- `REQ-SCB-015`
  Boundary inventory package dependency policy must be enforced from canonical
  TOML boundary records plus workspace Cargo metadata, not from prose-only
  review notes or ad hoc manifest scraping.

- `REQ-SCB-016`
  A direct workspace package dependency from owner package `X` to workspace
  package `Y` that is not listed in `dependencies.allowed_dependencies` for the
  owner record must fail with `SCB-DEPENDENCY-001`. A dev-dependency edge is
  allowed when `Y` is listed in `dependencies.allowed_dependencies` or
  `dependencies.allowed_dev_dependencies`; an edge that is both a normal and a
  dev dependency is checked as a normal edge.
  (Amended 2026-10-10, Accepted, #203.)

- `REQ-SCB-017`
  A direct workspace package dependency from workspace package `Y` to owner
  package `X` must fail with `SCB-DEPENDENCY-002` when the canonical dependent
  is absent from the owner's effective incoming allowlist. Aggregate permissions
  per canonical owner package: runtime permissions union every record's
  `dependencies.allowed_dependents` (R_i); dev-only permissions union each
  supplied `dependencies.allowed_dev_dependents` (D_i), falling back to R_i
  only for records omitting D_i. Explicit D_i=[] grants nothing from that
  record and does not revoke another record's grant. An empty effective union
  allows no dependent in that edge category; normal/build dominates mixed
  normal/dev edges. Staleness considers only categories to which an entry
  contributes, is deduplicated by canonical owner/dependent, and retains the
  upstream active-record gate. `SCB-DEPENDENCY-004` reports a stale
  canonical owner/dependent permission once, defaults to failure, and belongs
  to the `dependencies` filter family.
  (Amended 2026-10-12, Accepted,
  [ADR-017, change 2](../sc-lint/adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)

- `REQ-SCB-018`
  A direct workspace package dependency that matches an exact
  `dependencies.forbidden_edges` entry must fail with `SCB-DEPENDENCY-003`
  regardless of whether the same edge would otherwise pass owner/dependent
  allowlists.

- `REQ-SCB-019`
  The first package dependency policy implementation scope is direct
  workspace-member edges only, including direct edges declared under
  `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and
  target-specific dependency sections when they name another current workspace
  member. Transitive reachability and third-party crates.io or git
  dependencies outside the workspace-member set are out of scope until a later
  scheduled sprint adds them explicitly. `[dev-dependencies]` edges may also be
  allowed by `dependencies.allowed_dev_dependencies` (REQ-SCB-016); normal and
  build edges are unchanged. This package-edge scope does not exclude
  explicitly configured manifest policy, including third-party dependencies,
  under `--rule manifests`; that policy remains distinct from package-edge
  enforcement under `--rule dependencies`. Manifest findings use
  `SCB-MANIFEST-003` for a disallowed configured dependency, `-004` for a
  configured ownership violation, `-005` for a configured section violation,
  and `-006` for stale or inconsistent policy, including record-lock and
  active-owner coverage defects. All default to failure and belong to the
  `manifests` filter family; existing `-001`/`-002` remain unchanged.
  (Amended 2026-10-10, Accepted, #203; amended 2026-10-12, Accepted,
  [ADR-017, change 2](../sc-lint/adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)

- `REQ-SCB-020`
  Package dependency policy entries must be validated at inventory load.
  Malformed forbidden-edge strings, duplicate forbidden edges, duplicate
  package names inside one dependency-policy record, and unknown fields under
  `[dependencies]` are hard errors. A boundary record may carry one opaque
  `extensions` table (`[extensions.<namespace>]`), which is parsed as TOML and
  ignored; every other unknown record field is still a hard error. The optional
  `allowed_dev_dependents` field follows REQ-SCB-017. New semantic defects
  covered by ADR-017 are reported individually after parsing as structured
  findings with record/field context; one defect must not hide another.
  Existing syntax, schema, layout and I/O failures remain fatal, and public
  loader signatures remain unchanged. Preserve local composition-root symbols;
  only resolved external crate-qualified roots undergo dependent-policy checks.
  A resolved external root must name a package listed in that record's
  `dependencies.allowed_dependents` (`SCB-BOUNDARY-004`, kind
  `unapproved_composition_root`).
  `SCB-BOUNDARY-004` reports each semantic inventory defect, defaults to
  failure, and remains selected by both `boundaries` and `dependencies` filter
  families; structural loader failures remain fatal.
  (Amended 2026-10-10, Accepted, #203; amended 2026-10-12, Accepted,
  [ADR-017, change 3](../sc-lint/adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts); amended 2026-10-27, Accepted, ADR-020 clarification 4.1.)

- `REQ-SCB-021`
  Package dependency policy is a boundary-inventory rule family owned by
  `sc-lint-boundary`, but it remains distinct from manifest workspace/version
  hygiene. Package-edge enforcement must not be collapsed into the
  manifest-policy rule family.

## Inventory-Parity Enforcement

- `REQ-SCB-007`
  Inventory-parity checks must compare structured boundary requirements against
  the code graph at item-key granularity.

- `REQ-SCB-008`
  A missing documented item with no valid planning mapping must fail with
  `SCB-INVENTORY-001`.

- `REQ-SCB-009`
  A missing documented item scheduled in a future sprint may warn with
  `SCB-INVENTORY-002`.

- `REQ-SCB-010`
  A missing documented item whose scheduled sprint is current or past must fail
  with `SCB-INVENTORY-003`.

- `REQ-SCB-011`
  Inventory-parity warning eligibility must come only from structured planning
  metadata, not prose, comments, or freeform allowlists.

- `REQ-SCB-012`
  `boundaries/planning.toml` is the sole authoritative planning-metadata
  source for inventory parity and must be a regular file. Any other entry
  named `planning.toml` is an error, including nested entries, directories,
  symlinks, and case variants.

- `REQ-SCB-013`
  When `boundaries/` exists, `boundaries/planning.toml` must define
  `[planning].current_sprint`; when `boundaries/` is absent, the loader returns
  an empty inventory and no planning metadata is required. Current-sprint
  parsing failure must cause planned-but-missing items to fail rather than
  warn.

- `REQ-SCB-022`
  Every boundary record's `owner_crate_path` must equal the lib target name
  that `cargo metadata` reports for `owner_package`.
  (Amended 2026-10-10, Accepted, #203.)

- `REQ-SCB-023`
  A boundary's `[public]` section must define exactly one non-empty `facade` or
  `trait` value. Preserve these structural load errors; report additional
  semantic defects under ADR-017 individually after parsing with record/field
  context, including all defects in a combined-invalid fixture.
  (Extended 2026-10-12, Accepted,
  [ADR-017, change 3](../sc-lint/adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)

- `REQ-SCB-024`
  `allowed_dependencies`, `allowed_dev_dependencies` and
  `allowed_feature_dependencies` may contain a prefix pattern: a non-empty
  prefix followed by exactly one trailing `*` (for example `codex-*`), matching
  canonical workspace package names, not aliases. A pattern matching no
  package is not an error. A bare `*`, a `*` elsewhere, more than one `*`, or
  any pattern in `allowed_dependents` fails inventory loading.
  (Accepted 2026-10-16,
  [ADR-019, decision 1](../sc-lint/adr/ADR-019-boundary-prefix-patterns-and-feature-independence.md#decision).)

- `REQ-SCB-025`
  `SCB-FEATURE-001` runs for an owner only when its boundary record declares
  `[dependencies].allowed_feature_dependencies`; an empty list opts in with no
  exemptions, and a record without the field is not checked. For an opted-in
  owner it fails when a normal or build dependency (including target-specific
  ones) that resolves by path to a workspace package not matched by
  `allowed_feature_dependencies` sets `features` or `default-features = false`
  (including values inherited through `workspace = true`), or when the owner's
  `[features]` forward to it through `dep:x`, `x/feat`, `x?/feat`, or a bare
  `x` enabling optional dependency `x`. Dev-dependencies and same-named
  registry or git dependencies are not checked.
  (Accepted 2026-10-16,
  [ADR-019, decision 2](../sc-lint/adr/ADR-019-boundary-prefix-patterns-and-feature-independence.md#decision).)

- `REQ-SCB-014`
  Sprint comparison for inventory parity must use parsed ordering, not lexical
  string comparison.

## Stable consumer interfaces

Rule IDs and this analyzer's JSON output are stable consumer contracts
(`REQ-PRODUCT-002`, [ADR-011](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)).
