---
sprint: lint-spx.24
bead: lint-spx.24
epic: lint-spx
status: complete
branch: chore/orchestration-verification-fixes
worktree: /Users/randlee/github/sc-lint-worktrees/chore/orchestration-verification-fixes
pr_target: chore/orchestration-script-tests
closure_type: contract
adrs: []
requirements: []
---

# lint-spx.24 — Orchestration stack: mechanics defects from verification lint-spx.6

Layer 4 of stack #172 (#166 <- #167 <- #170), stacked on PR #170 @ ee1bee9.
Source: quality-mgr targeted verification `lint-spx.6`, questions Q1-Q5.
Full report: `atm read --task lint-spx.6 --all`. Every file:line below comes
from that report; re-verify each before editing.

No ADR or requirement governs the orchestration prompts, so `adrs` and
`requirements` are empty by design.

## Deliverables

1. **Q2** — test_orchestration_contracts.py:208-209 skips every key comparison
when atm is absent and CI has no atm -> render with Jinja and compare keys
always; derive req-qa/arch-qa keys from agent docs; delete dead
EXPECTED_JSON_KEYS.
2. **Q1** — triage-record.ttl.j2 has no sample vars and is not
in TEMPLATE_DIRS; task_id supplied-but-ignored in 4 templates; unused notes
(rust-qa) and lead (review-template); SKILL.md templates list omits
ruthless-boundary-qa-assignment and sprint-plan.
3. **Q5** — 'atm gh' does not exist
(quality-mgr.md:182-187); schema-reviewer agent does not exist but is
mandatory (quality-mgr.md:257,272,291) and cites atm-core ADR-061; --watch
used (SKILL.md:278, quality-mgr.md:185) but forbidden by qa-template step j;
'CLAUDE.md section 0' does not exist (SKILL.md:205,209); queue-parallelism
contradiction (qa-template a1 vs review-template d vs quality-mgr.md:54);
atm send --template vs atm task close --template for verdicts (quality-
mgr.md:347 vs :72,200,306); review-template lacks bd claim/close and refused
path; QA-FAIL bead outcome unspecified in templates; AGENTS.md duplicates
its Beads section (72,127); rust-best-practices-agent listed twice in
SKILL.md.
4. **Q3** — validate-findings.py empty dir exits 0 silently;
roster_check.py malformed .atm.toml message lacks file path and missing atm
gives raw Errno; check_dependencies.py ignores argv; tests for each.
Overlaps lint-spx.23 where the line is atm-core residue: .23 owns reviewer
content, this bead owns mechanics.



PARENT
↑ ○ lint-spx: (EPIC) Release 0.6.0 (Phase G adoption kit + boundary fixes + beads-driven orchestration) P1

BLOCKS
← ○ lint-spx.7: Merge orchestration stack to develop P2

Where two documents contradict each other, the rule is: templates are
authoritative for task steps, `SKILL.md` for the contract, agent files for
reviewer behaviour. Fix the non-authoritative side. For the QA queue
contradiction the intended behaviour is: QA tasks for different stacks run in
parallel; tasks on the same stack run in order. For verdict delivery the
intended mechanism is `atm task close --template`. For CI waiting, `--watch`
is forbidden (qa-template step j is right).

## Acceptance criteria

- The contract test compares rendered keys to agent-documented keys with and
  without `atm` on PATH (prove it: run once with `PATH=/usr/bin:/bin`).
- `grep -rn "atm gh\|schema-reviewer\|ADR-061\|CLAUDE.md §0" .claude` returns nothing.
- Every script corner case listed under Q3 has a test, and the test fails
  against the pre-fix script.
- `just lint` and `just test` pass; `git diff --check origin/develop..HEAD` clean.
- CLAUDE.md and AGENTS.md stay mirrored.
- Frontmatter `status: complete` with a `## Closeout`; bead `lint-spx.24`
  claimed with task start and closed with task close.

## Out of scope

- Reviewer rule content, atm-core exemptions and examples, ADR/REQ
  enforcement design (`lint-spx.23`). If a line is both a mechanics defect and
  atm-core residue (e.g. `schema-reviewer`), delete it here and note it in the
  closeout.
- Documentation of ADRs and requirements (`lint-spx.25`).

## Closeout

Completed in the restacked layer: empty-findings validator regression coverage,
actionable roster-check diagnostics, and the Windows-safe malformed-TOML
fixture. The latter fixes the Windows CI failure where an open
`NamedTemporaryFile` produced `Permission denied` rather than TOML parsing.

Cut to `lint-spx.33`: remaining contract-test expansion, dead-command removal,
template/skill contradiction cleanup, and the rest of the verification-script
hardening scope from this plan.

## lint-spx.37 Closeout

- The contract suite compares all seven committed `*.json.j2` templates in
  both ATM and no-ATM Jinja modes; each template has a receiving-agent mapping
  and is compared with the first fenced JSON contract in that agent.
- With the committed `arch-qa-assignment.json` sample,
  `carry_forward_findings_json` has the value `"[]"`; both ATM and the Jinja
  fallback render `"carry_forward_findings": "[]"`. The renderers retain
  different whitespace around Jinja block output.
- `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2` is bytewise
  identical to `atm-core` `origin/develop`.
