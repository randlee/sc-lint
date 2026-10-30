# Phase M planning gate, round 2 (lint-c6o9)

Run 2026-10-30 by flint at plan head 47b8f6d7, after clint's lint-09d2 bead repairs. Anchors are unchanged since round 1 (anchors.json identical to 7a67df4f). The procedure is the same as round 1:
- inline, with the opt-in set per command;
- `compose-item --for plan` per container bead;
- one `judge --gate plan` per bead, one anchor and one bead per Jev request;
- the 15 tuned anchors, with tuned overrides applied.

ANC-018, ANC-020 and ANC-021 are not yet gated (pending definition). Model `jev-1.14.0`, threshold 0.6.

**Result: no bead passes. Coverage leaves only ANC-024 uncovered** (r1: ANC-023, ANC-024, ANC-026). There are 14 failing cells (r1: 16). 8 are scope-driven and await Rand's scope approval; 6 are not.

## Verdicts (r2; `*` = flipped between failing and passing since r1)

| Bead | 014 | 016 | 019 | 022 | 023 | 024 | 025 | 026 | 027 | 028 | 029 | 030 | 031 | 032 | 033 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lint-m-1 | NA! | na | na | na | na | na | na | S* | na | na | na | na | na | na | na |
| lint-m-2 | na | na | na | na | na | na | S | na | NA! | na | S | na | NA! | na | na |
| lint-m-3 | na | na | S | S | S* | U! | na | U!* | S | S | na | na | U! | S | S |
| lint-m-4 | na | S | na | na | na | na | NA! | na | U! | na | NA! | S | S | na | na |
| lint-m-5a | NA! | na | na | na | na | na | na | S* | na | na | na | na | na | na | na |
| lint-m-5b | S | na* | na | na | na | na | na | U! | U! | na | na | U!* | U! | na | na |

## Flipped since r1

- **Now passing (4):**
  - m-1 · ANC-026 (U → S)
  - m-3 · ANC-023 (U → S)
  - m-5a · ANC-026 (NA → S)
  - m-5b · ANC-016 (U → na)
- **Newly failing (2), both low-relevance and leaning not_applicable:**
  - m-3 · ANC-026: relevant no@0.58.
  - m-5b · ANC-030: relevant no@0.56; "#786 47-item classification/closure table" may read as the boundary parity table.

## Growth check (`diff.item_size`)

Every flip to passing came with growth (m-1 +287, m-3 +586, m-5a +411, m-5b +240 chars). Per the workflow, growth is accepted only when each added passage cites a pre-existing locked anchor.
- **m-1:** the added passages are the ANC-026 rule restatement and an "ANC-014 layer share" line. Both cite locked anchors. OK.
- **m-5a:** the added passages are the "ANC-014 layer share" and "ANC-026 preservation" lines and a matching acceptance line. They cite locked anchors. OK.
- **m-5b:** the ANC-016 flip came from a replacement ("docs/table" → "#786 47-item classification/closure table"), not added scope. The added ANC-026 preservation lines cite ANC-026. OK.
- **m-3 (review):** the added text is .bootstrap-venv via pre=["just","bootstrap"], _py steps on runner.python, clippy via PATH/PYO3_PYTHON, and helpers on kit default/override. It is ANC-023/ANC-024 content and names ANC-022/ANC-019, but the new setup sentence does not cite ANC-023 inline. The Active Trace carries it. Team-lead to confirm that is enough citation.

## Failing cells awaiting Rand scope approval (ANC-025/027/029/031 scopes unchanged)

| Bead · anchor | Verdict (leaning, min) | r1 |
|---|---|---|
| m-2 · ANC-027 | not_addressed (0.79) | same |
| m-2 · ANC-031 | not_addressed (0.85) | same |
| m-3 · ANC-031 | underspecified (not_addressed, 0.43) | same |
| m-4 · ANC-025 | not_addressed (0.94) | same |
| m-4 · ANC-027 | underspecified (not_applicable, 0.58) | same |
| m-4 · ANC-029 | not_addressed (0.88) | same |
| m-5b · ANC-027 | underspecified (not_addressed, 0.46) | same |
| m-5b · ANC-031 | underspecified (not_addressed, 0.26) | same |

