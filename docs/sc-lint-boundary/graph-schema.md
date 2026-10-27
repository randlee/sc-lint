# `sc-lint` Graph Schema

This document records the current export contract for `sc-lint-boundary`.

## Versioning

- tool: `sc-lint-boundary`
- current schema version: `0.2.0`

Both graph JSON and findings JSON include:

- `tool`
- `version`
- `schema_version`

The schema version is the compatibility key for downstream graph consumers.

### Migration from 0.1.0

Version `0.2.0` reflects the trait-method identity correction in the graph
builder. Consumers must rebaseline stored graphs and findings that persist
`node_ids`; no legacy IDs or compatibility mapping are emitted.

The following IDs change:

- trait method IDs change from `<owner-id>::<method>` to
  `<trait-impl-id>::<method>`, so same-named methods from different trait
  implementations remain distinct;
- trait implementation IDs change when the trait has generic arguments or the
  implemented self type is generic or a reference, because those arguments are
  now retained in the implementation key.

The following IDs remain stable:

- path-owner IDs, including crate, module, type, and trait nodes;
- inherent method IDs (`<owner-id>::<method>`);
- non-generic trait implementation IDs and their other stable node metadata.

Downstream consumers should discard or migrate cached graph and finding
records, regenerate exports with `0.2.0`, and rebaseline any stored
`node_ids` or edge endpoints. Do not attempt to infer trait ownership from a
method-name suffix; use the implementation node and `contains`/`declares`/
`targets` edges.

## Graph Export Formats

Supported export formats:

- `json`
- `turtle`

Current CLI:

```text
sc-lint-boundary export-graph --root <repo> --format json
sc-lint-boundary export-graph --root <repo> --format turtle
```

## Node Model

Current node kinds:

- `crate`
- `module`
- `type`
- `variant`
- `field`
- `trait`
- `trait_ref`
- `function`
- `method`
- `impl`

Current shared node fields:

- `id`
- `kind`
- `label`
- `visibility`
- `package`
- `target`
- `manifest_path`
- `source_path`
- `module_path`
- `impl_kind`
- `impl_trait`
- `attributes`

Notes:

- `trait_ref` is used when an impl references a trait path that does not map to
  a local trait node already present in the graph.
- `visibility` is a normalized string:
  - `private`
  - `public`
  - `crate`
  - `restricted`

### Method identity

Inherent methods retain `<owner>::<method>` IDs. Trait methods use
`<impl-id>::<method>`, so traits with the same method name do not share a node
with each other or with inherent methods. Implementation keys retain trait
arguments and nontrivial self types, including references and generic arguments.
Consumers should use node metadata and `contains`/`declares`/`targets` edges
rather than infer trait ownership from a method-name suffix.

Qualified trait calls resolve to the corresponding implementation method.
Unqualified calls prefer an existing inherent target; otherwise the analyzer
uses the current trait implementation or a unique trait-method candidate.
Ambiguous candidates remain unresolved instead of being merged. An
enum-variant expression (`Self::A`, `E::A`) resolves to the variant node
`<owner>::variant::<name>`, never to a method, and variant edges do not enter
the owner graph. Cycle rules still operate on the underlying type owner;
distinct methods do not imply a waiver of owner-level cycle diagnostics,
except for delegation edges (see SCB-CYCLE-002/003 below).
(Amended 2026-10-10, Accepted, #204.)

Method-call edges: a method-call expression creates a reference only when its
receiver is exactly `self`, looking through parentheses, `&` and `*`. The
reference path is `Owner::method` and resolves like `Self::method(self)` per
the preceding paragraph. A receiver that is a field, local or call result
creates no edge (there is no type inference), and calls inside macro
invocations are not seen. (Amended phase L, ADR-020 decision 11.)

## Edge Model

Current edge kinds:

- `contains`
- `declares`
- `targets`
- `implements`
- `references`
- `references_type`
- `references_expr`

Notes:

- `references` is the compatibility aggregate edge.
- `references_type` and `references_expr` are the preferred semantic edges for
  new rule work.
- `targets` connects an `impl` node to the type it implements for.
- `implements` connects an `impl` node to a `trait` or `trait_ref` node.

## Findings Model

Current finding fields:

- `rule_id`
- `kind`
- `message`
- `owner_ids`
- `node_ids`

Current implemented rule ids:

- `SCB-CYCLE-001`
- `SCB-CYCLE-002`
- `SCB-CYCLE-003`
- `SCB-BOUNDARY-001`
- `SCB-BOUNDARY-002`
- `SCB-BOUNDARY-003`
- `PORT-001`
- `PORT-002`
- `PORT-003`

`SCB-CYCLE-002` and `SCB-CYCLE-003` exclude a delegation edge. A delegation
edge is a method-to-method expression edge between two distinct graph nodes
whose callee cannot reach the caller through the owner's expression edges.
Owner reference edges record the callee node. The callee's name and
visibility do not matter: a trait-impl method calling a same-named inherent
method is a call between two distinct nodes, and a public method calling a
different public method is not recursion. A method that resolves to its own
node stays a self-loop, and mutual recursion between methods is not a
delegation edge. (Amended 2026-10-10, Accepted, #204; delegation conditions
replaced by ADR-020 decision 11, Accepted.)

Exclusion of path-less findings: `SCB-CYCLE-002` and `SCB-CYCLE-003` findings
carry no path. For `exclude_paths` and `exclude_paths_by_rule` the analyzer
resolves the repo-relative source files of the finding's `node_ids`, and the
finding is excluded when any of those files matches a global or per-rule
pattern. The `Finding` schema is unchanged. (ADR-020 decision 5.)

## Source-Level Allowances

Current implemented source-level allowances:

- `boundary.allow("cycle.type_method_self_loop")`
- `boundary.allow("cycle.recursive_value_container")`

`cycle.recursive_value_container` is intended for deliberate recursive
container/value models where every owner participating in the recursive cycle
opts in explicitly.

## Default Trait Policy

The analyzer includes an embedded default policy file:

- `crates/sc-lint-boundary/config/defaults.toml`

Current trait self-loop filtering is configured through:

- `trait_self_loop.ignored_trait_paths`
- `trait_self_loop.ignored_trait_names`

This keeps common ecosystem trait families configurable without baking
repo-specific names into analyzer code.

## Stability Notes

Stable enough to build against now:

- `tool`
- `schema_version`
- node/edge top-level field names
- current rule ids

Still expected to evolve:

- exact node kind inventory
- exact edge kind inventory
- which advisory findings are emitted by default
- Turtle vocabulary expansion
