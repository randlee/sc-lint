# sc-lint-runtime CLI Reference

Version 0.8.8

Generated from the CLI command tree. Do not hand-edit.

## `sc-lint-runtime`

AST-sensitive Rust runtime analyzer

```text
Usage: sc-lint-runtime <COMMAND>
```

### `sc-lint-runtime analyze`

Analyze Rust source for runtime and async safety issues

```text
Usage: sc-lint-runtime analyze [OPTIONS]
```

**Options:**

- `--config <CONFIG>` — Analyzer configuration file; defaults to discovery from the workspace root
- `--format <FORMAT>` — default: `text` — choices: `text, json` — Findings output format
- `--root <ROOT>` — default: `.` — Workspace root to inspect
- `--rule-filter <SCB-RUNTIME-001..008[,..]>` — Comma-separated rule IDs: SCB-RUNTIME-001 through SCB-RUNTIME-008. Rule families are not supported
