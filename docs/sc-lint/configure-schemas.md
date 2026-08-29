# Configure Contract Schemas

Phase F has exactly four public JSON-schema authorities. They are versioned
`v1` contracts for `sc-lint configure`; a UI, agent skill, launcher, or later
apply implementation may consume them but must not define a parallel request,
plan, or result shape.

| Schema | Purpose | User-facing meaning |
| --- | --- | --- |
| [`sc-lint-configure-context.schema.json`](../../schemas/sc-lint-configure-context.schema.json) | no-write conventional-path observation | explains what was found without claiming that existing Justfile or workflow content was understood |
| [`sc-lint-configure-request.schema.json`](../../schemas/sc-lint-configure-request.schema.json) | explicit human/agent choices | records the requested version, each lint-family decision, and Just/CI posture as data rather than executable shell text |
| [`sc-lint-configure-plan.schema.json`](../../schemas/sc-lint-configure-plan.schema.json) | ordered preview of later mutations | exposes proposed work, conflicts, manual steps, and exportable patches before any write |
| [`sc-lint-configure-result.schema.json`](../../schemas/sc-lint-configure-result.schema.json) | stable CLI success/failure envelope | preserves the `configure`, `configure.plan`, and `configure.apply` identity and recovery-bearing errors |

## Context

The context schema is deliberately small. It reads only root `Cargo.toml`,
`sc-lint.toml`, `Justfile`, `.github/workflows/`, and `.sc-lint/` presence.
When a Justfile or workflow exists, `"inspected": false` is mandatory: it
means that sc-lint has not parsed, approved, or classified that user-owned
content. No context field represents source scans, Cargo metadata, command
execution, or a repository-compatibility verdict.

The `explanation` section carries the four commands users are setting up and
the existing integration that remains uninspected. The wizard's first page and
agent output render this data; neither may infer additional facts.

## Request

Every lint family is named: baseline, boundary, portability, runtime, and
attributes/directives. Each has a stable `state` plus a `decision`:

- `accept_recommendation` accepts the displayed recommendation;
- `modify` requires explicit structured `settings`;
- `disable` requires `state: "disabled"`.

The request contains only JSON values and argv arrays. It never accepts shell
fragments or UI markup. `just` and `ci` choose a bounded integration posture;
their existing content remains uninspected until a later fixture-proven
transformer can act on an approved plan.

## Plan and Result

The plan is separate from the top-level result envelope so F.2 can create it
and F.4a/F.4b can consume it verbatim. It has a versioned identifier,
operation order, conflicts, and manual steps. `operation_id` is the
cross-collection join key: every item in `conflicts` and every
`manual_steps[].operation_id` names an entry in `operations[].operation_id`.
The schema gives each occurrence one shared identifier type; F.2 validates
that the named operation exists when it builds a plan. A
`needs_confirmation` operation must name its reason and allowed choices. A
`manual_conflict` must carry both a typed conflict and an exportable unified
diff; neither is a permission to write an unknown file.

An operation that creates, updates, or removes a transaction artifact also
uses the optional additive `artifact_kind` field (`toml`, `justfile`, `shell`,
`json`, or `workflow_yaml`). It records the concrete private transaction type
in a reviewable plan without creating a public extension mechanism. Existing
v1 plans remain valid because the field is optional.

Result success uses the normal CLI envelope with the plan under `data`.
Failures use a fixed configure-code set and include a JSON pointer (or `null`
when no pointer applies), a stable `recovery` token, a short human-readable
`recovery_description`, and an offline documentation reference. Optional
`message` and `cause` fields retain the common `CliError` explanatory context
without creating a second error shape. `ConfigureError` normalizes through
this envelope; it is not a second error format.

## Golden Fixtures

The canonical samples live in
[`tests/fixtures/configure/contracts/`](../../tests/fixtures/configure/contracts/).
They are reproduced verbatim in Sprint F.1 and checked by
[`test_configure_contract_schemas.py`](../../.just/tests/test_configure_contract_schemas.py).
F.2, F.4a, and F.4b consume these fixtures rather than redefining public
fields. The test validates every fixture with Draft 2020-12 JSON Schema and
also proves the Sprint F.1 fenced examples have not drifted from the fixtures.

## Ownership and Evolution

Only an additive, backward-compatible schema revision may introduce a new
`v1` optional field. A breaking change requires a new schema version, updated
fixtures, documented migration, and an ADR/requirements review. Product policy
and writes remain in the released `sc-lint` command; the Python/Wyvern adapter
only presents or collects values allowed by these schemas.
