# sc-lint-boundary CLI Reference

Version 0.8.7

Generated from the CLI command tree. Do not hand-edit.

## `sc-lint-boundary`

AST-sensitive Rust boundary analyzer

```text
Usage: sc-lint-boundary <COMMAND>
```

### `sc-lint-boundary analyze`

Analyze Rust dependencies and manifest policy in a workspace

```text
Usage: sc-lint-boundary analyze [OPTIONS]
```

**Options:**

- `--config <CONFIG>` — Analyzer configuration file; defaults to discovery from the workspace root
- `--format <FORMAT>` — default: `text` — choices: `text, json` — Findings output format
- `--root <ROOT>` — default: `.` — Workspace root to inspect
- `--rule-filter <RULE-ID|FAMILY[,..]>` — Comma-separated rule IDs or supported rule families; overrides configured excludes

### `sc-lint-boundary export-graph`

Export the workspace dependency graph

```text
Usage: sc-lint-boundary export-graph [OPTIONS]
```

**Options:**

- `--format <FORMAT>` — default: `json` — choices: `json, turtle` — Graph serialization format
- `--root <ROOT>` — default: `.` — Workspace root to inspect
