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
sc-lint view graph
```

Findings identify rule, owner, path, and remediation context. The normalized
CLI envelope is suitable for CI; graph views are intended for architecture
inspection.

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
suppress a different rule reported for the same manifest. See
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
