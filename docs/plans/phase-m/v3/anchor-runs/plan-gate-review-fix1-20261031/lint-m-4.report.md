## Anchor gate: plan · lint-m-4 - m-4: port010-site-allow

Run `plan-20261031T194639Z-lint-m-4`, model `jev-1.14.0`, threshold 0.6. Counts: underspecified 1, satisfied 1, not_applicable 6.

> **lint-m-4 · ANC-009 · underspecified** Anchor: sc-lint's PORT-010 (sc-lint's rule that reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) changes must not blanket-suppress real parity gaps: a production #[cfg(unix)] item with no #[cfg(windows)] companion and no portable fallback must still be reported. You said (Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P9, Oct 30): "REQ-PRODUCT-004AA/PORT-010 (docs/requirements.md): "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

ADR-010 decision 6 (Accepted): "Explicit `#[cfg(unix)]` production boundaries are treated as structural parity questions for the shared portability family. Leaf rules may suppress inside those items, with the companion-or-fallback decision owned by the structural parity rule family instead of parallel per-rule allowlists."

Issue #786 (atm-core verdict table, 2026-10-30): "#720 covers only the crate-root main.rs shape; this issue tracks the complete remaining inventory and asks for per-case classification, not blanket suppression of real parity gaps."." Jev was not confident (min confidence 0.34); the anchor or the bead may be underspecified. (leaning: satisfied)
>   Low: conflict `contradicted` answered no at 0.34.
