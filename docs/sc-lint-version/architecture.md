# `sc-lint-version` Architecture

This document records the crate-local architecture summary for the planned
`sc-lint-version` crate.

## Role

`sc-lint-version` is the planned dedicated workspace crate for multi-family
interface-version checks.

It owns:

- Rust public API semver-check translation
- CLI and RPC/socket interface baseline comparison
- the multi-family business verdict model
- canonical interface artifacts consumed by the shared reporting layer

## Authoritative Architecture Sources

- [requirements.md](./requirements.md)
- [../phase-C/phase-C-plan.md](../phase-C/phase-C-plan.md)
- [../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)
- [../architecture.md](../architecture.md)

## Boundary Rules

- human-facing HTML/XHTML report rendering stays outside this crate
- the preferred shared reporting home is the `sc-compose` orbit
- the crate integrates through `sc-lint check interfaces`, not a separate
  end-user entrypoint

## Related Docs

- [../sc-lint/interface-reporting-constraints.md](../sc-lint/interface-reporting-constraints.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
