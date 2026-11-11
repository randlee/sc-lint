# Adopt `sc-lint` in a Rust repository

The `sc-lint-adoption` package creates one small consumer contract.
After adoption, a contributor or agent only needs these aggregate commands:

```bash
just setup
just lint
just test
```

`just setup` checks or provisions the minimum compatible `sc-lint` release.
`just lint` runs the selected required lint profile. `just test` is the
repository's own recipe.

## Install the package and start the skill

Add the repository marketplace, install `sc-lint-adoption`, and invoke the
`sc-lint-adoption` skill from the consumer repository. The skill is the
authoritative seven-step procedure: gather facts, write input, preview missing
destinations, seed the kit, run aggregate checks, remove scoped duplicate
scaffolding in the consumer PR, and attach evidence.

The skill delegates production publishing to the `sc-publish` publishing skill.
Adoption itself never publishes a release and does not infer release channels.

## Consumer end state

The installer seeds the following destinations only when they are missing. It
leaves an existing file, directory, or symlink untouched, regardless of its
contents; it does not merge, update, or remove consumer content. A dry-run
reports only missing destinations. The root `Justfile` is seeded without
managed markers, and the consumer owns it after creation.

The kit reads `.sc-lint/install.json` as its input; it does not create or
rewrite that input. The seeded consumer surfaces are:

```text
sc-lint.toml
README.sc-lint.md
Justfile
.sc-lint/bootstrap.py
.sc-lint/justfile                         (setup, lint and upgrade recipes)
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

`--dry-run` exits 0 when every destination already exists, 1 when destinations
are missing, and 2 for invalid input. Running the installer seeds only the
paths reported as missing; subsequent runs leave them unchanged.

The recommended recipe is `test *layer:`, which runs
`.sc-lint/bootstrap.py test --config sc-lint.toml {{layer}}`. The repository owns
it once written. Words after `just test` are forwarded to `sc-lint test` as
`[--fail-fast|--list] [selector [args...]]`: no selector runs every
`[[tool.sc-lint.test]]` row in order, a selector runs that row, and words after
the selector are appended to its command.

## `install.json`

The installer input describes consumer facts, not imperative setup code:

```json
{
  "minimum_version": "0.8.7",
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
`ci` and `test` are consumer-owned recipe declarations, not policy. The optional
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

Only `sc-lint` is kit-pinned; other tool pins stay
consumer-owned.

The worked example at
`tests/fixtures/adoption/analyzer-worked-example/` demonstrates an explicit
runtime reason, Linux portability target, named test layers, lint profile, and
a consumer-owned recipe.

## Assignment template

`packages/sc-lint-adoption/skills/sc-lint-adoption/adopt.xml.j2` is
rendered by ATM when it assigns the adoption work. The rendered assignment is
consumed by the `sc-lint-adopter` agent and carries the same seven-step
procedure described by the installed skill; it does not define another
workflow.
