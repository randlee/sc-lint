---
id: F.3b
title: Released Wyvern Wizard Capability Qualification
status: planned
target: develop
---

# Sprint F.3b — Released Wyvern Wizard Capability Qualification

## Goal

Qualify a released, checksum-recorded Wyvern artifact against the F.3a handoff
fixtures before sc-lint writes any multi-page UI adapter. This is an external
dependency gate, not a request to recreate wizard behavior in sc-lint.

## Hard Dependencies

- F.3a accepted UX contract and capability matrix;
- Wyvern `v0.2.1` (the first published release after the `v0.2.0` tag) is the
  initial qualification candidate. Its macOS arm64 archive has published SHA-256
  `5cfbc9d67232976036c7406d04486aaf98811821a78b4c93a0b953761404f510` and
  contains `wizard` assets plus the wizard HTTP routes. Wyvern 0.1.0 is known
  insufficient because it ships only blocking `message`, `input`, `markdown`,
  `question`, and `chrome` dialogs.

## Exact Targets

- `docs/sc-lint/configure-wizard-fixtures/wyvern-capability-matrix.md` (new)
- `docs/sc-lint/configure-wizard-fixtures/wyvern-smoke/` (new)

## Deliverables

- Recorded Wyvern version, release checksum, platform artifact, launch command,
  and protocol version for Linux, macOS, and Windows. The starting record is
  `v0.2.1`; a later release may replace it only by rerunning the full matrix.
- Automated headless evidence for forward, back, restored data, changed-branch
  forward-history truncation, first-page back disabled, finish, cancel,
  dismiss, timeout, and full-stack result delivery.
- A capability verdict against every F.3a host requirement. A failed or absent
  feature is a blocking upstream finding with the fixture and expected JSON;
  it must not be emulated by Python.

## Required Host-Protocol Sample

The smoke harness must make the host boundary reviewable as data rather than
by manually clicking a browser. The qualified release must accept a page
descriptor equivalent to this and return the documented terminal state:

```json
{
  "pages": [
    {"id": "overview", "data": {"minimum_version": "0.5.0"}},
    {"id": "baseline", "when": {"pointer": "/start", "equals": "configure"}}
  ],
  "initial_page": "overview"
}
```

The matrix records a result equivalent to `{"status":"finished","stack":[...]}`
and separate stable `cancelled`, `dismissed`, and `timeout` results. It proves
back restores opaque data and a changed branch removes stale forward state; the
harness never compensates for an absent host behavior.

## Acceptance Criteria

- The qualified release passes every F.3a host-capability case on all supported
  platforms and has no network or source-checkout requirement after install.
- A failure reproduces headlessly with an exact fixture and stable expected
  result, suitable for the Wyvern team to fix.
- the capability record names the exact release tag, archive name, SHA-256,
  protocol version, host OS/architecture, and command used for each result;
  an unverified tag or locally built executable is a failure.

## Required Validation

- release checksum verification and `--version` evidence per platform
- headless fixture execution for every matrix row
- `just lint` and `just test`

## This Sprint Does Not Close

- a custom wizard host, page renderer, or state machine in sc-lint;
- agent JSON or the sc-lint page implementation.
