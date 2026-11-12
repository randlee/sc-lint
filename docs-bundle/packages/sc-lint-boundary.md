# sc-lint-boundary

## Purpose and ownership

`sc-lint-boundary` is the AST-sensitive Rust analyzer for crate boundaries,
dependency policy, module ownership, and boundary directives. It ships both a
library and the `sc-lint-boundary` backend binary.

## Intended users

Workspace maintainers use it through `sc-lint lint sc-boundary`; direct binary
invocation is reserved for product development and release diagnostics.

## Configuration and inputs

The analyzer reads the workspace Cargo metadata and canonical boundary TOML
inventory. `sc-lint-attributes` and `sc-lint-directives` provide source policy;
the package's boundary requirements describe inventory fields and exceptions.
Optional manifest dependency policies are configured in the boundary analyzer
table through `manifest_dependency_allowlists`,
`global_dependency_ownership`, and `manifest_section_rules`; see
[configuration](../configuration.md) for their syntax.

`manifest_dependency_allowlists` permits named dependencies for an owner
manifest, `global_dependency_ownership` limits where a dependency may appear
in manifests and source, and `manifest_section_rules` restricts a dependency
to named manifest sections. These are optional policy families; an absent
family adds no checks. The families are released `sc-lint` consumer
configuration. atm-core keeps parity only for record-lock comparison of
`allowed_dependencies` and `allowed_dev_dependencies`; the four-case oracle is
in `crates/sc-lint-boundary/tests/issue136.rs`.

## Rules

Rule filters, `exclude_rules`, and `fail_rules` accept these IDs and the
family names `cycles`, `boundaries`, `internal-only`, `forbid-external-impls`,
`dependencies`, and `manifests`. Every rule fails by default except
`SCB-CYCLE-002` and `SCB-CYCLE-003`, which report without failing.

| Rule | Family | Reports |
| --- | --- | --- |
| `SCB-CYCLE-001` | cycles | An architectural cycle across two or more owners (modules, types, or traits). |
| `SCB-CYCLE-002` | cycles | A type or method self-loop within one owner. |
| `SCB-CYCLE-003` | cycles | A trait-impl self-loop within one owner. |
| `SCB-BOUNDARY-001` | boundaries, internal-only | A `boundary.internal_only` item that is externally visible. |
| `SCB-BOUNDARY-002` | boundaries, internal-only | A `boundary.internal_only` item referenced from outside its module. |
| `SCB-BOUNDARY-003` | boundaries, forbid-external-impls | An impl of a `boundary.forbid_external_impls` trait from another module. |
| `SCB-BOUNDARY-004` | boundaries, dependencies | An invalid boundary inventory record or field. |
| `SCB-BOUNDARY-005` | boundaries | Checks `[implementation].visibility` against the named source type and, when present, `[implementation].constructor` against its inherent `fn new`; reports visibility wider than declared or an item not found. Finding kind: `declared_visibility_mismatch`. |
| `SCB-CALLER-001` | boundaries | A restricted symbol called by a caller not on its approved-caller list. |
| `SCB-DEPENDENCY-001` | dependencies | A direct workspace-package dependency the owner's record does not allow. |
| `SCB-DEPENDENCY-002` | dependencies | A direct dependent the target's record does not allow. |
| `SCB-DEPENDENCY-003` | dependencies | A present edge listed in `forbidden_edges`. |
| `SCB-DEPENDENCY-004` | dependencies | A stale incoming permission with no live edge. |
| `SCB-FEATURE-001` | dependencies | Use of another package's features not listed in `allowed_feature_dependencies`. |
| `SCB-MANIFEST-001` | manifests | A missing required workspace-inherited package field. |
| `SCB-MANIFEST-002` | manifests | An internal path dependency whose version does not match its target. |
| `SCB-MANIFEST-003` | manifests | A dependency not on its owner's configured allowlist. |
| `SCB-MANIFEST-004` | manifests | A dependency declared or used outside its configured owner. |
| `SCB-MANIFEST-005` | manifests | A dependency in a manifest section its rule does not allow. |
| `SCB-MANIFEST-006` | manifests | Stale or inconsistent manifest policy, including record locks. |

## Moved rules: old to new

Portability and runtime rules no longer run from `sc-lint-boundary`. Its
`--rule-filter` rejects them with a usage error (exit 2), and its
configuration reads only boundary keys. Use the owning tool instead:

| Old rule (bundled) | New tool | Runnable selector | Configuration table |
| --- | --- | --- | --- |
| `PORT-001` to `PORT-005` | `sc-lint-portability` | `sc-lint lint sc-portability --rule-filter PORT-001` (any `PORT-*` ID) | `[tool.sc-lint.analyzers.portability]` |
| `SCB-RUNTIME-001`, `SCB-RUNTIME-002` | `sc-lint-runtime` | `sc-lint lint sc-runtime --rule-filter SCB-RUNTIME-001` (any `SCB-RUNTIME-00N` ID) | `[tool.sc-lint.analyzers.runtime]` |

