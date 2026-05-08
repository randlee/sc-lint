# sc-lint CLI Architecture

This document records the intended architecture of the top-level `sc-lint` CLI
crate.

## Role

The `sc-lint` CLI is the orchestration layer for the tool family.

It owns:

- command parsing
- config loading
- output/exit-code normalization
- dispatch to backend tools

It does not own:

- deep backend analysis logic
- backend crate business rules

## Architectural Model

The intended model is:

```text
sc-lint (CLI)
  -> Rust library backend, when available
  -> Rust binary backend, when needed
  -> Python utility, during migration windows
```

## Dispatch Principles

- backend crates remain self-contained
- the CLI decides which backend is used
- backend crates do not call each other directly
- backend replacement should not require changing the CLI command contract

## Initial Command Families

- `lint`
  - backend lint tools and wrappers
- `view`
  - report and visualization tools
- `version`
  - version and upgrade inspection

## Config Flow

Expected flow:

1. discover repo root
2. load shared config
3. resolve subcommand/tool target
4. dispatch to backend
5. normalize output and exit code

## Output Model

The CLI should present:

- consistent human-readable text output
- stable machine-readable output modes where supported
- stable success/failure exit codes across delegated tools

## Migration Role

The CLI is specifically intended to let `sc-lint` evolve without forcing all
tools into Rust at once.

That means it should tolerate:

- Rust-native tools
- Python-backed tools
- future extracted binaries

behind one stable user-facing surface.
