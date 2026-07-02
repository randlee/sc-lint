# `sc-lint` Requirements

These requirements define the planned behavior for the boundary-definition,
package dependency policy, and inventory-parity work in the `sc-lint` tool
family.

## Canonical Boundary Source

- `REQ-SCB-001`
  Canonical machine-readable boundary definitions must live in standalone TOML
  files under the `boundaries/` directory.

## Dual-Loader Transition

- `REQ-SCB-002`
  During migration, the loader must accept both Markdown-embedded boundary
  records and TOML boundary records while mapping both sources into one shared
  internal boundary model.

- `REQ-SCB-003`
  During the dual-loader phase, duplicate `boundary_id` authority across
  sources is a hard error by default.

- `REQ-SCB-004`
  During the dual-loader phase, conflicting definitions for the same
  `boundary_id` across Markdown and TOML are a hard error.

- `REQ-SCB-005`
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
  owner record must fail with `SCB-DEPENDENCY-001`.

- `REQ-SCB-017`
  A direct workspace package dependency from workspace package `Y` to owner
  package `X` must fail with `SCB-DEPENDENCY-002` when `Y` is not listed in
  `dependencies.allowed_dependents` for the owner record. An empty
  `allowed_dependents` list means no external workspace package may directly
  depend on that owner package.

- `REQ-SCB-018`
  A direct workspace package dependency that matches an exact
  `dependencies.forbidden_edges` entry must fail with `SCB-DEPENDENCY-003`
  regardless of whether the same edge would otherwise pass owner/dependent
  allowlists.

- `REQ-SCB-019`
  The first package dependency policy implementation scope is direct
  workspace-member edges only. Transitive reachability and non-workspace
  dependencies are out of scope until a later scheduled sprint adds them
  explicitly.

- `REQ-SCB-020`
  Package dependency policy entries must be validated at inventory load.
  Malformed forbidden-edge strings, duplicate forbidden edges, duplicate
  package names inside one dependency-policy record, and unknown fields under
  `[dependencies]` are hard errors.

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
  The default planning metadata source for inventory parity is
  `boundaries/planning.toml`.

- `REQ-SCB-013`
  `boundaries/planning.toml` must define `[planning].current_sprint`, and
  current-sprint parsing failure must cause planned-but-missing items to fail
  rather than warn.

- `REQ-SCB-014`
  Sprint comparison for inventory parity must use parsed ordering, not lexical
  string comparison.
