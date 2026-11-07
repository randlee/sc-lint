# Intent: phase-p (0.8.7)

## Purpose

Establish a firm file-ownership contract between sc-lint and its consumers, so that upgrading sc-lint no longer causes unintentional breaking changes.

> "The purpose of what you are defining is to establish a firm contract that will result in no more unintentional breaking changes .  i.e. the issues they have are all having to do w/ ownership of files." (Rand, 2026-11-07T00:59Z)

> "I do NOT want another round of issues.  The past 3-4 phases end up with a wave of issues from sc-compose/atm-core due to breaking changes from the past release of sc-lint." (Rand, 2026-11-07T00:58Z)

## Success

- Every path sc-lint writes in a consumer is either sc-lint-owned (always overwritten), consumer-owned (created only if missing, never edited) or retired (deleted by `sc-lint init --just`).
- Before 0.8.7 ships, each consumer (atm-core, sc-compose, sc-observability) has a migration guide naming every break and its fix.
- 0.8.7 publishes only after the candidate passes `just setup`, `just lint` and `just test` in real consumers.
- The install path is smaller after phase-p than before.

## Out of scope

- o-17 (#761) and o-18 (#771): deferred, not labelled 0.8.7.
- Retiring sc-publish's `setup-sc-lint` action, which is sc-publish's own work.
- Any consumer-specific rules in the sc-lint engine (ADR-016).

## Sources

- `~/phase-o-harvest/REQUIREMENTS-DIGEST.md`: Rand's rulings, verified verbatim against the transcripts.
- ADR-023, REQ-PRODUCT-025 and ADR-012. These must be amended to match the anchors before any sprint starts.
