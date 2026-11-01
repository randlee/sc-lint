# ADR-021 — Consumer Setup, Python Runner And Cfg Companions (Withdrawn)

> **Withdrawn. Do not implement anything in this record.** It was never
> accepted. [ADR-022](./ADR-022-tools-and-consumer-configuration.md)
> supersedes it. The proposed `setup.pre`, `runner.python`,
> `runner.kit_python`, `runner.export_env`, `platform_companions` and
> `posix_only` surfaces do not exist and must not be built.

| Field | Value |
| --- | --- |
| ID | ADR-021 |
| Status | Withdrawn (never accepted); superseded by [ADR-022](./ADR-022-tools-and-consumer-configuration.md) |
| Deciders | Rand |
| Related | ADR-010, ADR-016, ADR-017, ADR-020; #198, #720, #786 |

## Context

atm-core's [#198 answer](https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954)
commits to separate repository Python, kit Python and cargo/PyO3 execution.
Its 29 repository steps include three test steps; twelve OS-specific profile
entries invoke four wheel-only helpers. Cargo clippy also needs the repository
interpreter in `PYO3_PYTHON` and its executable directory on child `PATH`.
ADR-020 decisions 9/13 reject these settings, and that stands.

The [#786 shape table](https://github.com/randlee/sc-lint/issues/786#issuecomment-6513016641)
pins 47 findings in 18 paths at atm-core `6ffaa97be`. Seventeen paths are
classified false positives; `atm_temp.rs` mixes a false positive with three
deliberate POSIX-only helpers. S3 generalizes #720 beyond the crate root to
module functions and inherent methods. Rand ruled that S8 POSIX-only public
helpers are exempt because they are not a cross-platform parity gap.

## Withdrawal

The proposed decisions were withdrawn on 2026-10-30. They are kept only in
git history.

- **Consumer setup and interpreter selection** are rejected by ADR-022
  decision 6. Consumers keep this orchestration in their own recipes. Lint
  steps gain `env` and `cwd` (ADR-022 decision 3.2; Phase M m-3).
- **PORT-010 companion shapes S1 to S7** are defined as an amendment to
  ADR-010 and planned in Phase M.
- **S8 POSIX-only public helpers** use the per-site
  `#[sc_lint(allow(PORT-010), reason = "...")]` from ADR-022 decision 4
  (Phase M m-4). No `platform_companions` record or `posix_only` kind exists.
