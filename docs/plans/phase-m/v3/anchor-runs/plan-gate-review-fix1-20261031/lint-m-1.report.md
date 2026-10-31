## Anchor gate: plan · lint-m-1 - m-1: port010-same-name-siblings

Run `plan-20261031T194639Z-lint-m-1`, model `jev-1.14.0`, threshold 0.6. Counts: underspecified 2, satisfied 3, not_applicable 3.

> **lint-m-1 · ANC-006 · underspecified** Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report the declaration #[cfg(unix)] mod x; of a module that is never compiled on Windows. You said (Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P6d, Oct 30): "Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S6  Module declared `#[cfg(unix)] mod x;` and items inside it also carry cfg(unix); module is never compiled on Windows. Rows 13 (the decl), 17."

REQ-PRODUCT-004AA (docs/requirements.md), rule list: "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"." Jev was not confident (min confidence 0.55); the anchor or the bead may be underspecified. (leaning: satisfied)
>   Low: conflict `contradicted` answered no at 0.55.

> **lint-m-1 · ANC-010 · underspecified** Anchor: PORT-010, a rule sc-lint's portability analyzer reports from source, must be allowable at a single site: an item carrying #[sc_lint(allow(PORT-010), reason = "...")] is not reported by PORT-010. You said (Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P10c, Oct 30): "ADR-022 (Rand 2026-10-30: "This is a positive step.  This should guide both anchors and decisions about how atm-core fixes are handled.  Proceed"), decision 4: "Every rule an analyzer reports from source can be allowed at a single site with `#[sc_lint(allow(<RULE>), reason = \"...\")]`. A site allowance is the approved exception path for a deliberate, non-parity case such as #786 S8."." Jev was not confident (min confidence 0.34); the anchor or the bead may be underspecified. (leaning: not_addressed)
>   Low: relevance `relevant` answered yes at 0.34.
>   Low: compliance `addressed` answered no at 0.58.
