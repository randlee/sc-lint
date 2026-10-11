# Configuration

Consumer configuration lives in `sc-lint.toml`:

```toml
[tool.sc-lint]
minimum_version = "0.4.0"

[[tool.sc-lint.lint]]
name = "rust"
command = ["cargo", "check", "--workspace"]

[[tool.sc-lint.test]]
name = "tests"
command = ["cargo", "test", "--workspace"]
```

## Minimum version

`minimum_version` is parsed as SemVer and compared semantically. The installed
version must be greater than or equal to the floor; lexical string comparison
is not used. Missing, malformed, or too-old versions stop lint/test before
backend execution. Run `sc-lint compatibility check --config sc-lint.toml` to
inspect the preflight without running a profile.

## Profiles

Each `[[tool.sc-lint.lint]]` and `[[tool.sc-lint.test]]` entry needs a unique,
non-empty `name` and a non-empty argv `command` array. Entries run in order and
any failed entry fails the aggregate profile. Keep commands explicit and
reproducible; do not hide required work behind an advisory alias.

## Analyzer tables

Each analyzer reads only its own `[tool.sc-lint.analyzers.<name>]` table, where
`<name>` is `boundary`, `portability` or `runtime`. Every table takes:

- `enabled` (boolean, default `true`) turns the analyzer on or off.
- `reason` (string) records why; the analyzer does not read it.
- `exclude_rules` (rule ids or families) drops those rules' findings.
- `fail_rules` (rule ids or families) lists the rules that fail the verdict;
  other findings only report. Absent, the analyzer's built-in set applies.
- `exclude_paths` (globs relative to the workspace root; `**` spans
  directories) drops findings in matching files. Boundary applies it to
  SCB-MANIFEST findings, matched on the manifest path.

`portability` also takes `config_home_env`, the environment variable a test
helper must check before `dirs::home_dir()` (PORT-002 runs only when it is
set), `unix_path_prefixes`, which extends the built-in PORT-001 and PORT-006
prefix lists, and `targets`, the platforms the repository supports (accepted;
no rule reads it yet). `boundary` and `runtime` have no keys of their own.
Any other key is a configuration error (exit 3) naming the analyzer and the key.

```toml
[tool.sc-lint.analyzers.portability]
fail_rules = ["PORT-001", "PORT-002"]
exclude_paths = ["tests/fixtures/**"]
unix_path_prefixes = ["/opt/"]
```

`sc-lint lint sc-boundary|sc-portability|sc-runtime --rule-filter <RULE-ID|family>[,...]`
keeps exactly the selected rules and overrides `exclude_rules`; `fail_rules`
still decides the verdict. An unknown selector is a usage error (exit 2).

## Policy and logging

Repository policy belongs in the commands and their checked-in configuration.
Logging options can be set under the repository's existing logging settings;
they do not change the compatibility contract. Use `--json` for machine
consumers and keep the human diagnostics available for operators.

See [using sc-lint](./using-sc-lint.md) and the package guides for analyzer
configuration details.