The portability and runtime selectors take rule IDs only; `sc-lint-runtime`
rejects families. Their `exclude_rules`, `fail_rules`, `exclude_paths` and
`exclude_paths_by_rule` keys use the same names as the boundary table, and
`portability` also takes `unix_path_prefixes`, `config_home_env` and
`platform_adapters`. See [configuration](../configuration.md#analyzer-tables)
for the complete key list.

## Boundary inventory schema and visibility

Boundary records have no `[enforcement]` section. It was removed from the
record schema, and declaring it makes inventory loading fail with a diagnostic
that identifies the record file and requires deleting the section.

For `SCB-BOUNDARY-005`, the checked record fields are
`[implementation].visibility` and, when present,
`[implementation].constructor`. The visibility value is compared with the
named source type in `[implementation].module`; a type wider than declared or
one that cannot be found is reported. `trait_only` reports a public
implementing type. The constructor value is compared with that type's inherent
`fn new`; for example, `private` does not permit a public constructor.
`visibility = "public"` and `constructor = "none"` skip checks for those
items. The finding identifies the record file, item, and declared and actual
visibility.

## Recovering inventory and stale-policy findings

For `SCB-BOUNDARY-004`, use the reported record and field to correct the
inventory value or structure. For `SCB-DEPENDENCY-004`, remove a stale incoming
permission or restore the intended direct workspace-package edge. For
`SCB-MANIFEST-006`, use the owner and record path in the finding to remove a
duplicate or inactive allowlist, repair its record lock, or align the
allowlist with the owner's actual dependency sections. These are declaration
and inventory corrections; do not suppress the finding by deleting unrelated
metadata.

## Commands and output

```sh
sc-lint lint sc-boundary
```

Findings identify rule, owner, path, and remediation context. The normalized
CLI envelope is suitable for CI. `sc-lint view graph` is a reserved command
surface and currently returns `CLI.CAPABILITY_ERROR`; it is not an available
architecture-inspection view.

## CI and failures

Run the boundary profile as a required step. Invalid inventory, forbidden
edges, and unsuppressed cycles fail the profile. Check the CLI's structured
details before changing policy; do not silence a finding by deleting metadata.

## Dependency and manifest policies

Package edge policy is optional per inventory record. A record's
`allowed_dependencies` is its outgoing workspace-package allowlist;
`allowed_dev_dependencies` allows additional dev-only edges. `forbidden_edges`
denies exact direct edges. Incoming `allowed_dependents` permissions are
combined by canonical owner package: permissions from separate records union
together. For dev edges, each record with an explicit
`allowed_dev_dependents` contributes that list; when the field is omitted, its
`allowed_dependents` list also grants dev permission. An explicit empty
dev-allowlist grants none from that record but does not revoke another record's
grant.

For example, in a neutral fixture the `sample-api` record can allow only
`sample-app` as an incoming dependent. A direct edge from `sample-cli` then
reports `SCB-DEPENDENCY-002`; adding `sample-cli` to a second record for the
same canonical owner grants it because the incoming lists combine. A separate
record with an explicit empty dev list does not remove that grant.

To probe both an unexpected runtime dependent and a stale incoming permission
in a two-fault fixture, select both rule IDs explicitly:

```sh
sc-lint lint sc-boundary --rule-filter SCB-DEPENDENCY-002,SCB-DEPENDENCY-004 --json
```

The package-edge rules can be selected with `--rule-filter dependencies`;
manifest rules can be selected with `--rule-filter manifests`. The complete
analyzer runs both. `--rule-filter` accepts rule IDs or families and selects
from the full rule set; it takes precedence over `exclude_rules`, while path
exclusions still apply and `fail_rules` still determines the verdict. The
`exclude_paths_by_rule` map takes exact rule IDs and drops only that rule's
findings at matching paths. For example, excluding one manifest rule does not
suppress a different rule reported for the same manifest. `SCB-CYCLE-001`,
`-002` and `-003` findings carry no path; they are excluded when any source
file behind their nodes matches, so excluding `SCB-CYCLE-001` on one file
removes only the cycles built from that file. See
[configuration](../configuration.md) for the complete filter contract.

`SCB-DEPENDENCY-004` identifies a stale incoming permission. In a neutral
fixture, suppose two records describe the canonical owner `sample-api` and one
allows `sample-cli`, but no live edge of the permission's category points from
`sample-cli` to `sample-api`. The analyzer reports the unused owner/dependent
permission once, even if multiple records supplied it. The finding belongs to
`dependencies` and fails by default. Remove the stale permission or restore the
intended direct edge; package-edge checks cover direct workspace members, not
transitive or external-crate reachability.
Records with non-active statuses `planned`, `retired`, or `reserved_future` are
skipped; their dependency policies do not produce findings.

Manifest checks run on the complete analyzer and with
`--rule-filter manifests`. `SCB-MANIFEST-001` checks required workspace-inherited
package fields and `-002` checks internal path-dependency version alignment.
Configured manifest policy reports `-003` for disallowed configured
dependencies, `-004` for ownership violations, `-005` for section violations,
and `-006` for stale or inconsistent policy (including record-lock and active
owner coverage defects). These policy findings fail by default. Boundary
inventory semantic defects are reported as `SCB-BOUNDARY-004` with record/field
context; malformed syntax, schema/layout, and I/O errors remain fatal. A
combined-invalid inventory can therefore report each semantic defect as a
finding rather than stopping at the first one. For example, if a neutral
`sample-api` fixture has both a malformed semantic module path and an invalid
implementation type, both defects are reported with their record and field
context. A local composition-root symbol such as `run` remains local and is
preserved; only a resolved external crate-qualified root such as
`sample_client::Api` participates in dependent-policy validation. An unresolved
workspace package name also fails instead of being silently skipped.
For an external composition root, a unique library-target alias resolves to
its canonical Cargo package before comparison with `allowed_dependents`; the
policy may use that same resolvable alias.

Newer optional keys remain compatible with existing consumers: omitted
`exclude_paths_by_rule` and `platform_adapters` retain prior behavior, and an
absent optional manifest-policy family adds no manifest checks. Existing
inventory permission fields keep their established semantics. When a policy
record is present, an explicitly empty allowlist permits no entries; an empty
dependency list is restrictive, not an instruction to infer permissions. See
[configuration](../configuration.md) for the public keys and filters.

These optional records live in the boundary analyzer table, not in Cargo
manifests. This neutral example allows two direct package dependencies for one
owner, reserves a dependency for one manifest/source area, and permits a tool
only in the declared development sections:

```toml
[[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
owner_manifest_path = "crates/store/Cargo.toml"
allowed_dependencies = ["core-api", "db-driver"]

[[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
dependency = "db-driver"
allowed_manifest_paths = ["crates/store/Cargo.toml"]
allowed_source_roots = ["crates/store/src"]
manifest_message = "Only store may depend on db-driver."
source_message = "Only store source may import db-driver."

[[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
owner_manifest_path = "crates/app/Cargo.toml"
dependency_package = "fixture-kit"
allowed_sections = ["dev-dependencies", "target.cfg(unix).dev-dependencies"]
message = "fixture-kit belongs in declared test sections."
```

`allowed_source_roots` lists every source root that may use the owned crate;
no root is implied. An owner's integration tests live outside its `src`, so
allow them by listing the owner's `tests/` root as well:

```toml
[[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
dependency = "db-driver"
allowed_manifest_paths = ["crates/store/Cargo.toml"]
allowed_source_roots = ["crates/store/src", "crates/store/tests"]
manifest_message = "Only store may depend on db-driver."
source_message = "Only store and its integration tests may import db-driver."
```

A source file outside those roots reports `SCB-MANIFEST-004` when it uses the
owned crate: a path whose first segment is the crate (`db_driver::Client`,
`::db_driver::Client`, `use db_driver;`) or `extern crate db_driver`, in any
position (use, type, expression, pattern, struct literal, trait bound, impl
header, qualified path or attribute). Paths starting with `self`, `crate` or
`super` are never uses, and a bare `db_driver::` path is a local module path
when the same module declares `mod db_driver`. Comments, doc comments and
string literals are not uses. Attribute argument lists such as
`#[derive(db_driver::Row)]` are checked as paths. Macro token bodies keep a
textual match on `db_driver::`, `use db_driver` and `extern crate db_driver`
after dropping paths that start with `self`, `crate` or `super`; a whole file
that fails to parse keeps the textual match unchanged.

The manifest allowlist matches the dependency's Cargo package identity,
including renamed dependency entries. Ownership checks consider both manifest
declarations and source references beneath the allowed roots. Section strings
match exactly, including target qualifiers. A present empty allowed list
permits none. The records' messages appear with offending paths in findings.

### Canonical package identity

When resolving a package name from inventory or an edge, the analyzer checks
Cargo package name first, then a unique workspace-member directory, then a
unique library target name. If none resolves, it reports an unknown workspace
package reference rather than silently skipping the record. Prefer Cargo
package names in new inventory; directory and library-name matches are
compatibility fallbacks. Once resolved, policy comparisons use the canonical
Cargo package identity, including when a manifest dependency uses a rename.

The companion portability analyzer also uses the shared per-rule path filter.
When two platform files implement the same private adapter under different
module names, a reviewed `platform_adapters` pair identifies those files and a
reason; only the paired modules and their enclosed private implementation get
that treatment. For example, a fixture may pair `src/platform/unix.rs` with
`src/platform/windows.rs` while leaving a separately Unix-gated public item
subject to ordinary parity checks. See the canonical
[portability guide](./sc-lint-portability.md) and
[configuration guide](../configuration.md) for the accepted shape and filter
behavior.

## Related packages

Policy syntax comes from [attributes](./sc-lint-attributes.md) and
[directives](./sc-lint-directives.md); machine output uses
[schema](./sc-lint-schema.md). Pair boundary checks with
[portability](./sc-lint-portability.md) and [runtime](./sc-lint-runtime.md).
