# Intent - phase-m (v3)

Drafted 2026-10-30 for Rand's review. It frames every Phase M anchor. Jev
judges plan items against the anchors, not this document.

## Governing direction

Rand (2026-10-30): "I think a lot of sc-lint issues come from attempts to
mandate how the sc-lint tools are orchestrated where the focus should be aimed
at creation of tools that provide consistent/flexible consumer configuration."

Rand (2026-10-30): "atm-core should be able to delete its workarounds: when
sc-lint is done, every item on its P1 list passes as the GitHub issue
describes, with nothing hidden."

Rand (2026-10-30), on the anchors: "the anchors must be derived from the
req/adr and the issues we have failed to fix in 2-days of continuous work"
and "anchors need to be based on requirements".

ADR-022 records the direction. sc-lint provides tools and a consistent
consumer configuration surface. The consumer owns orchestration. Rand
(2026-10-30), on ADR-022: "This is a positive step.  This should guide both
anchors and decisions about how atm-core fixes are handled.  Proceed". Its
formal Status stays Pending until Rand accepts the text.

## What this plan is for

Resolve atm-core's P1 issues #786, #720, #198 and #136 by improving the tools
and the configuration surfaces ADR-022 names, not by prescribing how
consumers run sc-lint.

## What success looks like

- **#786 and #720.** PORT-010 recognises S3 same-name siblings and S6 Unix-only modules
  under the approved 2026-10-31 cut. Other shapes use reasoned site allowances;
  unallowed real parity gaps still report.
- **#786 other shapes, including S8.** These use a per-site
  `#[sc_lint(allow(PORT-010), reason = "...")]` (ADR-022 decision 4).
- **#198.** Lint steps accept `env` and `cwd` exactly as test command
  selectors do (ADR-016 "per-step/per-layer `env`"; ADR-022 decision 3.2). No
  setup hook or interpreter-selection key is added (ADR-022 decision 6).
- **#136.** SCB-MANIFEST-004 does not report a `self::`, `super::` or
  `crate::` path, or a local `mod`, named like an owned dependency, because
  that is not "a configured ownership violation" (REQ-SCB-019). Owner `tests/`
  roots in `allowed_source_roots` are documented as the supported
  configuration.
- **#385 and #195.** m-2 adds SCB-CYCLE-001 path filtering; m-3 preserves
  both failed-step streams and writes every step output under the logger
  root's steps/ directory.
- **Every sprint** traces to its issue and to the REQ or ADR it implements,
  and carries a deterministic regression for atm-core's reported case.

## Out of scope

- Blanket suppression: disabling a rule, a repository-wide exclusion, or
  widening allowed roots beyond the owner's own targets.
- New orchestration in the product: setup hooks, interpreter selection, or
  required recipe or CI shapes.
- atm-core gaps not on the P1 list (#783, #784), the 0.8.3
  release-platform work, and process or tooling changes.
- The rest of ADR-022, such as the shared unknown-key check and required
  `reason`. It becomes later work unless a P1 item needs it.

## Anchors

Anchors live in `anchors.json` beside this file (v3; v2 is superseded). Each
quotes a REQ or ADR sentence, together with the issue evidence that shows the
gap. Anchors that quote ADR-022 rest on Rand's direction above.
