# sc-lint CLI Reference

Generated from the CLI command tree. Do not hand-edit.

## `sc-lint`

Stable top-level CLI for the sc-lint tool family

```text
Usage: sc-lint [OPTIONS] [COMMAND]
```

**Options:**

- `--config <PATH>` — Read the consumer configuration from this path, relative to the repository root when relative
- `--json <JSON>` — choices: `true, false` — Write the stable JSON envelope to stdout (errors are written to stderr)
- `--log-console <LOG_CONSOLE>` — choices: `true, false` — Enable console logging for this command
- `--log-root <PATH>` — Override the configured command log directory
- `--root <PATH>` — Repository root used by lint, view, check, clippy, compatibility check, test, and CI

### `sc-lint check`

Run the native or Windows cross-target cargo check

```text
Usage: sc-lint check <TARGET>
```

**Arguments:**

- `target` *(required)* — choices: `native, xwin`

### `sc-lint ci`

Run the configured consumer CI profile, or the source-maintainer profile in a source checkout

```text
Usage: sc-lint ci
```

### `sc-lint clippy`

Run the native or Windows cross-target cargo clippy analysis

```text
Usage: sc-lint clippy <TARGET>
```

**Arguments:**

- `target` *(required)* — choices: `native, xwin`

### `sc-lint compatibility`

Check the configured consumer installation and version requirement

```text
Usage: sc-lint compatibility <COMMAND>
```

#### `sc-lint compatibility check`

Verify the installed sc-lint binary satisfies sc-lint.toml

```text
Usage: sc-lint compatibility check [OPTIONS]
```

**Options:**

- `--binary <path>` — Binary to probe. Defaults to `sc-lint` resolved from PATH

### `sc-lint docs`

Read an installed guide from the offline documentation bundle

```text
Usage: sc-lint docs [OPTIONS] [GUIDE]
```

**Arguments:**

- `guide` — choices: `README, installation, using-sc-lint, configuration, just-setup, ci, upgrade, troubleshooting, best-practices, cli-reference, sc-lint, sc-lint-attributes, sc-lint-analyzer-support, sc-lint-boundary, sc-lint-directives, sc-lint-portability, sc-lint-runtime, sc-lint-schema` — Guide to print or resolve. Defaults to the overview (`README`)

**Options:**

- `--path <PATH>` — choices: `true, false` — Print the guide or bundle path without a label

Guide aliases: `README.md` and `overview` select `README`; `setup` selects `installation`.

### `sc-lint init`

Create or verify the product-owned consumer integration files

```text
Usage: sc-lint init [OPTIONS] --just
```

**Options:**

- `--check <CHECK>` — choices: `true, false` — Verify that the generated integration is current without changing files
- `--dry-run <DRY_RUN>` — choices: `true, false` — Report the changes that would be made without changing files
- `--just <JUST>` *(required)* — choices: `true, false` — Generate the canonical thin Just integration

### `sc-lint lint`

Run one analyzer, maintainer lint profile, or configured consumer lint profile

```text
Usage: sc-lint lint [OPTIONS] <TARGET>
```

**Arguments:**

- `target` *(required)* — choices: `sc-boundary, sc-portability, sc-runtime, function-length, line-counts, identity-literals, fast, full, ci`

**Options:**

- `--consumer <CONSUMER>` — choices: `true, false` — Run a configured consumer profile. Supported only with `ci`
- `--fail-fast <FAIL_FAST>` — choices: `true, false` — Stop starting new cargo-lane and `after_all` steps after the first failure; running steps finish
- `--platform-scope <PLATFORM_SCOPE>` — choices: `host, all, linux, macos, windows` — Consumer CI platform scope: `host` runs unannotated and host-tagged steps; `all` also reports unsupported cross-platform steps; a named platform adds its steps. Requires `--consumer`
- `--profile <PROFILE>` — Select a consumer lint profile or step (`all` runs every step in declaration order); requires `--consumer`
- `--rule-filter <RULE_FILTER>` — Run these analyzer rules (`<RULE-ID|family>[,...]`); forwarded unchanged to the selected analyzer

### `sc-lint setup`

Install or repair the managed sc-lint release required by sc-lint.toml

```text
Usage: sc-lint setup [OPTIONS]
```

**Options:**

- `--dry-run <DRY_RUN>` — choices: `true, false` — Report the selected release and install location without changing either

### `sc-lint test`

Run the explicitly configured consumer test profile

```text
Usage: sc-lint test [OPTIONS] [SELECTOR [ARGS]]...
```

**Arguments:**

- `selector_and_args` — Selector to run, then the arguments appended to its command. Omitted runs `default` (every row of a `[[tool.sc-lint.test]]` list). Flags (`--fail-fast`, `--list`, `--config`, `--json`) must precede the selector; everything after it is forwarded verbatim. One positional, so clap stops parsing flags at the selector

**Options:**

- `--fail-fast <FAIL_FAST>` — choices: `true, false` — Same as `lint --consumer --fail-fast`
- `--list <LIST>` — choices: `true, false` — Print the declared selectors instead of running them

### `sc-lint upgrade`

Inspect or update the managed sc-lint release required by sc-lint.toml

```text
Usage: sc-lint upgrade [OPTIONS]
```

**Options:**

- `--check <CHECK>` — choices: `true, false` — Report whether the configured minimum version requires an update
- `--dry-run <DRY_RUN>` — choices: `true, false` — Report the selected release and install location without changing either

### `sc-lint version`

Print the product, backend, and contract versions

```text
Usage: sc-lint version
```

### `sc-lint view`

Show analyzer findings or the reserved graph view

```text
Usage: sc-lint view <TARGET>
```

**Arguments:**

- `target` *(required)* — choices: `graph, findings`
