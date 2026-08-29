# Configure wizard handoff fixtures

These are UX-design fixtures for Sprint F.3a. They are not consumer
qualification evidence and may not be used to claim Phase P conversion,
preview/apply/reapply, or CI success for either reference consumer.

## Fixture contract

Each `*-context.json` file is a provenance wrapper. Its `context` member is
validated against the F.1-owned
[`sc-lint-configure-context.schema.json`](../../../schemas/sc-lint-configure-context.schema.json).
The sibling `source` member is F.3a fixture provenance, not an extension to
the public context schema. It identifies the immutable source used to obtain
the observed facts without putting a clone path into a wizard payload.

`request-recommended.json` and `request-existing-conflict.json` validate
against the F.1 request schema. `plan-no-write-conflict.json` validates against
the F.1 plan schema and is the canonical no-write, manual-conflict rendering
scenario. The plan is advisory: its patch is display/export data, never a
permission to write a user-owned file.

## Recorded sources

| Fixture | Repository URL | Baseline commit | Observed result |
| --- | --- | --- | --- |
| `empty-rust-context.json` | `https://github.com/randlee/sc-lint.git` | `39390f659a2a5bb01ffe00704e6ed9055cc3df7a` | F.2's committed `tests/fixtures/configure/empty-rust` workspace |
| `sc-compose-context.json` | `https://github.com/randlee/sc-compose.git` | `38cf63a5e1fe68f93be39fbed30315de4e3b620f` | workspace; existing `sc-lint.toml`, Justfile, and workflow directory are uninspected |
| `atm-core-context.json` | `https://github.com/randlee/atm-core` | `b3475b397c544bd43a43fb97f855b6ddb68f01b1` | workspace; existing Justfile and workflow directory are uninspected |

The snapshots were made in disposable clones. Their working-directory paths,
Git configuration, remotes other than the recorded source URL, and all file
contents outside the bounded F.2 context are intentionally absent.

## Deterministic regeneration

Use any disposable directory and the recorded commit; do not inspect,
transform, or copy consumer utility files. The following command emits the
schema-governed `context` value. Add the recorded `source` object from the
table only when constructing the F.3a provenance wrapper.

```sh
git clone --no-checkout https://github.com/randlee/sc-compose.git /tmp/sc-compose-f3a
git -C /tmp/sc-compose-f3a checkout --detach 38cf63a5e1fe68f93be39fbed30315de4e3b620f
python3 -c 'import json, pathlib, sys; sys.path.insert(0, "scripts"); from sc_lint_configure import collect_context; print(json.dumps(collect_context(pathlib.Path("/tmp/sc-compose-f3a")), indent=2, sort_keys=True))'
```

Run the equivalent command with the recorded `atm-core` URL and commit for
`atm-core-context.json`. For `empty-rust-context.json`, run the same Python
expression with `tests/fixtures/configure/empty-rust` as the root at the
recorded sc-lint commit. The collection performs only F.2 conventional-path
checks; it starts no child process and makes no repository write.

## Redaction rules

- Store only the five F.2 conventional facts, the four standard developer
  commands, and the recorded repository identity/commit.
- Do not include clone paths, home directories, usernames, credential names,
  tokens, remote configuration, source archives, or copied utilities.
- Do not include Justfile, workflow, TOML, source, or shell contents from a
  consumer. `inspected: false` is the required representation of an existing
  Justfile or workflow directory.
- Request fixtures contain typed JSON and argv arrays only. They never contain
  shell fragments, UI markup, or executable commands.

## Expected design and test scenarios

| Scenario | Fixture(s) | Expected handoff behavior |
| --- | --- | --- |
| Empty Rust repository | `empty-rust-context.json`, `request-recommended.json` | Overview reports no existing integration; Just and CI pages show their no-existing-file defaults. |
| Existing consumer integration | `sc-compose-context.json`, `atm-core-context.json`, `request-existing-conflict.json` | Overview and integration pages visibly state that existing files are not inspected; no compatibility verdict or rewrite is offered. |
| Manual collision | `plan-no-write-conflict.json` | Final review shows a manual conflict, exportable diff, recovery action, and unavailable apply confirmation. |
| Navigation | all context fixtures | Back restores selected JSON values; an edited earlier choice truncates later history; cancel and dismiss return `cancelled` without writes. |

The F.3b Wyvern capability suite will turn these scenarios into release-artifact
and screenshot evidence. F.3a supplies data and expected behavior only; it
does not supply a launcher, page asset, screenshot, or consumer conversion.
