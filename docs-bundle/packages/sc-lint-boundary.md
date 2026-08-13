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

## Related packages

Policy syntax comes from [attributes](./sc-lint-attributes.md) and
[directives](./sc-lint-directives.md); machine output uses
[schema](./sc-lint-schema.md). Pair boundary checks with
[portability](./sc-lint-portability.md) and [runtime](./sc-lint-runtime.md).
