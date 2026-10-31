# Configuration

Consumer configuration lives in `sc-lint.toml`:

```toml
[tool.sc-lint]
minimum_version = "0.7.3"

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
non-empty `name` and a non-empty argv `command` array. Entries run
concurrently (cargo steps serialized in declaration order) and every failed
entry is reported. Keep commands explicit and reproducible; do not hide
required work behind an advisory alias.

Lint steps also accept `env` and `cwd`, with the same meaning as these fields
on test command selectors. Values are passed literally to the child process;
sc-lint does not expand variables or resolve environment values as paths.
`cwd` is relative to the repository root.

```toml
[[tool.sc-lint.lint]]
name = "atm-core-clippy"
command = ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]
env = { PYO3_PYTHON = ".bootstrap-venv/bin/python" }
cwd = "crates/atm-core"
```

Here `PYO3_PYTHON` is passed verbatim; the child tool decides how to interpret
it. `[tool.sc-lint].setup` and `[tool.sc-lint].runner` are not supported keys.
When `--log-root` is set, each executed profile step writes both output streams
to `<log-root>/steps/<expanded-plan-index>-<safe-name>.log`. Failed steps also
show stdout and stderr in the command error.

Instead of rows, `[tool.sc-lint.test]` may be a table of named selectors with
an optional `default`:

```toml
[tool.sc-lint.test]
default = "unit"

[tool.sc-lint.test.selectors.unit]
command = ["cargo", "test"]
env = { RUST_LOG = "debug" }
cwd = "crates/app"
platforms = ["linux", "macos"]
after_all = false

[tool.sc-lint.test.selectors.integration]
command = ["cargo", "test", "--test", "integration"]

[tool.sc-lint.test.selectors.full]
group = ["unit", "integration"]
```

A file uses one form; see [Test selectors](./using-sc-lint.md#test-selectors).

## Analyzer tables

Each analyzer reads only its own `[tool.sc-lint.analyzers.<name>]` table, where
`<name>` is `boundary`, `portability` or `runtime`. Common keys are:

- `enabled` (boolean, default `true`) turns the analyzer on or off.
- `reason` (string) records why; the analyzer does not read it.
- `exclude_rules` (rule ids or families) drops those rules' findings.
- `fail_rules` (rule ids or families) lists the rules that fail the verdict;
  other findings only report. Absent, the analyzer's built-in set applies.
- `exclude_paths` (globs relative to the workspace root; `**` spans
  directories) drops findings in matching files. In Boundary, only
  `SCB-MANIFEST-001` through `SCB-MANIFEST-006` carry a finding path;
  `SCB-CYCLE-002` and `SCB-CYCLE-003` instead use the source files behind
  their nodes, and are excluded when any such file matches.
- `exclude_paths_by_rule` maps exact rule ids to arrays of workspace-relative
  globs. It drops only findings from that rule at matching paths; other rules
  in the same file still run. Boundary supports `SCB-MANIFEST-001` through
  `SCB-MANIFEST-006`, `SCB-CYCLE-002`, and `SCB-CYCLE-003`. Entries for all
  other Boundary rules are configuration errors naming the rule because their
  findings have no source path; configuration loading rejects them before
  analysis. Remove the entry or choose a rule with source paths.

`portability` also takes `config_home_env`, the environment variable a test
helper must check before `dirs::home_dir()` (PORT-002 runs only when it is
set), `unix_path_prefixes`, which adds to the built-in PORT-001 and PORT-006
prefix lists, and `targets`, an accepted platform declaration that no current
rule reads. It also accepts `platform_adapters`: explicit pairs of Unix and
Windows module paths, with a review reason, for equivalent implementations
whose names differ.
Only the configured pair and its enclosed private implementation receive the
adapter treatment; separately Unix-gated public items remain subject to parity
checks. Boundary accepts the optional manifest dependency policy families
`manifest_dependency_allowlists`, `global_dependency_ownership`, and
`manifest_section_rules`; these are released consumer-owned configuration,
while atm-core retains its record-lock parity checks for the allowlist fields.
Its package-edge policy is supplied by its inventory. Runtime has no
analyzer-specific keys. Any unknown
key, unknown rule, or wrong value type is a configuration error (exit 3)
identifying the analyzer and field.

```toml
[tool.sc-lint.analyzers.portability]
fail_rules = ["PORT-001", "PORT-002"]
exclude_paths = ["tests/fixtures/**"]
unix_path_prefixes = ["/opt/"]

[tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
"SCB-RUNTIME-001" = ["tests/fixtures/generated/**"]

[[tool.sc-lint.analyzers.portability.platform_adapters]]
unix_module = "src/platform/unix.rs"
windows_module = "src/platform/windows.rs"
reason = "These modules implement the same private adapter interface."
```

`sc-lint lint sc-boundary|sc-portability|sc-runtime --rule-filter <RULE-ID|family>[,...]`
keeps exactly the selected rules and overrides `exclude_rules`; path exclusions
still apply, and `fail_rules` still decides which selected findings fail the
verdict. Unknown selectors are usage errors (exit 2). A lint-fail verdict exits
10; malformed analyzer configuration exits 3. The report remains available in
text or JSON output.

Boundary package-edge policy is expressed in the boundary inventory records.
Boundary manifest dependency policy is declared in the
`[[tool.sc-lint.analyzers.boundary.<family>]]` tables of `sc-lint.toml`, not
in Cargo manifests. Optional policy records add policy checks only
when present; an explicitly empty allowlist allows no entries. See the
[boundary guide](./packages/sc-lint-boundary.md) for neutral examples.

## Policy and logging

Repository policy belongs in the commands and their checked-in configuration.
Logging options can be set under the repository's existing logging settings;
they do not change the compatibility contract. Use `--json` for machine
consumers and keep the human diagnostics available for operators.

See [profiles, platform scope, and declared tools](./using-sc-lint.md#profiles-platform-scope-and-declared-tools)
for consumer lint-step syntax, and the package guides for analyzer
configuration details.
