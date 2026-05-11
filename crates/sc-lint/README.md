# `sc-lint`

`sc-lint` is the top-level CLI crate and the canonical non-interactive machine
contract for the `sc-lint` tool family.

## Purpose

Use `sc-lint` when you want a single entry point that:

- parses top-level commands and profiles
- loads repo-root configuration
- dispatches to the owning analyzer crate or Python-backed utility
- normalizes success/error output into the shared CLI contract
- emits structured command lifecycle logs

Implemented analyzer-facing subcommands include:

- `sc-lint lint sc-boundary`
- `sc-lint lint sc-portability`
- `sc-lint lint sc-runtime`
- `sc-lint check native`
- `sc-lint clippy native`
- `sc-lint ci`

## Key Types

Primary CLI-owned types live in this crate:

- `Command`
  - top-level parsed command surface
- `LintProfile`
  - `fast`, `full`, and `ci` lint orchestration policy
- `OutputMode`
  - human vs JSON output selection
- `CliError`
  - stable machine-visible error shape for non-interactive commands

Dispatch and normalization stay in this crate so backend crates keep ownership
of rule logic while `sc-lint` owns the user-facing contract.

## Usage

Human mode:

```bash
sc-lint lint sc-boundary
sc-lint lint sc-portability
sc-lint lint sc-runtime
```

Machine mode:

```bash
sc-lint --json lint sc-boundary
sc-lint --json view findings
sc-lint --json version
```

Repo-local wrappers:

```bash
just lint
just lint sc-boundary
just lint sc-portability
just ci
```

## Related Crates

`sc-lint` is the entry point and delegates to these sibling crates:

- [`sc-lint-boundary`](../sc-lint-boundary/README.md)
- [`sc-lint-portability`](../sc-lint-portability/README.md)
- [`sc-lint-runtime`](../sc-lint-runtime/README.md)
- [`sc-lint-schema`](../sc-lint-schema/README.md)
- [`sc-lint-directives`](../sc-lint-directives/README.md)
- [`sc-lint-attributes`](../sc-lint-attributes/README.md)

## Further Reading

- [CLI requirements](../../docs/sc-lint/cli-requirements.md)
- [CLI architecture](../../docs/sc-lint/cli-architecture.md)
- [CLI contract](../../docs/sc-lint/cli-contract.md)
- [Logging design](../../docs/sc-lint/logging.md)
