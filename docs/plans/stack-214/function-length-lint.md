---
sprint: stack-214-L4
bead: lint-8qz.5
epic: lint-8qz
status: complete
branch: feature/function-length-lint-v2
worktree: /Users/randlee/github/sc-lint-worktrees/feature/function-length-lint-v2
pr_target: fix/qa-template-strict-xml
closure_type: contract
authoritative: this assignment (Rand PR shape rule, bd memory pr-shape-rule)
---

# stack-214-L4 — function-length lint

Gate: functions at or over 80 code lines fail only when new or grown past
`.just/function-length-baseline.tsv`. Functions at 70 to 79 are advisories.
Existing over-cap functions are recorded, not refactored. Regenerate with
`python -m sc_lint.lint_function_length --write-baseline`.

Recorded growth: `workflow.rs:lint_profile_plan` 97 → 103, from adding the
function-length step to the Full profile. This is a decision for team-lead.
