# ADR-023 — One Update Contract Across Install Methods

| Field | Value |
| --- | --- |
| ID | ADR-023 |
| Status | Pending — Rand approval required before sprint o-2 is dispatched |
| Deciders | Rand |
| Related | ADR-012, ADR-015, ADR-016, ADR-022; REQ-PRODUCT-019, REQ-PRODUCT-023; #862, #903 |

## Context

The installer is seed-only (ADR-022, #862): it writes a destination only when
it does not exist. The kit projects every package file into
`plugins/sc-lint/`, including `install.py` and `tests/`. An upgrade therefore
leaves the vendored `install.py` at its old version but seeds newer tests
against it, so the 0.8.6 kit tests fail on a 0.8.4 install (#903). The tools
are unaffected; the vendored installer's TOML writer is.

Rand, 2026-11-05: "1 - remove the kit tests. 2 - we need to consistently
update the repo through all supported install methods. we can not leave stale
scripts to support one method and not the other."

## Decision (proposed, for Rand's ruling)

1. The kit's tests are never projected into a consumer (sprint o-1).
2. Kit files are either **product-owned** (refreshed to the shipped bytes by
   every install method) or **consumer-owned** (seed-only, ADR-022). No file
   is product-owned for one method and seed-only for another.
3. The vendored `plugins/sc-lint/install.py` is product-owned. Every method
   (marketplace plugin run from `CLAUDE_PLUGIN_ROOT`, the vendored kit, the
   `just upgrade` recipe and `.sc-lint/bootstrap.py`) runs or refreshes the
   same version-pinned installer; none depends on a stale seeded copy.
4. Consumer-owned files (`Justfile`, `sc-lint.toml`, `install.json`) remain
   seed-only.

Sprint o-2 first commits the complete method inventory in this ADR and
implements the contract identically across it.

## Alternatives

- **Do not vendor the installer.** Methods run only the version-pinned
  installer from the plugin or wheel; no stale copy can exist. Simpler, but
  drops the offline vendored-kit fallback (ADR-015, SKILL.md fallback).
- **Refresh by version stamp.** Vendored product-owned files carry a version
  stamp and are replaced when older. Keeps the fallback; adds a stamp contract.

## Consequences

Upgrades converge on the shipped bytes for product-owned files in every
method. The open ruling is which alternative to adopt; o-2 is not dispatched
until Rand approves.
