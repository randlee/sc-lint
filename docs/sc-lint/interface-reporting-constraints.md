# Interface Reporting Constraints

This document records the repo-local planning constraints for the shared
reporting line that Phase `C` depends on.

## Report Package Shape

- reports are generated from structured data and reusable templates
- the package contains:
  - one self-contained main HTML report
  - one JSON sidecar as the canonical machine-readable source of truth
  - separate XHTML section fragments/panels for deeper per-section context

## Rendering Workflow

- the rendering path uses one reusable HTML-report workflow rather than
  repo-specific ad hoc HTML generation
- the reusable workflow uses `sc-compose render` and Jinja (`.j2`) templates
  rather than hand-authored HTML code
- the reusable workflow is the shared `html-report` skill used by the Phase
  `C` planning line for generated reports
- the preferred home for that reusable template stack is the `sc-compose` repo,
  potentially as a dedicated `sc-reporting` capability, because the same panel
  pattern should serve lint and non-lint reports
- the workflow must keep report meaning in structured inputs and templates
  instead of agent-authored prose patches
- the JSON sidecar remains authoritative for change detection; HTML and XHTML
  surfaces are human-facing derivatives
- report-template families should stay independent from any one feature and
  must support at least:
  - public API reports
  - CLI contract reports
  - ICD-style RPC/socket reports
  - future non-lint reports such as smoke, integration, DB/query, or
    state-machine outputs
- consumer repos must be able to override selected report templates through
  `sc-lint` config under `[reporting.templates.<report_kind>]`

## Panel Copy Actions

- every XHTML section fragment/panel exposes built-in copy actions for:
  - canonical section JSON payload
  - canonical section context text
- copy controls belong to the reusable template stack rather than
  per-report custom scripting

## Non-Goals

- hand-written monolithic HTML documents
- per-report custom JavaScript beyond minimal reusable copy support
- report rendering logic embedded directly inside `sc-lint-version`
