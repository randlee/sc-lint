# `sc-lint` Crate Architecture

This document records the crate-local architecture summary for the top-level
`sc-lint` package.

## Role

`sc-lint` is the stable CLI crate. It coordinates:

- command resolution
- repo-root and config discovery
- backend dispatch and output normalization
- CLI-owned structured logging
- top-level interface-version checks through `check.interfaces`

## Authoritative Architecture Sources

The detailed architecture authorities for this crate are:

- [cli-architecture.md](./cli-architecture.md)
- [cli-contract.md](./cli-contract.md)
- [logging.md](./logging.md)
- [../architecture.md](../architecture.md)

## Boundary Rules

- backend crates do not own top-level parsing or envelope normalization
- delegated backend binaries remain backend-owned tools behind the CLI surface
- structured logging stays in the CLI crate, not in backend crates
- future `sc-lint-version` integration remains a top-level command path, not a
  separate user-facing entrypoint

## Related Docs

- [requirements.md](./requirements.md)
- [crate-architecture.md](./crate-architecture.md)
