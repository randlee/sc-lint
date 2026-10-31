# `sc-boundary` User Guide

`sc-boundary` is the shipped boundary-analysis tool for AST-sensitive Rust
ownership, boundary, and manifest-policy checks.

## Purpose

Use `sc-boundary` to check:

- architectural cycles
- source-level boundary declarations such as `boundary.internal_only`
- external impl restrictions such as `boundary.forbid_external_impls`
- direct workspace package dependency policy from boundary inventory
- Rust-native manifest-policy checks added in A.7

Rule IDs include:

- `SCB-CYCLE-001`
- `SCB-CYCLE-002`
- `SCB-CYCLE-003`
- `SCB-BOUNDARY-001`
- `SCB-BOUNDARY-002`
- `SCB-BOUNDARY-003`
- `SCB-CALLER-001`
- `SCB-DEPENDENCY-001`
- `SCB-DEPENDENCY-002`
- `SCB-DEPENDENCY-003`
- `SCB-DEPENDENCY-004`
- `SCB-FEATURE-001`
- `SCB-MANIFEST-001`
- `SCB-MANIFEST-002`
- `SCB-MANIFEST-003`
- `SCB-MANIFEST-004`
- `SCB-MANIFEST-005`
- `SCB-MANIFEST-006`
- `SCB-BOUNDARY-004`

Rule filters accept individual rule IDs and these kebab-case family names:

- `cycles`
- `boundaries`
- `internal-only`
- `forbid-external-impls`
- `dependencies`
- `manifests`

## Invocation

Primary product surface:

```bash
sc-lint lint sc-boundary
sc-lint --json lint sc-boundary
```

Repo-local convenience path:

```bash
just lint sc-boundary
```

Backend-local path:

```bash
cargo run -p sc-lint-boundary -- analyze --root . --format text
cargo run -p sc-lint-boundary -- analyze --root . --format json
```

Backend-local filtered analysis:

```bash
cargo run -p sc-lint-boundary -- analyze --root . --rule-filter cycles --format text
cargo run -p sc-lint-boundary -- analyze --root . --rule-filter SCB-DEPENDENCY-001 --format text
cargo run -p sc-lint-boundary -- analyze --root . --config sc-lint.toml --rule-filter manifests --format text
```

## Analyzer Configuration

`--config <path>` selects the TOML file used for analyzer settings. When it is
omitted, `sc-boundary` reads `sc-lint.toml` from the workspace root. Boundary
settings live under `[tool.sc-lint.analyzers.boundary]`:

```toml
[tool.sc-lint.analyzers.boundary]
enabled = true
reason = "boundary checks apply to this workspace"
exclude_rules = ["SCB-MANIFEST-001"]
fail_rules = ["cycles", "SCB-DEPENDENCY-001"]
exclude_paths = ["crates/generated/**"]
```

The table supports the common keys `enabled`, `reason`, `exclude_rules`,
`fail_rules`, `exclude_paths`, and `exclude_paths_by_rule`; boundary has no
analyzer-specific keys. `exclude_paths_by_rule` maps an exact rule ID to path
globs and filters findings from that rule without excluding other rules in the
same file. Global and per-rule path exclusions only affect findings with a
source path (or rules whose analyzer resolves their source paths, as
`SCB-CYCLE-001`, `-002` and `-003` resolve the source files behind their
nodes). A finding
from a path-less rule that has no source paths cannot be matched by
`exclude_paths`; it is retained. Per-rule exclusions for such rules are
rejected as `ANALYZER_CONFIG.PATH_EXCLUSION_UNSUPPORTED`.
Rule IDs and family names can be used in `--rule-filter`, `exclude_rules`,
and `fail_rules`.

An unknown `--rule-filter` selector is a usage error and exits with code `2`.
Invalid configuration, including an unknown table key or an invalid key
value, exits with code `3`. These match the top-level CLI's normalized usage
and configuration exit codes.

