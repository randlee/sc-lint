# Adopt `sc-lint` in a Rust repository

The `sc-lint-adoption` package creates one small, managed consumer contract.
After adoption, a contributor or agent only needs these aggregate commands:

```bash
just setup
just lint
just test
```

`just setup` checks or provisions the minimum compatible `sc-lint` release.
`just lint` runs the selected required lint profile. `just test` is the
repository's own recipe. When the `Justfile` has none, the installer adds
sc-lint's recommended `test` recipe, which runs the declared test layers; an
existing `test` recipe is kept. A repository with no `Justfile` starts from
`plugins/sc-lint/templates/Justfile.default`. Consumer-specific recipes remain
ordinary recipes outside the managed `Justfile` import block.

## Install the package and start the skill

Add the repository marketplace, install `sc-lint-adoption`, and invoke the
`sc-lint-adoption` skill from the consumer repository. The skill is the
authoritative seven-step procedure: gather facts, write input, review drift,
apply the kit, run aggregate checks, remove scoped duplicate scaffolding in the
consumer PR, and attach evidence.

The skill delegates production publishing to the `sc-publish` publishing skill.
Adoption itself never publishes a release and does not infer release channels.

## Consumer end state

The installer writes or merges these consumer surfaces:

```text
sc-lint.toml
README.sc-lint.md
Justfile                                  (merged; see below)
.sc-lint/bootstrap.py
.sc-lint/justfile                         (setup, lint and upgrade recipes)
.sc-lint/install.json
.sc-lint/installed.json
.github/workflows/sc-lint.yml
.claude/agents/sc-lint-adopter.md
.claude/skills/sc-lint-adoption/**        (SKILL.md, adopt.xml.j2, evals/)
plugins/sc-lint/install.py
plugins/sc-lint/install.schema.json
plugins/sc-lint/README.md
plugins/sc-lint/analyzer-catalog/**
plugins/sc-lint/schema.d/**
plugins/sc-lint/templates/**
plugins/sc-lint/.claude-plugin/**
plugins/sc-lint/.claude/**
plugins/sc-lint/.github/actions/setup-sc-lint/action.yml
plugins/sc-lint/.sc-lint/**
```

`.sc-lint/install.json` stores normalized JSON while preserving declaration order;
`.sc-lint/installed.json` records the digest of every managed file. The vendored
`plugins/sc-lint/install.py` is the consumer-side installer for later checks
and updates.

A repository with no `Justfile` gets `plugins/sc-lint/templates/Justfile.default`:
`set windows-shell := ["cmd.exe", "/c"]`, a `python` variable, `default: lint`,
the recommended `test` recipe and the managed block.

An existing `Justfile` keeps all of its text. The installer appends the managed
block, and adds the recommended `test` recipe before it when the file has no
`test` recipe. For an existing `Justfile` the block carries the `windows-shell`
setting (unless the consumer already sets `windows-shell` or
`justfile.windows_shell` is false) and the `python` variable the kit recipes use:

```just
# >>> sc-lint managed integration >>>
set windows-shell := ["cmd.exe", "/c"]
python := if os_family() == "windows" { "python" } else { "python3" }
import '.sc-lint/justfile'
# <<< sc-lint managed integration <<<
```

Do not edit text inside that block. Keep repository-specific recipes before or
after it; that is the supported extension point.

The recommended recipe is `test *layer:`, which runs
`.sc-lint/bootstrap.py test --config sc-lint.toml {{layer}}`. The repository owns
it once written. Words after `just test` are forwarded to `sc-lint test` as
`[--fail-fast|--list] [selector [args...]]`: no selector runs every
`[[tool.sc-lint.test]]` row in order, a selector runs that row, and words after
the selector are appended to its command.

The pre-existing-file clash check applies on every install to kit targets
absent from the prior installed manifest. It skips the merged `Justfile` and
skips `.sc-lint/install.json` only when that file is the selected input. If
`--input` points elsewhere, a different pre-existing `.sc-lint/install.json`
is a conflict; all other untracked target paths whose contents differ from the
kit are also reported before the installer writes anything.

## `install.json`

The installer input describes consumer facts, not imperative setup code:

```json
{
  "minimum_version": "0.8.5",
  "profiles": {"ci": ["sc-lint", "lint", "ci"]},
  "ci": {"os": ["linux", "macos", "windows"], "enabled": true},
  "analyzers": {
    "runtime": {"enabled": false, "reason": "no async runtime"},
    "portability": {"enabled": true, "targets": ["linux", "macos", "windows"]}
  },
  "justfile": {"windows_shell": true},
  "test": {"unit": ["cargo", "test"]}
}
```

`minimum_version`, `profiles`, `ci` and `analyzers` are required; any field not
in `plugins/sc-lint/install.schema.json` fails the install.

- `minimum_version` is a SemVer floor, rendered under `[tool.sc-lint]`.
- `profiles` maps each lint step name to its command. Every entry renders as
  one `[[tool.sc-lint.lint]]` row, in declaration order; the example renders
  `name = "ci"`, `command = ["sc-lint", "lint", "ci"]`. An entry may instead be
  an object with `command` plus any of `name`, `platform`, `target`, `host`,
  `after_all`, `env`, `env_paths` and `cwd`.
- `lint_profiles` (optional) groups step names into named profiles under
  `[tool.sc-lint.lint_profiles]`, for example `{"ci": ["boundary", "portability"]}`.
  `default_lint_profile` selects the default under `[tool.sc-lint]`; it can
  name a `lint_profiles` entry.
