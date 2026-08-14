---
id: F.3
title: Agent JSON And Explanatory Wyvern Configuration Wizard
status: planned
target: develop
---

# Sprint F.3 — Agent JSON And Explanatory Wyvern Configuration Wizard

## Goal

Expose the F.2 plan through a first-class agent JSON contract and a Wyvern web
UI that tells a human what sc-lint is about to set up before asking for choices.
Both routes produce the same schema-valid request JSON; neither can make a
repository change on its own.

## Hard Dependencies

- F.1 request/plan/UI contract and ADR-014
- F.2 bounded context and deterministic plan
- verified Wyvern 0.1.0 `wyvern-wizard`/`wyvern-host` protocol at the supported
  pinned interface

## Exact Targets

- `scripts/sc_lint_configure.py`
- `scripts/sc_lint_configure_wyvern.py` (new)
- `schemas/sc-lint-configure-context.schema.json` (new)
- `schemas/sc-lint-configure-request.schema.json` (new)
- `.claude/skills/sc-lint-consumer-setup/SKILL.md` (new)
- `.claude/skills/sc-lint-consumer-setup/references/agent-json.md` (new)
- configure front-end fixtures/tests
- `docs-bundle/configuration.md`
- `docs-bundle/using-sc-lint.md`
- `docs-bundle/best-practices.md`
- `docs-bundle/troubleshooting.md`

## Deliverables

- `sc-lint configure --request <path|-> --root <path> --dry-run --json` is the
  canonical agent path. It accepts a JSON document or stdin, never starts a UI,
  returns its normalized request and F.2 plan, and has no mutation capability
  in this MVP.
- the sc-lint-owned Python launcher starts the verified Wyvern 0.1.0 host/wizard
  protocol for `--ui wyvern` and waits for its JSON POST result. It remains
  thin: it passes validated context to the schema-driven dialog and normalizes
  the submitted JSON; it does not duplicate product planning policy or directly
  link a Rust Wyvern library. No new Rust wizard binary is introduced.
- the first wizard page is “What sc-lint will set up.” It shows conventional
  facts found, the developer commands it will standardize, all proposed files,
  changes it intentionally will not inspect, and the reasons for each
  recommendation. Subsequent pages are baseline, boundary, portability,
  runtime, attributes/directives, consumer command groups, Just integration,
  CI integration, and final review. Every page shows the current selection,
  recommendation, rationale, and accept/modify/disable choice.
- the attributes/directives page describes attribute availability and
  declarative boundary intent separately from lint execution; it does not
  fabricate an `sc-lint-attributes` executable profile.
- the final review renders the F.2 advisory plan, affected paths, uninspected
  integration, conflicts, version authority, profile commands, and CI choice.
  It records explicit confirmation for the later F.4 apply operation;
  cancellation/close/error returns `cancelled` and has no file effects.
- Wyvern input/output is mapped through a narrow Python adapter with a pinned
  protocol version, timeout/error handling, and schema validation. The product
  remains functional when Wyvern is not installed.
- the Claude Code skill tells an agent how to run bounded context collection,
  construct/validate the request JSON from repository knowledge, use dry-run,
  interpret `manual_conflict`, and require an explicit user confirmation before
  a future apply operation. It never screen-scrapes the UI or invents a probe.

## Acceptance Criteria

- JSON request and Wyvern answers for the same selections normalize
  to equal schema-valid request JSON and equal `configure.plan` JSON.
- each page can be accepted with the recommended value, explicitly modified,
  or explicitly disabled; an agent can express every equivalent choice in JSON.
- no family page exposes an analyzer binary name, Cargo package selection,
  copied script, or shell command string as a consumer-facing requirement.
- malformed/unknown Wyvern response and unavailable Wyvern produce stable
  errors with a JSON recovery path.
- context/request schemas make every UI field and response unambiguous; tests
  assert the first-page explanation includes detected facts, proposed files,
  no-write scope, and the standard developer command contract.
- cancellation and timeout are tested without invoking a real browser; a
  protocol fixture validates the optional real-Wyvern adapter separately.
- offline docs explain agent JSON, the page model, each input field, preview,
  cancellation, and recovery.

## Required Validation

- JSON schema/golden-envelope tests
- Wyvern adapter protocol fixtures, unavailable/exited/timed-out cases
- cross-adapter request/plan equivalence tests
- Claude Code skill example/contract validation
- `just lint`
- `just test`

## This Sprint Does Not Close

- application of the final plan;
- supported legacy deletion or sc-compose conversion;
- generated Action workflow validation.
