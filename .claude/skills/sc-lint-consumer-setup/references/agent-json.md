# Agent JSON configure route

This reference documents the public, deterministic F.2 planning command used
by the [`sc-lint-consumer-setup`](../SKILL.md) skill. It accepts an explicit
F.1 request and creates a no-write preview only.

## Inputs

Choose the consumer root explicitly. Supply a schema-valid F.1 request from a
file or standard input. The accepted F.3a page-selection examples are
[`request-recommended.json`](../../../../docs/sc-lint/configure-wizard-fixtures/request-recommended.json)
and
[`request-existing-conflict.json`](../../../../docs/sc-lint/configure-wizard-fixtures/request-existing-conflict.json).
The authoritative field definition is the
[F.1 request schema](../../../../schemas/sc-lint-configure-request.schema.json);
the success/failure envelope is the
[F.1 result schema](../../../../schemas/sc-lint-configure-result.schema.json).

## Preview from a request file

```text
sc-lint configure --request <request.json> --root <consumer-root> --dry-run --json
```

The command returns one JSON envelope with `ok: true`,
`command: "configure.plan"`, and a schema-valid plan at `data`. It does not
create, replace, delete, install, or execute anything in the selected root.

The F.3c conformance example, executed by the repository test suite, is:

```text
sc-lint configure --request docs/sc-lint/configure-wizard-fixtures/request-recommended.json --root tests/fixtures/configure/agent/empty-rust --dry-run --json
```

## Preview from standard input

Invoke the same command with `--request -`, then provide the complete request
object on standard input:

```text
sc-lint configure --request - --root <consumer-root> --dry-run --json
```

The standard-input bytes must be the same schema-valid JSON that would appear
in a request file. Do not construct a shell pipeline, interpolate values, or
append repository-derived commands. File and standard-input routes produce the
same JSON plan for the same root and request.

The conformance suite executes the standard-input form against
`tests/fixtures/configure/agent/sc-compose` with the accepted
`request-existing-conflict.json` payload.

## Result handling

On success, preserve the returned operation order and show every operation to
the user. `needs_confirmation`, conflicts, and manual steps mean the target
remains user-owned; they do not authorize a write. Explicit user confirmation
is required before a later sprint's apply workflow may be considered.

On failure, the command writes a schema-valid F.1 failure envelope. Report:

- `error.code` for stable classification;
- `error.pointer` for the only JSON value to repair;
- `error.recovery` and `error.recovery_description` for the next action; and
- `error.docs_ref` for offline documentation.

For example, a boundary choice with `decision: "modify"` but no `settings`
returns `CLI.CONFIGURE_UNSUPPORTED_SCHEMA` with pointer
`/request/lint_families/boundary`. Repair that request field and rerun the
same preview. The target root remains unchanged.

## Prohibited additions

This route has no discovery or wrapper extension point. Do not parse a
Justfile or workflow, scan source, call Cargo metadata, launch Wyvern, use a
second dispatcher, define a schema copy, or turn the JSON request into shell
text. The public command performs the bounded observation and all request/plan
validation.
