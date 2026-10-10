# ADR-015 — Standard Repo Tools Adoption Kit

| Field | Value |
| --- | --- |
| ID | ADR-015 |
| Status | Proposed (amendment 2026-10-10, phase-h) |
| Date | 2026-08-29 |
| Deciders | user, flint, team-lead |
| Approved by | user (Rand Lee), 2026-08-29 |
| Amendment | 2026-10-10 phase-h: Decision 1 conflict/delete rules, Decision 4 selector arguments and import-only `.sc-lint/justfile`, Decision 7 per-analyzer table contract in `sc-lint-schema`; awaiting approval in PR #206 |
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
   (`README.sc-lint.md`), templates rendered only for `sc-lint.toml`, the
   optional CI workflow and the `Justfile` import block, and `--dry-run`
   drift as unified diff with exit 1. Any conflict with a consumer file exits
   2 with no write; on a first install this includes a pre-existing file at a
   kit path. The kit's only delete operation removes a path it recorded in
   `.sc-lint/installed.json`, still unmodified, that the current kit no longer
   installs.
2. The kit is the **only** adoption path. `sc-publish` delegates sc-lint
   setup to the kit (its `setup-sc-lint` action and version pin are removed);
   `sc-lint.toml` `[tool.sc-lint].minimum_version` is the sole version pin in
   a consumer.
3. `sc-lint` crates, scripts, templates, fixtures, and skills work against
   **any** Rust repository. Nothing in this repository names or fingerprints
   a specific consumer. Anything that would exist in two consumers belongs in
   a kit.
4. The consumer interface is ADR-012's four recipes, `just setup | lint |
   test | upgrade`, delegating to `.sc-lint/bootstrap`
   (`.sc-lint/bootstrap.ps1` on Windows), with `just lint [<selector>
   [<scope>]]` and `just test [<selector>...] [-- <argument>...]` arguments
   passed through unchanged. The kit vendors the product-owned bootstrap
   assets verbatim (byte-identical to `crates/sc-lint/assets/`). The kit's
   `.sc-lint/justfile` is an import target, not a root Justfile: it is
   `crates/sc-lint/assets/consumer-Justfile` without the leading
   `default: lint` recipe, defines only the four recipes as
   platform-attributed `[script]` recipes, and declares no setting, no
   variable and no `default` recipe, so its recipes run under any consumer
   shell. The managed import block adds
   `set windows-shell := ["pwsh", "-NoLogo", "-Command"]` only when
   `install.json` `justfile.windows_shell` is not `false` and the consumer
   Justfile selects no shell; that setting applies only to the consumer's own
   recipes. `install.json` is the JSON image of the rendered `[tool.sc-lint]`
   tables, and the kit renders only for sc-lint 0.6.1 or later.
5. Greenfield and adoption produce an identical end state: the new-repository
   template is the kits applied to an empty Cargo workspace.
6. ADR-014 is rejected. No Phase F code is merged; salvageable text (the
   managed-import marker block, the canonical workflow YAML) is recovered by
   copy into the kit.
7. **Modular applicability.** The independent crates apply only where their
   premises hold. The kit installs one uniform file set but never forces an
   analyzer; `[tool.sc-lint.analyzers]` is the sole repo-specific analyzer
   input, with one table per analyzer such as
   `runtime = { enabled = false, reason = "no blocking waits" }` and
   `portability = { enabled = true, unix_path_prefixes = ["/scratch/"] }`.
   An absent or disabled analyzer is never run by default; run directly, a
   disabled analyzer reports `pass` with no findings. No crate, kit asset,
   skill, or template presumes another crate is enabled. The adoption skill
   derives each table from runtime/platform facts and records why in the
   `reason` key. Templates render the tables from `install.json`; drift fails
   if they disagree. Each analyzer validates and applies its own table
   through the analyzer-configuration contract in `sc-lint-schema`
   (`read_repo_config`, `AnalyzerConfig`, `RuleSelector`, `RuleCatalog`):
   parsing, validation and rule-set arithmetic only, with no selection
   policy. The contract lives in `sc-lint-schema` because every analyzer,
   including `sc-lint-boundary`, already depends on it. Rejected:
   `sc-lint-analyzer-support`, which the boundary analyzer may not depend on
   and which ADR-013 keeps free of rule identifiers; and a new crate, which
   REQ-CLI-011 reserves for explicit design review.

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
- “Identical tooling” means identical installer, interface, and drift check,
  not an identical lint set. The kit covers the common base only and documents
  how to extend; it does not anticipate consumer-specific needs.
- A consumer-local extension observed in at least two repositories may be
  promoted into the kit or wheel as shared base behavior.
