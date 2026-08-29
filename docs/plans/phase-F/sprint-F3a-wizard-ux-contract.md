---
id: F.3a
title: Configure Wizard UX Contract And Wyvern Handoff Package
status: planned
target: develop
---

# Sprint F.3a — Configure Wizard UX Contract And Wyvern Handoff Package

## Goal

Write the authoritative, implementation-neutral UX specification that the
Wyvern team and the sc-lint adapter implement without rediscovering product
policy. This is specification work only: it creates no launcher, web page, or
target-repository mutation.

## Hard Dependencies

- F.1 accepted configure request/plan/error schemas and ADR-014;
- F.2 accepted bounded-context and deterministic-plan contract;
- the `sc-compose` repository at `https://github.com/randlee/sc-compose.git`
  and `atm-core` repository at `https://github.com/randlee/atm-core`, each
  cloned to any engineer-selected local path, pinned to its recorded baseline
  commit, and copied into a disposable fixture/worktree before inspection.

## Exact Targets

- `docs/sc-lint/configure-wizard-ux.md` (new, authoritative handoff)
- `docs/sc-lint/configure-wizard-fixtures/empty-rust-context.json` (new)
- `docs/sc-lint/configure-wizard-fixtures/sc-compose-context.json` (new)
- `docs/sc-lint/configure-wizard-fixtures/atm-core-context.json` (new)
- `docs/sc-lint/configure-wizard-fixtures/request-recommended.json` (new)
- `docs/sc-lint/configure-wizard-fixtures/request-existing-conflict.json` (new)
- `docs/sc-lint/configure-wizard-fixtures/plan-no-write-conflict.json` (new)
- `docs/sc-lint/configure-wizard-fixtures/README.md` (new)

## Deliverables

- A ten-page UX contract: overview, baseline, boundary, portability, runtime,
  attributes/directives, command groups, Just integration, CI integration, and
  final review. For every page it names visible fields, default/recommended
  value, help text, JSON pointer, validation rule, enabled/disabled condition,
  `Back`/`Next`/`Cancel` label, and the exact next-page branch.
- The handoff fixes the following page sequence so the Wyvern team can design
  presentation without choosing product behavior. The authored UX document
  expands every row into exact field labels, help, error/recovery copy, and
  schema references.

  | Page | Required decision/data | Forward rule |
  | --- | --- | --- |
  | 1. Overview | bounded facts, four-command contract, proposed files, reasons | `Next` enters baseline; no decision bypasses this page |
  | 2. Baseline | accept, modify argv arrays, or disable baseline format/Clippy/test profile | valid selection enters boundary |
  | 3. Boundary | disabled/recommended/enabled plus structured inventory setting | valid selection enters portability |
  | 4. Portability | disabled/recommended/enabled | valid selection enters runtime |
  | 5. Runtime | disabled/recommended/enabled | valid selection enters attributes/directives |
  | 6. Attributes/directives | declarative source intent; no executable profile command | valid selection enters command groups |
  | 7. Command groups | every optional named group explicitly enabled, disabled, or unselected | selection enters Just integration |
  | 8. Just integration | keep, generate, supported migration, or conflict/patch review | enters CI; unresolved collision blocks confirmation |
  | 9. CI integration | keep, generate Action, supported patch, or conflict/patch review | enters final review; unresolved collision blocks confirmation |
  | 10. Final review | normalized request, ordered no-write plan, diffs/conflicts/manual steps | `Confirm` emits request/plan only; `Back` restores data |
- A stable layout contract: progress indicator, page title, explanation panel,
  selection controls, immutable discovered-facts panel, validation/error panel,
  and a compact pending-change summary. The overview and final review have
  their own specified layouts; no product copy or interaction is left to a page
  implementer to invent.
- State rules: all selected values survive back/forward restoration; a changed
  branch truncates stale forward pages; cancellation/dismissal returns
  `cancelled` without writes; final confirmation produces a request/plan only,
  not apply; unresolved conflict makes the apply confirmation unavailable.
- A handoff fixture pack with schema-valid, sanitized context/request/plan JSON
  for an empty Rust repo and for the current sc-compose and atm-core shapes.
  The README records source repository URLs, baseline commits, fixture-generation
  command, redaction rules, and expected screenshot/test scenarios.
