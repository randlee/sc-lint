# Cross-Platform Guidelines

This document defines the high-level cross-platform expectations for `sc-lint`.

## Scope

`sc-lint` must run correctly on:

- Linux
- macOS
- Windows

This applies to:

- Rust crates
- repo-local lint runner automation
- CI workflows
- release packaging where applicable

## Requirements

- Prefer platform-neutral path handling.
- Do not assume Unix-only filesystem roots or separators.
- Keep shell and scripting behavior compatible with the repo CI matrix.
- Treat Windows support as first-class, not best-effort.

## Enforcement

- CI must continue to run on:
  - `ubuntu-latest`
  - `macos-latest`
  - `windows-latest`
- Portability rules in `sc-lint-portability` should continue to evolve as a
  separate lint family.

## Related Docs

- [docs/architecture.md](./architecture.md)
- [docs/project-plan.md](./project-plan.md)
- [docs/sc-lint-boundary/graph-schema.md](./sc-lint-boundary/graph-schema.md)
