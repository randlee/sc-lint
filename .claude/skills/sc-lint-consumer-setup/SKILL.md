---
name: sc-lint-consumer-setup
description: Create and preview a deterministic, no-write sc-lint configure request for a Rust consumer repository.
---

# sc-lint consumer setup

Use this skill only for the deterministic JSON preview route. It is suitable
for an agent or automation that must describe exactly what it is setting up
without parsing a repository, prompting through a browser, or making a write.

## Boundary

The public command is:

```text
sc-lint configure --request <request.json|-> --root <consumer-root> --dry-run --json
```

It is the single validation and planning authority. The command observes only
the conventional facts in the F.1 [configure schemas](../../../docs/sc-lint/configure-schemas.md):
the root Cargo marker and presence of `sc-lint.toml`, `Justfile`, `.sc-lint/`,
and `.github/workflows/`. It does not approve, parse, or rewrite a present
Justfile or workflow. This F.3c skill does not launch Wyvern, render wizard
pages, apply a plan, or write the target repository.

Do not add a wrapper, repository probe, Cargo metadata call, source scan,
installer command, shell fragment, or a second schema. Do not infer an
integration posture from file contents. Use only the declared command and
JSON data whose fields are owned by the F.1 request schema.

## Agent workflow

1. Ask for the explicit consumer repository root. Do not search upward,
   choose a workspace automatically, or reuse an unrelated working directory.
2. Review the supplied F.3a fixture/context facts when available. Treat an
   existing Justfile or workflow as **present but not inspected**. If context
   is absent, the public preview command is the only permitted bounded
   observation; do not invent supplementary probes.
3. Create a versioned request conforming to the F.1 request schema. Every
   lint-family and Just/CI posture must be an explicit JSON decision. Start
   from the accepted F.3a mapping when it matches the requested setup.
4. Run the no-write preview command. Read the complete JSON envelope, not
   terminal formatting. A successful result has `ok: true`,
   `command: "configure.plan"`, and an ordered plan under `data`.
5. For `ok: false`, stop. Report `error.code`, `error.pointer`, `error.recovery`,
   `error.recovery_description`, and `error.docs_ref`; repair only the named
   JSON value, then preview again. Never guess a correction from repository
   contents.
6. For a successful plan, present the exact request and every operation,
   conflict, and manual step to the user. Obtain explicit confirmation before
   requesting any later apply workflow. A F.3c preview itself authorizes no
   write, and this skill must not invoke an apply command.

## Required JSON discipline

- Commands in a baseline profile are arrays of argv tokens, never shell text.
- `--request -` reads one complete JSON request from standard input; it is not
  an invitation to compose a shell pipeline.
- Validate against the F.1 schema references; do not copy a schema into a
  skill, prompt, or wrapper.
- Preserve unknown user-owned files. A conflict or `needs_confirmation`
  operation is review data, never permission to overwrite it.

See [the agent JSON reference](references/agent-json.md) for executable
file/stdin examples and the exact result/error handling contract.
