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

Phase F adds the product-owned `configure` contract: the CLI owns the
versioned context/request/plan/result schemas, policy validation, and later
apply dispatch; the optional Python/Wyvern adapter is presentation-only. The
authoritative schema definitions and their golden fixtures are in
[`configure-schemas.md`](./configure-schemas.md). Later configuration fixtures
must prove the same public contract on Linux, macOS, and Windows.

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
