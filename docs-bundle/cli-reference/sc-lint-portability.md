# sc-lint-portability CLI Reference

Version 0.8.8

Generated from the CLI command tree. Do not hand-edit.

## `sc-lint-portability`

AST-sensitive Rust portability analyzer

```text
Usage: sc-lint-portability <COMMAND>
```

### `sc-lint-portability analyze`

Analyze Rust source for portability issues

```text
Usage: sc-lint-portability analyze [OPTIONS]
```

**Options:**

- `--config <CONFIG>` — Analyzer configuration file; defaults to discovery from the workspace root
- `--format <FORMAT>` — default: `text` — choices: `text, json` — Findings output format
- `--root <ROOT>` — default: `.` — Workspace root to inspect
- `--rule-filter <PORT-RULE-ID[,..]>` — Comma-separated PORT rule IDs; overrides configured excludes
