# `atm-core` Import Candidates For `sc-lint`

This inventory reviews `atm-core` `develop` for active-use integration assets
that are not yet present in `sc-lint`.

Scan result:

- no top-level `.claude/skills/` entries are missing in `sc-lint`
- no top-level `.claude/agents/` entries are missing in `sc-lint`
- the remaining deltas are templates, tests, and consumer-repo wrappers

## Candidate Entries

| Source path in `atm-core` | What it does | Scope | Recommendation |
| --- | --- | --- | --- |
| `.claude/skills/codex-orchestration/sprint-plan.md.j2` | Machine-parsable sprint-plan template that matches the current `atm-core` sprint-doc shape: frontmatter, exact targets, deliverables, required work, explicit code samples, non-closure, acceptance criteria, and validation. | Repo-agnostic; directly useful to `sc-lint` plan hardening. | `import` |
| `scripts/test_find_todos.py` | Unit tests for `scripts/find_todos.py`, which is already present in `sc-lint` and is now referenced by the imported `todo-triage` skill. | `sc-lint`-specific because the runtime script already exists locally. | `import` |
| `scripts/test_triage_carry_forward.py` | Unit tests for `scripts/triage_carry_forward.py`, which is already present in `sc-lint` and is now referenced by the imported `triaging-findings` skill. | `sc-lint`-specific because the runtime script already exists locally. | `import` |
| `.just/lint_unix_gating.py` | Consumer-repo wrapper that runs the portability subset and reports only `PORT-004` / `PORT-005` findings in the house lint format. | Consumer integration for repos that adopt `sc-lint`; not core tool-repo behavior. | `adapt` |
| `.just/check_test_identity_literals.py` | Consumer-repo wrapper for identity-literal policy using repo-local config and suppression conventions. | Consumer integration; `sc-lint` owns the underlying capability but not this exact repo policy. | `adapt` |
| `.just/check_line_counts.py` | Consumer-repo line-budget checker over workspace crates with repo-local thresholds and exclusions. | Consumer integration; overlaps with the extracted `line-counts` capability but carries repo-specific policy. | `adapt` |
| `.just/run_view.py`, `.just/build_view_site.py`, `.just/view_*.py`, `.just/templates/view-*.j2` | Repo-specific view-site generation for `atm-core` architecture reports. | Strongly `atm-core`-specific; current `sc-lint` product surface does not ship these view targets. | `skip` |
| `.just/lint_same_host_portability.py` | Repo-specific portability guard for same-host daemon adapters and legacy TODO markers. | Strongly `atm-core`-specific production policy. | `skip` |
| `.github/workflows/gemini-dispatch.yml`, `.github/workflows/gemini-invoke.yml`, `.github/workflows/gemini-plan-execute.yml`, `.github/workflows/gemini-review.yml`, `.github/workflows/gemini-scheduled-triage.yml`, `.github/workflows/gemini-triage.yml` | GitHub automation around `atm-core`'s Gemini dispatch and review flow. | `atm-core` orchestration, not `sc-lint` product behavior. | `skip` |

## Recommended Near-Term Imports

1. Import `.claude/skills/codex-orchestration/sprint-plan.md.j2` so future
   sprint docs are generated from the same structured shape already used in
   `atm-core`.
2. Import `scripts/test_find_todos.py` to put the already-used TODO-triage
   helper under test.
3. Import `scripts/test_triage_carry_forward.py` to put the already-used
   carry-forward triage helper under test.

## Adapt-Only Items

- The `.just/` wrappers around `line-counts`, identity literals, and Unix
  gating are useful examples for consumer repos adopting `sc-lint`, but they
  should not be copied into the tool repo verbatim because their reporting and
  config assumptions are consumer-specific.

## Skip Items

- `atm-core`'s view-site wrappers and Gemini workflows are active there, but
  they are tied to `atm-core` runtime surfaces and hosting automation rather
  than missing `sc-lint` product capability.
