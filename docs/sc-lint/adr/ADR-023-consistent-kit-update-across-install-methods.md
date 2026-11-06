# ADR-023 — One Install And Upgrade Path, With Documented File Ownership

| Field | Value |
| --- | --- |
| ID | ADR-023 |
| Status | Pending — Rand acceptance required before sprints o-15, o-16, o-11 and o-12 are dispatched |
| Deciders | Rand |
| Related | ADR-012, ADR-015 (vendored-kit fallback superseded), ADR-016, ADR-017 (tool declarations move), ADR-022; REQ-PRODUCT-019, -020, -021, -022, -024, REQ-PRODUCT-025 (new); #862, #903, #835 |

## Rand's direction (2026-11-05)

- "we need to consistently update the repo through all supported install
  methods. we can not leave stale scripts to support one method and not the
  other."
- "does install.py install sc-lint? consumers should be told to remove it."
- "We should always install following the same path and everything needs to
  work. not overwriting a file that we maybe use is not a requirements based
  decision."
- "There must be clear documented config files and what we 'upgrade' and what
  customers use for config must be clearly documented."
- "the decisions you make must work on atm-core and on sc-lint even if it
  involves them following a documented procedure to clean things up. I do not
  want more scripts that 'guess' about what to do." — "must work on atm-core,
  sc-compose, sc-observability."
- "we should NOT have labels that do nothing."

## Context

Facts verified 2026-11-05 at sc-lint `develop` and in each consumer:

