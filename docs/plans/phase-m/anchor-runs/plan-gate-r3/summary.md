# Phase M planning gate, round 3 (lint-0m30)

Run 2026-10-30 by flint at plan head 9d1e996a, after clint's lint-q397 repairs. Anchors are unchanged since round 2.
- **Beads:** lint-m-1, m-3, m-5a and m-5b only. m-2 and m-4 are held at two identical failing sets until Rand rules on the ANC-025/027/029/031 scopes; their latest runs are r2.
- **Procedure:** same as rounds 1 and 2 — inline with the opt-in per command, one anchor and one bead per Jev request, the 15 tuned anchors with overrides applied.
- **Not yet gated:** ANC-018, ANC-020 and ANC-021.

Model `jev-1.14.0`, threshold 0.6.

**Result: no bead passes. Using the latest run per bead (r3 for these four, r2 for m-2 and m-4), only ANC-024 is uncovered.** The four beads have 8 failing cells (r2: 11 on the same beads).

## Verdicts (r3; `*` = flipped between failing and passing since r2)

| Bead | 014 | 016 | 019 | 022 | 023 | 024 | 025 | 026 | 027 | 028 | 029 | 030 | 031 | 032 | 033 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lint-m-1 | NA! | na | na | na | na | na | na | U!* | na | na | na | na | na | na | na |
| lint-m-3 | na | na | S | S | S | U! | na | na* | S | S | na | na | U! | S | S |
| lint-m-5a | NA! | na | na | na | na | na | na | S | na | na | na | na | na | na | na |
| lint-m-5b | S | na | na | na | na | na | na | U! | U! | na | na | na* | U! | na | na |

## Flipped since r2

- **Now passing:**
  - m-3 · ANC-026 (U → na).
  - m-5b · ANC-030 (U → na), after "do not infer intention from names" and "similarly named" were limited to platform_companions declarations.
- **Newly failing: m-1 · ANC-026 (S → U, leaning satisfied, min 0.52).** relevant yes 1.0 / addressed yes 0.83 / contradicted no 0.52. m-1's only change was rewording the ANC-014 line ("This layer removes the six S3 occurrences; m-5b owns final 47-item closure"). The ANC-026 sentences are byte-identical to r2, where they were satisfied. This is run-to-run variation on unchanged text, not a wording regression; the low answer is contradicted at 0.52.

## Growth check

- **m-3 (+9):** adds the inline "ANC-023:" citation that r2 lacked. No flip.
- **m-5b (+144):** the ANC-030 flip came with growth. The added passages limit two sentences to platform_companions declarations and add an "ANC-014 final acceptance" trace line. They cite or narrow to locked anchors (ANC-014, ANC-026); no new scope.
- **m-1 and m-5a** shrank.

## Awaiting Rand

| Bead · anchor | Verdict (leaning, min) | Reason |
|---|---|---|
| m-3 · ANC-024 | underspecified (leaning **contradicted**, 0.26) | ANC-022 / ANC-024 tension: m-3 moves sc_lint.* helpers formerly run via _py to the kit default. Persists r1 → r3. |
| m-3 · ANC-031 | underspecified (not_addressed, 0.42) | ANC-031 scope; persists r1 → r3 |
| m-5b · ANC-027 | underspecified (not_addressed, 0.55) | ANC-027 scope; persists r1 → r3 |
| m-5b · ANC-031 | underspecified (not_applicable, 0.35) | ANC-031 scope; persists r1 → r3 |

## Other failing cells

| Bead · anchor | Verdict (leaning, min) | Answers rel / addr / contra | History | Note |
|---|---|---|---|---|
| m-1 · ANC-014 | not_addressed (0.83) | yes 1.0 / no 0.97 / no 0.83 | r1, r2, r3 | Third failure. The "layer" wording was reworded again; Jev still reads ANC-014 (an end-state outcome) as not carried by a layer bead. Treat it as a trace or question decision, not more rewording. |
| m-5a · ANC-014 | not_addressed (0.49) | yes 1.0 / no 1.0 / no 0.49 | r1, r2, r3 | Same. |
| m-5b · ANC-026 | underspecified (leaning **contradicted**, 0.34) | yes 0.98 / yes 0.79 / yes 0.34 | r1, r2, r3 | The targeted repair (limiting the name sentences to declarations) did not clear it; contradicted fell from 0.46 to 0.34. The remaining contradiction cue is not obvious from the text. Tuning, or Jev's per-question explanation, would help more than another reword. |
| m-1 · ANC-026 | underspecified (leaning satisfied, 0.52) | yes 1.0 / yes 0.83 / no 0.52 | new | Variation on unchanged text (above). |

## Identical failing sets (3-identical rule, per bead)

| Bead | r1 | r2 | r3 | Status |
|---|---|---|---|---|
| lint-m-1 | 014, 026 | 014 | 014, 026 | differs from r2 |
| lint-m-3 | 023, 024, 031 | 024, 026, 031 | 024, 031 | differs |
| lint-m-5a | 014, 026 | 014 | **014** | **identical to r2** (r1 differs). A fourth run with the same set is the third identical run: hard stop. |
| lint-m-5b | 016, 026, 027, 031 | 026, 027, 030, 031 | 026, 027, 031 | differs |
| lint-m-2, m-4 | — | — | not run | held at 2 identical sets |

## Low-confidence diagnostics (lint-e9jk)

Every cell with min confidence below 0.6 is a failing cell above. All satisfied/not_applicable cells in r3 have every role at 0.6 or above. No bead passes, so there is no production PASS.

## Coverage (script; latest run per bead: r3 for m-1/m-3/m-5a/m-5b, r2 for m-2/m-4)

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
| ANC-026 | underspecified | not applicable | not applicable | not applicable | satisfied | underspecified | yes |
| ANC-027 | not applicable | not addressed | satisfied | underspecified | not applicable | underspecified | yes |
| ANC-028 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-029 | not applicable | satisfied | not applicable | not addressed | not applicable | not applicable | yes |
| ANC-030 | not applicable | not applicable | not applicable | satisfied | not applicable | not applicable | yes |
| ANC-031 | not applicable | not addressed | underspecified | satisfied | not applicable | underspecified | yes |
| ANC-032 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-033 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |

Uncovered anchors (no item satisfies them): ANC-024