## Other failing cells

| Bead · anchor | Verdict (leaning, min) | Answers rel / addr / contra | r1 | where / change |
|---|---|---|---|---|
| m-1 · ANC-014 | not_addressed (0.89) | yes 1.0 / no 0.89 / no 0.92 | persists | "ANC-014 layer share: the six S3 occurrences produce no PORT-010 finding; final 47-item closure belongs to m-5b". Jev still reads a layer share as not carrying an end-state anchor. The "layer share" fix did not land; dropping ANC-014 from layer traces or a question/scope decision is triage. |
| m-5a · ANC-014 | not_addressed (0.42) | yes 1.0 / no 0.98 / no 0.42 | persists | Same pattern. The 0.42 on contradicted is "S8 public helpers remain reported at this layer". |
| m-3 · ANC-024 | underspecified (leaning **contradicted**, 0.28) | yes 1.0 / no 0.28 / yes 0.55 | persists | "Any sc_lint.* helper invocation formerly reached through those recipes follows the separate kit default (ANC-022) or explicit kit_python override (ANC-019), not runner.python executable selection." The plan now makes explicit that helpers run by _py today move off .bootstrap-venv. That is the ANC-022 / ANC-024 tension: a Rand decision, not wording. |
| m-5b · ANC-026 | underspecified (leaning **contradicted**, 0.46) | yes 0.98 / yes 0.62 / yes 0.46 | persists | Likely "do not infer intention from names or adjacent code" and "Same-file or similarly named unrelated items never become exempt" read against ANC-026's same-name sibling rule. Change: scope those sentences to the m-5b declaration surface (S2/S4/S7/S8), so they don't read as overriding inherited S3 same-name pairing. |
| m-3 · ANC-026 | underspecified (not_applicable, 0.58) | no 0.58 / no 0.96 / no 0.98 | new | Low relevance; m-3 has no PORT-010 work. Noise or scope (ANC-026 scope covers port010 fixtures only). |
| m-5b · ANC-030 | underspecified (not_applicable, 0.56) | no 0.56 / no 1.0 / no 0.92 | new | "the #786 47-item classification/closure table" may evoke the audit parity table. Low relevance; leaning not_applicable. |

## Identical failing sets (3-identical-set rule)

- **m-2 and m-4:** each failed with exactly the same anchor set in r1 and r2. All of those cells are scope-driven (ANC-025/027/029/031). This is the second identical run; a third identical run is a hard stop. Neither bead should be re-run until Rand decides the scopes.
- The other four beads' failing sets changed.

## Low-confidence diagnostics (lint-e9jk)

Every cell with min confidence below 0.6 is a failing cell above. All 76 satisfied/not_applicable cells (of 90) have every role at 0.6 or above. No bead passes, so there is no production PASS.

## Coverage (script)

## Anchor coverage (plan)

| Anchor | lint-m-1 | lint-m-2 | lint-m-3 | lint-m-4 | lint-m-5a | lint-m-5b | Covered |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ANC-014 | not addressed | not applicable | not applicable | not applicable | not addressed | satisfied | yes |
| ANC-016 | not applicable | not applicable | not applicable | satisfied | not applicable | not applicable | yes |
| ANC-019 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-022 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-023 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-024 | not applicable | not applicable | underspecified | not applicable | not applicable | not applicable | no |
| ANC-025 | not applicable | satisfied | not applicable | not addressed | not applicable | not applicable | yes |
| ANC-026 | satisfied | not applicable | underspecified | not applicable | satisfied | underspecified | yes |
| ANC-027 | not applicable | not addressed | satisfied | underspecified | not applicable | underspecified | yes |
| ANC-028 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-029 | not applicable | satisfied | not applicable | not addressed | not applicable | not applicable | yes |
| ANC-030 | not applicable | not applicable | not applicable | satisfied | not applicable | underspecified | yes |
| ANC-031 | not applicable | not addressed | underspecified | satisfied | not applicable | underspecified | yes |
| ANC-032 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-033 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |

Uncovered anchors (no item satisfies them): ANC-024

