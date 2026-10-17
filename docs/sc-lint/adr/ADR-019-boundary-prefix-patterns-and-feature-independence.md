# ADR-019 — Boundary prefix patterns and opt-in feature independence

Status: Accepted (Rand, 2026-10-16)

Related: ADR-004, ADR-011, ADR-017.
Motivating contribution: PR #603 (external), for a fork that keeps its own
crates independent of a frequently synced upstream workspace whose packages
share a `codex-` prefix.

## Context

`allowed_dependencies` and `allowed_dev_dependencies` (REQ-SCB-016) name every
permitted workspace package. A fork that mirrors a large upstream workspace
must list dozens of upstream packages by name and edit that list on every
sync.

Package dependency policy (`SCB-DEPENDENCY-001..003`) sees dependency edges
only. An owner can stay within its allowed edges and still couple itself to
another workspace package through Cargo features: setting `features` or
`default-features = false` on the dependency, or forwarding its own
`[features]` to it. No current rule reports that coupling.

## Decision

1. **Prefix patterns.** `allowed_dependencies`, `allowed_dev_dependencies` and
   `allowed_feature_dependencies` accept a prefix pattern: a non-empty prefix
   followed by exactly one trailing `*` (for example `codex-*`). The pattern
   matches canonical workspace package names only, not aliases. A pattern that
   matches no package is not an error. A bare `*`, a `*` anywhere else, more
   than one `*`, or any pattern in `allowed_dependents` fails inventory
   loading. `*` cannot occur in a Cargo package name, so a pattern is never
   ambiguous with a package; implementations keep patterns and exact names as
   distinct values rather than storing a pattern as a package name.

2. **`SCB-FEATURE-001`, opt-in per boundary record.** The rule runs for an
   owner only when its boundary record's `[dependencies]` section declares
   `allowed_feature_dependencies` (an empty list opts in with no exemptions).
   A record without the field is not checked, so no existing record changes
   result on upgrade. For an opted-in owner, the rule fails when, for a normal
   or build dependency (including target-specific ones) that resolves by path
   to a workspace package not matched by `allowed_feature_dependencies`:
   - the dependency sets `features` or `default-features = false`, including
     values the owner inherits through `workspace = true`; or
   - the owner's `[features]` forward to it: `"dep:x"`, `"x/feat"`,
     `"x?/feat"`, or a bare `"x"` that enables optional dependency `x` through
     its implicit feature.

   Dev-dependencies are not checked, matching how `SCB-DEPENDENCY-001` treats
   them separately. A registry or git dependency that shares a workspace
   member's name is ignored. The rule belongs to the `dependencies` rule
   family and runs under `--rule-filter dependencies`.

3. **Compatibility.** Both additions are additive: existing records keep their
   results. Older binaries reject a record that declares
   `allowed_feature_dependencies` (`deny_unknown_fields`), so a consumer
   adopting it pins a release that supports it. The release notes
   state this under Compatibility.

## Consequences

- A fork lists `codex-*` once instead of every upstream package.
- Feature coupling becomes enforceable for the records that ask for it, with
  no change for anyone else.
- Package-edge collection stays name-based for `SCB-DEPENDENCY-*`; making it
  path-based like `SCB-FEATURE-001` is follow-up work, not part of this ADR.

## Requirements

REQ-SCB-024 (prefix patterns) and REQ-SCB-025 (`SCB-FEATURE-001`) in
[`docs/sc-lint-boundary/requirements.md`](../../sc-lint-boundary/requirements.md).

*ADR-019 | sc-lint | 2026-10-17*
