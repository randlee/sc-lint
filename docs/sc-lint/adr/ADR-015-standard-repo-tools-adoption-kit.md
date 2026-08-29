# ADR-015 — Standard Repo Tools Adoption Kit

| Field | Value |
| --- | --- |
| ID | ADR-015 |
| Status | Accepted |
| Date | 2026-08-29 |
| Deciders | user, flint, team-lead |
| Approved by | user (Rand Lee), 2026-08-29 |
| Supersedes | ADR-014 (Phase F, rejected, never merged) |

## Context

The sc ecosystem has ~10+ Rust repositories that must use identical
repository tooling, and new Rust repositories must start from the same end
state. Today most development effort is spent re-creating in one repository
what already exists in another. Phase F attempted to solve this with a
`sc-lint configure` Rust engine that carried sc-compose-specific
fingerprints; its target shape no longer exists because `sc-publish`
(`plugins/sc-publish`: skills, agent prompts, GitHub workflows, `install.py`)
now defines how shared tooling is vendored into consumers.

## Decision

1. `sc-lint` ships a vendorable **adoption kit** at
   `packages/sc-lint-adoption`, installed into consumers as
   `plugins/sc-lint`, in the exact form of `sc-publish`: `install.py` driven
   by `install.json`, byte-for-byte copied assets, renamed README
   (`README.sc-lint.md`), templates rendered only for `sc-lint.toml` and the
   `Justfile` import block, `--dry-run` drift as unified diff with exit 1,
   conflict with a user-owned file as exit 2, and no delete operation.
2. The kit is the **only** adoption path. `sc-publish` delegates sc-lint
   setup to the kit (its `setup-sc-lint` action and version pin are removed);
   `sc-lint.toml` `[tool.sc-lint].minimum_version` is the sole version pin in
   a consumer.
3. `sc-lint` crates, scripts, templates, fixtures, and skills work against
   **any** Rust repository. Nothing in this repository names or fingerprints
   a specific consumer. Anything that would exist in two consumers belongs in
   a kit.
4. The consumer interface is ADR-012 unchanged: `just setup | lint | test |
   upgrade`, delegating to `.sc-lint/bootstrap`. The kit vendors the
   product-owned bootstrap assets verbatim.
5. Greenfield and adoption produce an identical end state: the new-repository
   template is the kits applied to an empty Cargo workspace.
6. ADR-014 is rejected. No Phase F code is merged; salvageable text (the
   managed-import marker block, the canonical workflow YAML) is recovered by
   copy into the kit.

## Consequences

- Consumer repositories converge on one auditable file set (see the Phase G
  plan "Consumer End State"); drift is a defect reported by `install.py
  --dry-run`, not a wizard prompt.
- Adoption is a skill plus templates (`sc-lint-adoption`), never a Rust
  engine; the `sc-lint` binary gains no `configure` surface.
- Consumer-specific migration knowledge lives in the consumer's PR, not in
  this repository.
- Rejected alternative: a `sc-lint configure` Rust command with per-consumer
  fingerprints (Phase F). It coupled the product to one consumer's snapshot
  and fought `sc-publish`'s own installer.
