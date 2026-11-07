# DRAFT: ADR-023 / REQ-PRODUCT-025 / ADR-012 amendment for phase-p

Status: **draft, awaiting Rand's approval.** Nothing below is applied yet. Each change cites the anchor (`anchors.md`) that drives it.

## ADR-023 changes

### Status row
`Accepted — Rand, 2026-11-06; amended <date> (init deletes retired paths; no init --check)`

### §2 File ownership: replace the paragraph under the table
> sc-lint-owned files are not edit points: every `init` overwrites them with the embedded bytes without reading or comparing them. There is no digest list, no conflict detection, no "keep if it exists", and no `--check` mode. The consumer reviews what `init` changed in its commit or pull-request diff. (ANC-002, ANC-005)

### §3: retitle and change the first two rows
Retitle: **"Retired paths — deleted by `sc-lint init --just`; other leftovers removed by the consumer following the migration guide"**

Add a column **Removed by**:

| Path | Removed by |
|---|---|
| `plugins/sc-lint/**` | `init` (ANC-004) |
| `.sc-lint/install.json` | `init`; the consumer copies `tools` to `[tool.sc-lint.tools]` **first** |
| `.sc-lint/installed.json` | `init` |
| `.sc-lint/source-install.json` | `init` |
| `README.sc-lint.md` | `init` |
| `.github/actions/setup-sc-lint/**` | consumer, per the guide (owned by sc-publish, which retires it upstream) |
| sc-lint helper copies under `.just/` and the scripts that make them | consumer, per the guide (consumer scripts; open-ended, so `init` never guesses at them, ANC-012) |
| `[[tool.sc-lint.lint]]`/`[[tool.sc-lint.test]]` rows and consumer tests calling a retired path | consumer, per the guide |

Replace the sentence about `init --just --check` with:
> `init` deletes only the fixed paths marked `init` above; it never inspects other files to decide what to remove (ANC-012). Files under an sc-lint-owned directory that sc-lint no longer ships are deleted with that directory's refresh.

### Migration guide section
- Step 2 → "**Before running `init`**, copy `tools` from `.sc-lint/install.json` into `sc-lint.toml` as `[tool.sc-lint.tools]`; `init` deletes `install.json`."
- Step 3 → "Delete the consumer-removed leftovers in §3 and every lint/test row, script or test that calls a retired path."
- Step 6 → "Run `sc-lint init --just`, review the diff, then run `just setup`, `just lint` and `just test`. All pass."
- Replace "The guide has one short subsection each…" with: "Each consumer has its own guide under `docs/migrations/0.8.7/` (atm-core @ `integrate/phase-bg`, sc-compose @ `integrate/phase-v`, sc-observability), naming every break and its fix (ANC-006 to ANC-008)."

### §6 Consumer canary
Change the consumer list to **atm-core and sc-compose** (ANC-010). sc-observability has no `sc-lint.toml` or `.sc-lint/` yet, so a canary there has nothing to exercise; its guide says so. **⚠ Rand to confirm.**

### Consequences
Replace "`init --check` catches stale sc-lint-owned files" with "every `init` restores sc-lint-owned files, and the diff shows what changed".

## REQ-PRODUCT-025: replacement text
> sc-lint is installed through a release channel, and a repository is adopted and upgraded only through `sc-lint init --just` (upgrade: `just upgrade`). All consumer configuration lives in `sc-lint.toml`, including `[tool.sc-lint.tools]`; sc-lint owns `.sc-lint/` and the adoption skill and agent. Every path sc-lint writes or deletes is in the ADR-023 ownership and retired-path tables. Consumer files are created only when missing and never rewritten. sc-lint-owned files are overwritten with the shipped bytes on every `init`, with no content comparison, digest list, conflict detection or `--check` mode. `init` deletes the fixed retired paths ADR-023 marks `init`; the consumer removes the other leftovers following its migration guide. sc-lint runs no detection heuristics. Each sc-lint-owned file has exactly one source in this repository.

## ADR-012: amend decision 3
> `--dry-run` is non-mutating. Re-running a current integration is idempotent. (`--check` removed by ADR-023 amendment <date>.)

## Open for Rand
1. ANC-005: drop `init --check` entirely? (Flint recorded your "yes, … will show up when commit/pr" as agreement; please confirm.)
2. Canary consumers: atm-core and sc-compose only, or add sc-observability?
3. Are atm-core, sc-compose and sc-observability the only consumers 0.8.7 must not break, or are there others from the 10+ to add?
