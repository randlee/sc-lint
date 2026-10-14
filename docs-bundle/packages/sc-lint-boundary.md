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

Manifest checks run on the complete analyzer and with
`--rule-filter manifests`. `SCB-MANIFEST-001` checks required workspace-inherited
package fields and `-002` checks internal path-dependency version alignment.
Configured manifest policy reports `-003` for disallowed configured
dependencies, `-004` for ownership violations, `-005` for section violations,
and `-006` for stale or inconsistent policy (including record-lock and active
owner coverage defects). These policy findings fail by default. Boundary
inventory semantic defects are reported as `SCB-BOUNDARY-004` with record/field
context; malformed syntax, schema/layout, and I/O errors remain fatal. An
unresolved workspace package name also fails instead of being silently skipped.
Optional policy configuration is inactive when absent, while a present empty
allowlist permits no entries. See [configuration](../configuration.md) for
rule and path filters.

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

When resolving a package name from inventory or an edge, the analyzer checks
Cargo package name first, then a unique workspace-member directory, then a
unique library target name. If none resolves, it reports an unknown workspace
package reference rather than silently skipping the record. Prefer Cargo
package names in new inventory; directory and library-name matches are
compatibility fallbacks.

## Related packages

Policy syntax comes from [attributes](./sc-lint-attributes.md) and
[directives](./sc-lint-directives.md); machine output uses
[schema](./sc-lint-schema.md). Pair boundary checks with
[portability](./sc-lint-portability.md) and [runtime](./sc-lint-runtime.md).