Graph export stays backend-local and is adjacent to, but not part of, the
top-level `lint sc-boundary` contract:

```bash
cargo run -p sc-lint-boundary -- export-graph --root . --format json
cargo run -p sc-lint-boundary -- export-graph --root . --format turtle
```

## Representative Pass

Source with a local-only API and no cross-module violations:

```rust
mod parser {
    #[sc_lint(boundary.internal_only)]
    pub(crate) struct Token;

    pub(crate) fn parse() -> Token {
        Token
    }
}
```

Representative text result:

```text
sc-lint-boundary: PASS
scanned crates: 1
findings: 0
```

## Representative Fail

Trait marked `forbid_external_impls` but implemented outside its declaring
module:

```rust
mod api {
    #[sc_lint(boundary.forbid_external_impls)]
    pub trait Encode {}
}

mod adapters {
    pub struct Json;

    impl crate::api::Encode for Json {}
}
```

Representative finding:

```text
SCB-BOUNDARY-003 forbid_external_impls violation:
trait crate::api::Encode implemented outside declaring module by crate::adapters::Json
```

Manifest-policy failures also surface through the same tool:

```text
SCB-MANIFEST-001:
workspace.package.version missing from Cargo.toml
```

Named-caller policy uses boundary inventory, not source annotations. A
canonical allowlist entry looks like this:

```toml
[callers]
approved = [
  { symbol = "restricted::run", callers = ["app::allowed::Facade"] },
]
```

That entry means `restricted::run` may be reached by `app::allowed::Facade`,
while any other external caller produces:

```text
SCB-CALLER-001 named caller allowlist violation:
restricted symbol restricted::run called by unapproved external caller other::blocked::Facade
```

Operator guidance:

- use `[callers].approved` when a symbol is valid but only for a named set of
  external callers
- use `references.forbidden` when the boundary should reject the dependency
  edge entirely rather than allow a curated caller set
- `references.scope = "outside_owner_crate"` still exempts owner-crate callers
  for `SCB-CALLER-001`; the allowlist only governs external callers

Package dependency policy also uses boundary inventory rather than source
annotations:

```toml
[dependencies]
allowed_dependents = ["sc-lint"]
allowed_dependencies = ["sc-lint-directives", "sc-lint-schema"]
allowed_dev_dependencies = ["sc-lint-test-support"]
forbidden_edges = [
  { from = "sc-lint-boundary", to = "sc-lint-attributes" },
  { from = "sc-lint-boundary", to = "sc-observability" },
]
```

That entry drives direct-workspace-edge findings through the same command path:

```text
SCB-DEPENDENCY-001 package dependency not allowed:
workspace package `sc-lint-boundary` directly depends on `sc-lint-attributes` but `sc-lint-attributes` is not listed in `BOUNDARY-ScLintBoundaryAnalyzer` allowed_dependencies
```

`allowed_dependencies` permits direct workspace-package dependencies of any
kind. A dev-dependency is also permitted when its target appears in
`allowed_dev_dependencies`; normal and build dependencies still require
`allowed_dependencies`. If an edge is both a normal dependency and a
dev-dependency, it is checked as a normal dependency.

`allowed_dependencies`, `allowed_dev_dependencies` and
`allowed_feature_dependencies` accept a single trailing `*` as a prefix
pattern (for example `"codex-*"` matches every workspace package whose name
starts with `codex-`). A `*` anywhere else, or a bare `*`, is a config error.
A pattern that matches no workspace package is not an error. Patterns match
canonical package names only, and `allowed_dependents` does not accept them.

`SCB-FEATURE-001` (run under `--rule-filter dependencies`) checks an owner only
when `allowed_feature_dependencies` is present. An empty list opts in with no
exemptions; omitting it leaves existing records unchecked. The rule keeps the
owner's feature graph independent of other workspace packages:

