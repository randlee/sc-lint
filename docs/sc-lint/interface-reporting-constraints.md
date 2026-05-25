# Interface Reporting Constraints

This document records the repo-local planning constraints for Phase `C`
published interface reporting.

## Report Package Shape

- published interface reports are generated from structured data and reusable
  templates
- the package contains:
  - one self-contained main HTML report
  - one JSON sidecar as the canonical machine-readable source of truth
  - separate XHTML section fragments/panels for deeper per-section context

## Rendering Workflow

- the rendering path uses one reusable HTML-report workflow rather than
  repo-specific ad hoc HTML generation
- the reusable workflow is the shared `html-report` skill used by the Phase
  `C` planning line for generated interface reports
- the workflow must keep report meaning in structured inputs and templates
  instead of agent-authored prose patches
- the JSON sidecar remains authoritative for change detection; HTML and XHTML
  surfaces are human-facing derivatives

## Panel Copy Actions

- every XHTML section fragment/panel exposes built-in copy actions for:
  - canonical section JSON payload
  - canonical section context text
- copy controls belong to the reusable template stack rather than
  per-report custom scripting

## Non-Goals

- hand-written monolithic HTML documents
- per-report custom JavaScript beyond minimal reusable copy support