- An explicit Wyvern capability matrix that distinguishes required host
  features from sc-lint-owned domain behavior. Required host features are
  multi-page page descriptors, browser-history back/forward restoration,
  conditional next-page branching, opaque per-page data, cancel/dismiss,
  finish with full stack, local-only serving, and headless deterministic tests.
  The plan records that released Wyvern 0.1.0 has only single-dialog commands
  and therefore does not meet this contract.

## Production-Ready Closure

Every listed deliverable must land production-ready for the F.3a UX handoff:
the complete page contract, fixtures, and host-capability criteria are usable
without rediscovering policy. Nothing may be deferred except work explicitly
listed in [This Sprint Does Not Close](#this-sprint-does-not-close).

## Required Contract Samples

The fixture pack must retain these normalized shapes (with actual observed
facts, not hand-authored guesses). These examples give the Wyvern team stable
data to render while F.1 finalizes the published schemas:

```json
{
  "schema_version": "v1",
  "context": {
    "cargo_toml": {"present": true, "kind": "workspace"},
    "sc_lint_toml": {"present": true},
    "justfile": {"present": true, "inspected": false},
    "github_workflows": {"present": true, "inspected": false},
    "sc_lint_directory": {"present": false}
  },
  "source": {
    "repository": "sc-compose",
    "repository_url": "https://github.com/randlee/sc-compose.git",
    "baseline_commit": "38cf63a5e1fe68f93be39fbed30315de4e3b620f"
  }
}
```

```json
{
  "schema_version": "v1",
  "context": {
    "cargo_toml": {"present": true, "kind": "workspace"},
    "sc_lint_toml": {"present": false},
    "justfile": {"present": true, "inspected": false},
    "github_workflows": {"present": true, "inspected": false},
    "sc_lint_directory": {"present": false}
  },
  "source": {
    "repository": "atm-core",
    "repository_url": "https://github.com/randlee/atm-core",
    "baseline_commit": "b3475b397c544bd43a43fb97f855b6ddb68f01b1"
  }
}
```

```json
{
  "schema_version": "v1",
  "request": {
    "minimum_version": "0.5.0",
    "lint_families": {
      "baseline": {"state": "recommended"},
      "boundary": {"state": "enabled", "settings": {"inventory": "detect"}},
      "portability": {"state": "enabled"},
      "runtime": {"state": "disabled"},
      "attributes": {"state": "recommended"}
    },
    "just": {"mode": "keep_existing"},
    "ci": {"mode": "keep_existing"}
  }
}
```

The committed fixture JSON contains repository identity and baseline commit,
not any local clone path. The fixture README records the remote URL,
generation command, and redaction rules; an engineer may use any clone
location. These source records are planning inputs, never data sent to or
rendered by a consumer wizard.

## Acceptance Criteria

- A Wyvern engineer can implement the wizard without asking what appears on a
  page, what a choice means, where it is stored, how it is validated, or which
  navigation result follows it.
- Every human-visible choice has one JSON representation; every JSON field has
  a named page and validation/recovery copy. No UI-only policy exists.
- The fixture pack contains no executable shell text, credentials, local home
  path, source archive, or copied utility, and reproduces the current consumer
  facts from the recorded baseline commits.
- F.3a fixture repositories are UX-design inputs only. They may not be cited
  as Phase P qualification evidence, which requires fresh released-artifact
  preview/apply/reapply and CI evidence for both consumers.
- the authoritative UX document includes the complete per-page table above
  with field labels, help/error/recovery copy, JSON pointers, defaults,
  validation, and navigation. A prose summary or static mock-up without it is
  not an acceptable handoff.
- The capability matrix has a testable acceptance case for every required
  Wyvern feature and labels missing capability as blocking—not a reason for an
  sc-lint Python state-machine workaround.
- The F.1-owned ADR-014 names this handoff package as the page-behavior
  authority and records that the wizard is capability-gated; it does not claim
  Wyvern 0.1.0 has a released wizard API.

## Required Validation

- JSON Schema validation for every fixture and pointer referenced by the UX
  contract
- fixture redaction review and deterministic regeneration check
- Markdown-link validation and a page/field/pointer completeness check
- `just lint`
- `just test`

## This Sprint Does Not Close

- release qualification or implementation of a Wyvern multi-page host;
- an sc-lint launcher, HTML/CSS/JS page, or target-repository write;
- consumer conversion, which remains Phase P work.
