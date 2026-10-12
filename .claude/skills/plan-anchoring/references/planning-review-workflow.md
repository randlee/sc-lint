---
version: 0.4.4
---

# Planning-review workflow

### C. Planning gate

Read `references/plan-source-contract.md`, `references/planning-gate.md`, and
`references/triage-and-iteration.md`. Materialize one complete sprint item at a
time from an authoritative Markdown phase file/directory or from an exact
`bd`/`br` epic copy; never concatenate the phase index, epic, intent, or another
sprint into the item. When both sources exist, verify copy parity before Jev is
called. Jev's feedback is the measure of plan quality, and this loop is how the
planner raises it: run Jev, fix the plan, run again, until every item passes. A
failing verdict or a low confidence means the item is unclear, excessive or
missing something, and the planner acts on it; it is never explained away. The
usual fix is to simplify: cut work no anchor asks for, cut prose and ceremony,
and state what remains plainly. Add text only where Jev found something missing.

1. **Run.** Enumerate the planning beads (`bd children <epic> --json`) and delegate each to `anchor-plan-review` (see Agent Delegation). It composes the bead's title, description and acceptance criteria (design is measured by `shape`, not judged), resolves overrides from the plan-questions beads, runs the script once (one Jev request per anchor; retries and near-threshold confirmation inside), and returns Jev's verdicts unchanged, with `to_fix` naming the weak sentence and change for every failing cell. Post each report on its bead.
2. **Triage** every failing verdict: re-read the sent text, then decide whether it is a plan defect (the bead), an anchor defect (statement or scope), a question defect (wording), or a Jev error. Before editing, record `to_fix` with the anchor ID, the exact weak or conflicting sentence, and one action (`cut`, `replace`, or `add`). Prefer cuts, then replacement. Adding text is allowed only for content required by a locked ANC/REQ/ADR that predates the failed run; cite that ID and its source. A reason written after the failure does not create scope. Only a plan defect is fixed in the plan. `PLAN.TOO_LARGE` is a symptom: run `shape`, remove unapproved work, cut prose and ceremony, and split only what is still too large (`references/plan-size-and-shape.md`). Never split or trim merely to pass the gate.
3. **Apply** one class of fix per iteration: edit the bead, or `update --statement/--scope` and re-test fixtures, or tune the question bead and re-run `check-fixtures`.
4. **Re-run** only the affected beads, `diff` each against its previous run, and recompute coverage in code, never by asking Jev. `diff.item_size` must be reviewed: a failing-to-passing edit that grew is not accepted unless every added passage cites a pre-existing locked ANC/REQ/ADR; otherwise cut or replace instead.

```bash
python3 "$S" coverage --runs doc/plan/phase-current/anchor-runs --gate plan --format md
```

5. **Stop** when every bead's latest run passes and every locked anchor is `satisfied` by at least one bead. Report any allowed growth with its pre-existing authority when planning is complete. A hard stop also ends the loop: three identical failing runs of one bead, or an anchor reworded three times. Question design: `references/plan-anchor-questions.md`. Verdict logic: `references/verdict-gating.md`.

### E. Report

Reports quote the user's own words back (`references/report-format.md`). Post the markdown from `report --run <run> --format md` as a comment on the judged bead (`bd comment <id> --file <report.md>`) or on the PR. Posting happens in the main session; agents never write to beads.

<!-- The workflow text above is preserved verbatim from the 0.4.3 entrypoint. -->