- `ci` records the platform matrix of the generated workflow. The workflow runs
  `just setup`, then `just lint ci` when `profiles` has a `ci` step (`just lint`
  otherwise), then `just test`. When `lint_profiles` declares `ci`, Linux runners
  run `just lint ci all` and other runners `just lint ci`.
- `analyzers` records each analyzer choice with an explicit reason; derive it
  from observed async-runtime and target-platform facts. Each entry renders as
  `[tool.sc-lint.analyzers.<name>]`.
- `test` (optional) maps test layer names to commands. Each entry renders as one
  `[[tool.sc-lint.test]]` row in declaration order; an object entry takes
  `command` and `after_all`.
- `line_counts`, `identities`, `version_sync`, `cargo_shear`, `logging` and
  `artifacts` (optional) render root policy tables. For example,
  `"identities": {"forbidden_literals": ["example"]}` renders `[identities]`
  with that list. The owning CLI/backend validates each policy.

Null values are not TOML values and are rejected. `profiles`, `lint_profiles`,
`ci` and `test` are consumer-owned recipe declarations, not policy. Keep these
settings in `.sc-lint/install.json` and reinstall; editing the managed
`sc-lint.toml` directly creates drift. Reinstalling the same input is
idempotent. The optional
`justfile` object accepts `windows_shell`, a boolean that defaults to `true`;
set it to `false` to omit the kit's `set windows-shell` line. A consumer-owned
`set windows-shell` is never duplicated.

## Migrate existing named test recipes

Inspect the root `Justfile` for every `test-<name>` recipe. Put that recipe's
underlying command in the matching consumer-owned `install.json` `test` entry;
the installer renders it as a `[[tool.sc-lint.test]]` layer. Then replace the
old recipe body with `just test <name>` so callers retain the named entry point
while execution is declared in the shared test contract. Preserve all
consumer-owned recipes that are not tests. This is an explicit consumer-PR
migration: the kit does not infer commands from arbitrary repository scripts
or delete recipes.

For example, migrate `test-integration` from `cargo test --test integration`
to this input and compatibility recipe:

```json
"test": {"integration": ["cargo", "test", "--test", "integration"]}
```

```just
test-integration:
    just test integration
```

## Drift and safe application

Run the installer with `--dry-run` before every write. It prints unified diffs
for every drifted managed file and returns:

- `0` — the consumer already matches the kit;
- `1` — safe proposed changes are present; review them, then apply;
- `2` — malformed input, a managed-marker conflict, or a pre-existing
  untracked kit path whose content differs from the desired output. The
  installer lists each such path and writes nothing. The clash check runs on
  every install; the merged `Justfile` and selected `.sc-lint/install.json`
  input are exempt. `--dry-run` also exits 2 when the consumer Justfile and
  the kit justfile define the same recipe or setting, and names each one.
  Stop and repair the conflict without overwriting consumer-owned content.

After applying, rerun `--dry-run`. The consumer PR must include the literal
`sc-lint adoption dry-run exit 0` line only when the final recheck converges.
If drift remains, attach that output instead.

## Offline documentation

The product-owned offline documentation bundle is available after adoption.
Locate its installed filesystem path with `sc-lint docs --path`; use
`sc-lint docs` for the overview or `sc-lint docs <guide>` for an installed
guide such as `installation` and `troubleshooting`. Do not substitute a source
checkout for this product-owned bundle.

## How to extend

This guide describes the separate adoption-kit installer. For the installed
product's `sc-lint.toml`, analyzer filters, consumer lint profiles, platform
scope, `exec`, and release recovery contracts, use the canonical offline
[configuration](../../docs-bundle/configuration.md),
[usage](../../docs-bundle/using-sc-lint.md), and
[troubleshooting](../../docs-bundle/troubleshooting.md) guides.

- **Analyzers:** set `analyzers.<name>` in `install.json`; the kit renders it as
  `[tool.sc-lint.analyzers.<name>]`. The keys are listed in `configuration.md`,
  "Analyzer tables"; any other key fails the install.
- **Test layers:** add entries to `test`; each renders as a
  `[[tool.sc-lint.test]]` row. `just test` runs every row in order,
  `just test <layer>` runs one, and words after the layer name are appended to
  its command.
- **Lint steps and profiles:** add entries to `profiles` (one
  `[[tool.sc-lint.lint]]` row each) and group them with `lint_profiles`.
- **Consumer-owned recipes:** add normal root `Justfile` recipes outside the
  managed import block.

Kit-rendered command arrays must name a shipped binary or `sc_lint` module only; profile and test-layer commands are consumer-owned and rendered verbatim.
Do not name a repository-relative script in a kit-rendered step. Model environment, platform, pre-work, and post-work with
declarative fields instead. Only `sc-lint` is kit-pinned; other tool pins stay
consumer-owned.

The worked example at
`tests/fixtures/adoption/analyzer-worked-example/` demonstrates an explicit
runtime reason, Linux portability target, named test layers, lint profile, and
a consumer-owned recipe.

## Assignment template

`packages/sc-lint-adoption/.claude/skills/sc-lint-adoption/adopt.xml.j2` is
rendered by ATM when it assigns the adoption work. The rendered assignment is
consumed by the `sc-lint-adopter` agent and carries the same seven-step
procedure described by the installed skill; it does not define another
workflow.
