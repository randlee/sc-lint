# `sc-lint-analyzer-support` Architecture

## Role

The neutral `WorkspaceMetadata::workspace_packages()` accessor adapts Cargo
metadata into dependency and target records without embedding analyzer policy.
It preserves dependency kinds, renames and target platforms, and target source
paths and kinds for consumers that need workspace facts.

This library provides rule-neutral AST source discovery, scope classification,
text report rendering, and shared analyzer configuration parsing. It does not
define lint rules, rule IDs, or an analyzer command surface.

Private modules, whose items the crate root re-exports:

- `analyze_options` — `AnalyzeOptions`, the shared root and output-format
  input. `sc-lint-portability` and `sc-lint-runtime` re-export it;
  `sc-lint-boundary` converts its own options (which add the rule filter) into
  it.
- `relative_path` — `repo_relative_source_path`, `RepoRelativeSourcePathError`
  and `relative_path_under_root`: repository-relative, slash-separated path
  normalization and root-prefix stripping used by discovery and findings.
- `source_scan` — source discovery, `classify_scope`, and `is_test_only`
  test-attribute recognition (`#[test]`, qualified test attributes,
  `cfg(test)`, `cfg(all(test, ...))`).
- `analyzer_config` — the shared `[tool.sc-lint.analyzers.<name>]` parser.
- `render` — `render_findings_report`, the shared text report.

Since 0.8.5 these responsibilities include the shared `AnalyzeOptions` input
(workspace root and output format) that the analyzers re-export,
repository-relative source-path normalization (`repo_relative_source_path`,
`RepoRelativeSourcePathError`), root-prefix stripping
(`relative_path_under_root`) and test-only attribute recognition
(`is_test_only`). These fit ADR-013 as written: path normalization belongs to
source discovery, `is_test_only` to scope classification, and
`AnalyzeOptions` carries no rule ids or analyzer policy, so no ADR amendment
is needed.

## Governing ADRs

- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [ADR-013 — Analyzer shared support](../sc-lint/adr/ADR-013-analyzer-shared-support.md)

## Boundary Rules

- Analyzer crates may depend on this crate; this crate must not depend on an
  analyzer or the top-level CLI.
- Rule IDs, rule-specific policy, and backend machine contracts remain owned
  by each analyzer.
- Shared configuration behavior uses only rule data supplied by the analyzer.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-analyzer-support/README.md)
