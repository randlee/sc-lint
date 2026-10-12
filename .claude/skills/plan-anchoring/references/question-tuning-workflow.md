---
version: 0.4.4
---

# Question-tuning workflow

### B2. Tune and test the questions per anchor

Read `references/question-tuning.md`. After the first draft and approved
INTENT/ANC packet exist, launch one named `anchor-tuning` planner subagent
(Claude: Opus; Codex: Sol) and continue authoring concurrently. Render its one
fenced request from `templates/anchor-tuning-tune-request.md.j2`; never supply a
desired plan verdict. The tuner freezes an oracle, evaluates a larger candidate
pool through parallel Jev calls, and returns the smallest semantically exact,
stable role-complete set plus canonical history. Validate its schema and
`anchor_tuning_contract.py` result before writing the selected questions to the
`bd` or `br` question record. Leaf review waits for current protocol seals. On
ambiguous authority, present the tuner's clarity-only recommendation to the
user; the planner never tunes questions itself.

Before this per-anchor tuning, a changed or unsealed testability rubric must be
calibrated by the same Opus/Sol agent with operation `testability`, rendered
from `templates/anchor-tuning-testability-request.md.j2`. Its frozen
plan-independent corpus is the authority: two confident agreeing answers are
required per accepted cell, with a third call only to resolve disagreement.
Requirement form remains deterministic lint and dedupe compares statements,
never shared provenance quotes.

<!-- The workflow text above is preserved verbatim from the 0.4.3 entrypoint. -->