- The owner's normal and build dependencies on workspace packages must not
  set `features` or `default-features = false`, and the owner's own
  `[features]` must not forward to them (`"dep:x"`, `"x/feat"`, `"x?/feat"`,
  or a bare `"x"` enabling an optional dependency's implicit feature), unless
  the target package is listed in `allowed_feature_dependencies` (prefix patterns
  allowed). Inherited workspace settings and target-specific dependencies are
  included. Dev-dependencies are not checked, mirroring how
  `SCB-DEPENDENCY-001` treats them separately.
- Workspace dependencies are identified by path resolution, so a registry or
  git dependency that shares a member's name is ignored.

```toml
[dependencies]
allowed_dependents = ["codex-cli"]
allowed_dependencies = ["codex-*"]
allowed_feature_dependencies = ["codex-cli"]
forbidden_edges = []
```

Boundary inventory records may use either structured or arrow-delimited
forbidden edges:

```toml
forbidden_edges = [
  { from = "sc-lint-boundary", to = "sc-lint-attributes" },
  "sc-lint-boundary -> sc-observability",
]
```

Boundary records define exactly one non-empty `[public].facade` or
`[public].trait`. `owner_crate_path` must equal the library target name for
`owner_package` reported by `cargo metadata`, including an explicitly named
`[lib]` target. Do not derive it by replacing hyphens in the package name. For
example, `sc-lint-boundary` has the default library target name
`sc_lint_boundary`, but a package with `[lib] name = "boundary_api"` must use
`boundary_api`.

A boundary record may also carry one opaque `extensions` table, whose
namespaced entries are parsed as TOML and ignored by `sc-lint`:

```toml
[extensions.example]
consumer_metadata = "consumer-defined metadata"
```

Other unknown fields in a boundary record remain errors.

Operator guidance:

- use `[dependencies]` for package-level architectural dependency seams
- use `references.forbidden` for source-level reference bans inside otherwise
  valid package edges
- use manifest policy for workspace metadata hygiene and internal path-version
  alignment rather than architectural package ownership

## Disable Model

Top-level `sc-lint` does not add rule-disable flags for `sc-boundary`.

Supported source-level suppression is limited to the current approved
`#[sc_lint(...)]` boundary directives:

- `#[sc_lint(boundary.allow("cycle.type_method_self_loop"))]`
- `#[sc_lint(boundary.allow("cycle.recursive_value_container"))]`

Supported boundary declarations are:

- `#[sc_lint(boundary.internal_only)]`
- `#[sc_lint(boundary.forbid_external_impls)]`

Important limits:

- these directives are backend-owned, not CLI-owned
- the cycle allowances only apply to the named cycle categories
- there is no top-level `--allow`, `--deny`, or profile-specific rule override
  flag in release `0.1.x`

## Output And Logging

Human output is a presentation layer. The canonical machine surface is:

```bash
sc-lint --json lint sc-boundary
```

That command returns the standard top-level envelope from
[`cli-contract.md`](../../docs/sc-lint/cli-contract.md):

- success:
  - `ok: true`
  - `command: "lint.sc-boundary"`
  - backend findings payload under `data`
- failure:
  - `ok: false`
  - `command: "lint.sc-boundary"`
  - stable `CliError` object under `error`

Structured logs for this tool use:

- service name:
  - `sc-boundary`
- command identity:
  - `lint.sc-boundary`

Read the shared command lifecycle like this:

- `cli.command.started`
  - command entry record for `lint.sc-boundary`
- `cli.command.completed`
  - final verdict plus `summary` and `elapsed_ms`
- `cli.command.error`
  - stable error code plus `CliError.kind` and summary message

Delegated/backend normalization records also appear for this tool:

- `cli.dispatch.started`
- `cli.dispatch.normalized`

For A.7 manifest-policy coverage, the same command path also carries:

- `manifest_policy_mode = "rust-native"`
- `manifest_policy_parity = "python-oracle"`

See:

- [logging.md](../../docs/sc-lint/logging.md)
- [ADR-006](../../docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md)
