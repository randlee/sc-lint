## Anchor gate: plan · lint-m-3 - m-3: lint-step-env-cwd and visible output

Run `plan-20261031T194639Z-lint-m-3`, model `jev-1.14.0`, threshold 0.6. Counts: underspecified 1, satisfied 1, not_applicable 6.

> **lint-m-3 · ANC-012 · underspecified** Anchor: sc-lint must not add a setup-hook or interpreter-selection configuration key, such as [tool.sc-lint].setup.pre or [tool.sc-lint].runner.python. You said (Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P12, Oct 30): "ADR-022 (Rand 2026-10-30: "This is a positive step.  This should guide both anchors and decisions about how atm-core fixes are handled.  Proceed"), decision 6: "sc-lint does not add setup hooks or interpreter-selection policy. These are consumer orchestration. A consumer expresses them with step argv, step `env` and its own recipes."

ADR-020 decision 9 (Accepted): "**#198 (setup hook).** Won't-do. ADR-016's hook restriction is unchanged; `setup.pre` and `runner.python` are not added."." Jev was not confident (min confidence 0.49); the anchor or the bead may be underspecified. (leaning: satisfied)
>   Low: relevance `relevant` answered yes at 0.49 (second ask).
>   Unstable: two identical requests gave satisfied then underspecified.
