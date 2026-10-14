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

## Governing ADRs

- [ADR-005 — CLI profiles and cross-target preflight](./adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [ADR-006 — AI-first CLI contract](./adr/ADR-006-ai-first-cli-contract.md)
- [ADR-007 — Analyzer-crate partition](./adr/ADR-007-analyzer-crate-partition.md)
- [ADR-008 — Structured logging boundary](./adr/ADR-008-sc-observability-logging.md)
- [ADR-009 — Observability boundary policy](./adr/ADR-009-observability-boundary-policy.md)
- [ADR-011 — Interface versioning](./adr/ADR-011-interface-versioning-and-published-artifacts.md)
- [ADR-012 — Consumer adoption and Just contract](./adr/ADR-012-consumer-adoption-and-just-contract.md)
- [ADR-015 — Standard repo-tools adoption kit](./adr/ADR-015-standard-repo-tools-adoption-kit.md)
- [ADR-016 — Python wheel runtime](./adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md)
- [ADR-017 — Phase K consumer contracts](./adr/ADR-017-phase-k-consumer-contracts.md)

## Boundary Rules

- backend crates do not own top-level parsing or envelope normalization
- delegated backend binaries remain backend-owned tools behind the CLI surface
- structured logging stays in the CLI crate, not in backend crates
- future `sc-lint-version` integration remains a top-level command path, not a
  separate user-facing entrypoint

## Related Docs

- [requirements.md](./requirements.md)
- [crate-architecture.md](./crate-architecture.md)