- Three routes write into a consumer repository: `sc-lint init --just` (Rust),
  and the kit's `install.py` run from the marketplace plugin or from the
  vendored `plugins/sc-lint/` copy. They follow different rules. `init` refreshes
  `bootstrap.py` only when its digest matches a known prior release and
  otherwise refuses (`consumer_integration.rs:24-27, 279-309`). `install.py`
  writes only paths that do not exist (`install.py:255, 282`), so no kit file is
  ever refreshed (#903).
- `install.py` does not install sc-lint. `.sc-lint/bootstrap.py` installs the
  verified release and the version-matched wheel into `.sc-lint/venv`.
- `bootstrap.py` and `sc_lint.setup_tools` read `tools` from
  `.sc-lint/install.json` at setup (`bootstrap.py:233, 280`;
  `setup_tools.py:28`), but the adoption skill writes `install.json` at the
  repository root (`SKILL.md:50`), so tools declared through the skill are not
  provisioned.
- Product files exist in several unsynchronized copies: three `action.yml`
  variants and four Justfile sources (only `bootstrap.py` copies are
  test-checked).
- Consumers diverge:
  - **atm-core** (0.8.4): full kit adoption: `plugins/sc-lint/**` (30 files),
    `README.sc-lint.md`, `.sc-lint/{bootstrap.py,justfile,install.json,installed.json}`,
    the adoption skill and agent, a `Justfile` that imports `.sc-lint/justfile`,
    `kit-drift-*` lint steps that run the vendored `install.py`, and five tests
    reading `install.json`. `install.json` alone holds
    `tools = {cargo: {cargo-xwin: 0.18.4}, rustup_targets: [x86_64-pc-windows-msvc]}`.
  - **sc-compose** (0.4.0): `sc-lint.toml`, `Justfile`, a repo-local
    `.github/actions/setup-sc-lint`, `scripts/materialize_sc_lint_runtime.py`
    that copies helpers out of the source tarball into `.just/`, no `.sc-lint/`.
  - **sc-observability** (0.4.0 or a source build at `ba2d9bf6`): no
    `sc-lint.toml`, lowercase `justfile` that never calls sc-lint, a repo-local
    `setup-sc-lint` action, a homegrown source installer writing
    `.sc-lint/source-install.json`, and `boundaries/**/*.toml`.
  - **sc-lint** itself: `sc-lint.toml`, `Justfile`, `.sc-lint/bootstrap.py`.

## Decision

### 1. One path

| Step | The only supported way |
| --- | --- |
| Install the binary | any release channel: Homebrew, release archive, Scoop, winget, or the `randlee/sc-lint@v1` Action in CI. All deliver the same verified artifact (REQ-PRODUCT-020/021/022). Building sc-lint from source is not a consumer install method. |
| Adopt a repository | `sc-lint init --just`, then edit `sc-lint.toml`. The adoption skill does exactly this; it has no installer of its own. |
| Upgrade | `just upgrade`, which runs, in order: the binary and wheel upgrade (`.sc-lint/bootstrap.py upgrade`), `sc-lint init --just`, and tool provisioning. |
| Python helpers | come only from the version-matched wheel in `.sc-lint/venv` (REQ-PRODUCT-024); never copied into the repository. |

### 2. File ownership

**Your data goes in `sc-lint.toml`. sc-lint owns `.sc-lint/`.** Every path
sc-lint touches is in this table; sc-lint never writes any other path.

| Path | Owner | What `sc-lint init --just` does |
| --- | --- | --- |
| `sc-lint.toml` | **consumer** — all sc-lint configuration, including `[tool.sc-lint.tools]` | creates it with defaults only if missing; never reads it for rewrite, never overwrites it |
| `Justfile` or `justfile` | **consumer** | creates `Justfile` only if neither name exists. The consumer keeps the one line `import '.sc-lint/justfile'`; sc-lint never edits an existing Justfile |
| `.github/workflows/*` | **consumer** | never written. The CI guide gives the `randlee/sc-lint@v1` snippet to copy |
| `.sc-lint/bootstrap.py` | **sc-lint** | always overwritten with the shipped bytes |
| `.sc-lint/justfile` | **sc-lint** | always overwritten with the shipped bytes |
| `.claude/skills/sc-lint-adoption/**` | **sc-lint** | always overwritten with the shipped bytes |
| `.claude/agents/sc-lint-adopter.md` | **sc-lint** | always overwritten with the shipped bytes |
| `.sc-lint/venv/` | **sc-lint**, untracked state | not written by `init`; provisioned by `just setup`; listed in `.gitignore` |

sc-lint-owned files are not edit points: every `init` overwrites them without
comparing content. No digest list, no conflict detection, no "keep if it
exists".

### 3. Retired paths — removed by the consumer, following the migration guide

| Path | Why retired | Where its data goes |
| --- | --- | --- |
| `plugins/sc-lint/**` (includes `install.py`, `tests/`) | installer code copied into the consumer goes stale (#903) | nothing; `sc-lint init --just` replaces it |
| `.sc-lint/install.json` | second consumer config file | `tools` → `[tool.sc-lint.tools]` in `sc-lint.toml`; everything else is already in `sc-lint.toml` or was an adoption-time choice now owned by the consumer's `Justfile` / workflow |
| `.sc-lint/installed.json` | digest manifest for a refresh heuristic | nothing |
| `.sc-lint/source-install.json` | source builds are not an install method | nothing |
| `README.sc-lint.md` | offline docs come from `sc-lint docs` (REQ-PRODUCT-021) | nothing |
| `.github/actions/setup-sc-lint/**` (any repo-local copy) | the released `randlee/sc-lint@v1` Action replaces it (REQ-PRODUCT-022) | its `version` input → `minimum_version` |
| sc-lint helper copies under `.just/` (`run_lint.py`, `lint_common.py`, `check_version_sync.py`, `.sc-lint-runtime-version`, …) and scripts that download, copy or build them | helpers come from the wheel (REQ-PRODUCT-024) | nothing |
| `[[tool.sc-lint.lint]]` / `[[tool.sc-lint.test]]` rows and consumer tests that call any retired path | they call files that no longer exist | delete the row, or point it at `sc-lint.toml` |

`sc-lint init --just --check` exits non-zero when an sc-lint-owned file
differs from the shipped bytes. It does not search for retired paths; the
migration guide lists them and the consumer removes them.

### 4. Version pins

`[tool.sc-lint].minimum_version` is the only sc-lint version a consumer
declares to sc-lint (REQ-PRODUCT-019); it is a floor. Pins a consumer keeps for
its own purposes — Cargo dependencies on the `sc-lint-attributes` or
`sc-lint-directives` crates, or another environment that installs the wheel —
are the consumer's concern (#835) and are not read or checked by sc-lint.

### 5. One source per sc-lint-owned file

Each sc-lint-owned file has exactly one source in this repository, embedded
in the binary. No other copy exists: the kit's copies, `install.py`, its
templates and the vendored `setup-sc-lint` action are removed.

### 6. Consumer canary before every publish

Release preflight installs the release candidate and runs `just setup`,
`just lint` and `just test` in atm-core, sc-compose and sc-observability; any
failure blocks the release. A change that breaks a consumer ships only with its
migration step in `upgrade.md`, in the same PR.

## Migration guide (shipped as `docs-bundle/upgrade.md`, "Migrating to 0.8.7")

The guide is the procedure; sc-lint runs no migration script. Steps for
every repository, in order:

1. Install the 0.8.7 binary through a release channel.
2. If `.sc-lint/install.json` has `tools`, copy it into `sc-lint.toml` as
   `[tool.sc-lint.tools]` (exact shape in the guide).
3. Delete every retired path from section 3 that exists, and every lint/test
   row, script or test that calls one.
4. Run `sc-lint init --just`. Ensure the Justfile contains
   `import '.sc-lint/justfile'`.
5. Set `minimum_version = "0.8.7"`; point CI at `randlee/sc-lint@v1`.
6. Run `sc-lint init --just --check`, then `just setup`, `just lint`,
   `just test`. All pass.

The guide has one short subsection each for atm-core (0.8.4 kit adoption),
sc-compose (0.4.0, repo-local action and `.just/` helper copies) and
sc-observability (0.4.0 or source build, lowercase `justfile`) naming the files
each removes; sc-lint follows the same contract.

## Consequences

- Every route ends in the same repository state; there is one consumer config
  file and one set of sc-lint-owned files, and both are documented.
- `ci.*` and `justfile.windows_shell` stop being settings: they only shaped
  files the consumer now owns (no settings that do nothing).
- `bootstrap.py` and `sc_lint.setup_tools` read tools from `sc-lint.toml`.
- Consumers that skip the guide keep working until a retired file breaks; the guide lists every leftover and `init --check` catches stale sc-lint-owned files.

## Verification

The consumer canary (decision 6). For 0.8.7 it runs against each consumer's
migration PR; the release waits until all three pass.
