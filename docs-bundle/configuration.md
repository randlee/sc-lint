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

## Policy and logging

Repository policy belongs in the commands and their checked-in configuration.
Logging options can be set under the repository's existing logging settings;
they do not change the compatibility contract. Use `--json` for machine
consumers and keep the human diagnostics available for operators.

See [using sc-lint](./using-sc-lint.md) and the package guides for analyzer
configuration details.
