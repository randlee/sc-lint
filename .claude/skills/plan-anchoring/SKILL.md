---
name: plan-anchoring
version: 0.3.0
description: Capture the facts a user states while planning as numbered anchors (ANC-001...), one clear, testable requirement each, with the user's verbatim words kept as provenance, alongside existing requirements (REQ) and ADRs. Turn them into Jev-judged questions, tune and test the questions per anchor, and enforce them at two gates with a triage-apply-rerun loop. The planning gate checks each plan bead still says what the user said; the development gate checks the delivered sprint work followed it. Use when planning a phase or sprint with beads, when the user says "anchor this", "lock this", "don't let this drift", or "remember I said", when re-opening a plan after context compression, or when reviewing a sprint against its plan. Not for general code review, and not a replacement for plan hardening.
---

# Plan Anchoring

Facts stated during planning get compressed out of context, ceremony accumulates, unapproved work creeps in, and plans and code drift from what was said. This skill makes each stated fact durable: the user's words are kept verbatim; the orchestrator writes each as one testable requirement the user approves; the requirement becomes orthogonal Jev questions, tuned per anchor; and the plan and the delivered work are judged against it in a loop of run, triage, fix, re-run.

This file is the table of contents; read the reference for the step you are on.

## Step 1 - Verify dependencies

Run before any capture, delegation, or gate. On a missing dependency, surface the gap and stop.

```bash
which python3 && python3 --version          # 3.9 or newer
which bd && bd --version                    # beads CLI, 1.x ("(dev)" suffix is fine)
test -n "$TYPESAFE_API_KEY" && echo "TYPESAFE_API_KEY set" || echo "TYPESAFE_API_KEY missing"
echo "remote judge opt-in: ${PLAN_ANCHOR_ALLOW_REMOTE_JUDGE:-unset}"
```

If `bd` is not on PATH, Claude Code's bash may not share your shell's PATH. Check common locations:

```bash
for p in "/opt/homebrew/bin/bd" "$HOME/go/bin/bd" "$HOME/.local/bin/bd" "/usr/local/bin/bd"; do
  [ -x "$p" ] && echo "Found at: $p" && break
done
```

If found off PATH, `export PATH="<dir>:$PATH"` for the session. If anything is missing, read `references/installation-and-troubleshooting.md`. Capture and locking (steps A and B) need only `python3` and `bd`; the gates (C and D) also need the API key and the opt-in in `references/jev-api.md`, because they send anchor statements and item text to TypeSafe.

Then resolve the script once. It sits next to this file:

```bash
S="<this skill's directory>/scripts/jev_judge.py"   # absolute; as a plugin: $CLAUDE_PLUGIN_ROOT/skills/plan-anchoring/scripts/jev_judge.py
A=doc/plan/phase-current/anchors.json
python3 "$S" --help
```

Every command below uses `$S` and `$A`; every agent invocation passes `$S` as `script`.

## Concepts

| Concept | ID | What it is | Where it comes from |
| --- | --- | --- | --- |
| Intent | none | One-page "why" for the plan; frames every question, never judged | Written during planning |
| Anchor | ANC-NNN | One testable requirement written from something the user said; the exact words are kept as its `quote` | The user, via the orchestrator, approved by the user |
| Requirement | REQ-* | A numbered technical requirement | Existing register |
| ADR | ADR-* | A numbered architectural decision record | Existing register |

All three ID'd kinds become Jev questions and are judged the same way. Details: `references/anchor-document.md`.

## Layout

```
doc/plan/phase-current/
├── intent.md            # the overview
├── anchors.json         # record of truth: every anchor with provenance, status, beads, fixtures
├── anchors.md           # rendered from anchors.json; never hand-edited
└── anchor-runs/         # one JSON per gate run, dry runs included; versioned with the plan for diffs
```

Beads hold the questions: one anchor bead per fact, with a plan-questions child and a dev-questions child (`references/bead-layout.md`).

## Workflow

### A. Capture anchors while planning (main session, interactive)

Read `references/anchor-capture.md`. Watch for directives about how the work must be done; a phase needs five to fifteen, not one per sentence. For each candidate, write one testable requirement (`statement`) and keep the user's exact words as the `quote`. Before proposing, run `python3 "$S" rubric --statement "<candidate>" --quote "<words>" --scope "<scope>"`: it lints and asks Jev whether the statement is one constraint, observable, in requirement form, faithful to the quote, scoped, and positive. Fix anything `needs_work`. Propose both texts together, check `anchors.md` for duplicates, and record only when the user confirms or says "lock this". The statement is never the quote; `add` refuses that, and `add --rubric` refuses a statement the rubric rejects.

### B. Lock and record

```bash
mkdir -p doc/plan/phase-current
[ -f doc/plan/phase-current/intent.md ] || cp "$(dirname "$S")/../assets/templates/intent.md" doc/plan/phase-current/intent.md
python3 "$S" add --anchors "$A" --quote "<the user's exact words>" \
  --statement "<one testable requirement: subject, must/must not, action, scope>" \
  --kind directive|prohibition|structural --stated-by "<name>" --context "planning session" --summary "<short label>" \
  --scope "<bead-level signals: adds a CLI subcommand, edits Cargo.toml, touches crates/x>" \
  --deterministic-check "<diff, git log, grep or test that proves it literally>" --status locked   # first add creates anchors.json
  # add runs the testability lint and refuses near-duplicates (ANCHOR.DUPLICATE_CANDIDATE); see references/anchor-capture.md
python3 "$S" update --anchors "$A" --id ANC-001 --fixture-pass "<a sentence a bead would say if it satisfied this>" \
  --fixture-fail "<a sentence that would violate it>"                                              # draft, then get the user's approval
python3 "$S" validate --anchors "$A" && python3 "$S" render --anchors "$A"
```

Then create the bead tree and record its IDs with `set-beads` (`references/bead-layout.md`). Requirements and ADRs governing this plan are added the same way with `--source-type REQ|ADR --source-ref <ID>`; they keep their register ID. Fill in `intent.md` if just created. Before the first gate run, every locked anchor must be `lint` clean, have approved fixtures, and pass `check-fixtures` with the default questions; otherwise tune first (B2). Periodically run `dedupe --anchors "$A"` and fold duplicates (`retire --replaced-by`).

### B2. Tune and test the questions per anchor

Read `references/question-tuning.md`. Each anchor's plan-questions and dev-questions beads start at `{}`; the generic defaults must be proven adequate per anchor. As soon as an anchor is locked with approved fixtures, spin off `anchor-question-tune` in the background (one per anchor and gate, up to three at once) with the anchor, the gate, and any known-good and known-bad beads the user can name. It measures the current wording against fixtures and known beads, rewrites one misjudging question per round, re-measures, and returns a tested override with its acceptance table, or the finding that the statement is the problem. On return: show the user the table, write the override into the question bead (`bd update <bead> --body-file`), and post the acceptance as a comment there. On a statement problem: `update --statement`, re-run the rubric, tune again. Do not run the planning gate on an anchor whose questions have not passed this step. Manual tuning follows the same procedure.

### C. Planning gate

Read `references/planning-gate.md` and `references/triage-and-iteration.md`. This is a loop:

1. **Run.** Enumerate the planning beads (`bd children <epic> --json`) and delegate each to `anchor-plan-review` (see Agent Delegation). It composes the bead's title, description and acceptance criteria (design is measured by `shape`, not judged), resolves overrides from the plan-questions beads, runs the script once (one Jev request per anchor; retries and near-threshold confirmation inside), and returns verdicts with each cell's `leaning`. Post each report on its bead.
2. **Triage** every failing verdict: re-read the sent text, then decide whether it is a plan defect (the bead), an anchor defect (statement or scope), a question defect (wording), or a Jev error. Only the first is fixed in the plan. `PLAN.TOO_LARGE` is a symptom: run `shape`, remove unapproved work, cut prose and ceremony, and split only what is still too large (`references/plan-size-and-shape.md`). Never split or trim to pass the gate.
3. **Apply** one class of fix per iteration: edit the bead, or `update --statement/--scope` and re-test fixtures, or tune the question bead and re-run `check-fixtures`.
4. **Re-run** only the affected beads, `diff` each against its previous run, and recompute coverage in code, never by asking Jev:

```bash
python3 "$S" coverage --runs doc/plan/phase-current/anchor-runs --gate plan --format md
```

5. **Stop** when every bead's latest run has no `contradicted`, `not_addressed`, `underspecified` or `error` verdict (the script exits 1 otherwise) and every locked anchor is `satisfied` by at least one bead; or on a hard stop: three identical failing runs of one bead, a scope disagreement only the user can settle, or an anchor reworded three times. If Jev looks wrong and no defect applies, say so in the posted report and ask the user. Question design: `references/plan-anchor-questions.md`. Verdict logic: `references/verdict-gating.md`.

### D. Development gate (sprint review)

Read `references/development-gate.md` and `references/triage-and-iteration.md`. At sprint delivery, the quality manager delegates to `anchor-sprint-review` with the sprint's epic bead ID and the evidence (sprint report, PR number or body, test summary). It runs the development question set, whose evidence question separates "we followed it" from "here is where", and returns the deterministic checks to pair with each `satisfied` or `asserted_only` anchor. Run and record those checks; Jev judges text, and only a diff or git history proves the code. Then the loop of C: triage each failing verdict (work defect, report defect, anchor defect, question defect, Jev error), fix where the defect is, re-run, diff. Question design: `references/code-review-questions.md`; dev-gate wording is tuned in the dev-questions bead.

### E. Report

Reports quote the user's own words back (`references/report-format.md`). Post the markdown from `report --run <run> --format md` as a comment on the judged bead (`bd comment <id> --file <report.md>`) or on the PR. Posting happens in the main session; agents never write to beads.

### F. Maintain

`python3 "$S" promote --anchors "$A" --id ANC-012 --adr ADR-0007` promotes a durable anchor, keeping the ID. A changed directive is a new anchor; retire the old with `python3 "$S" retire --anchors "$A" --id ANC-012 --replaced-by ANC-020 --reason "<why>"`. Scope, summary, fixtures and deterministic checks change through `update`. Re-render after every change. Lifecycle: `references/anchor-document.md`.

## Agent Delegation

Capture (A) and locking (B) stay in the main session. Question tuning (B2) and the gates (C, D) run in isolated agents that return fenced JSON; the tuner runs in the background. The skill presents results and makes every write to beads and the record.

| Operation | Agent | Inputs | Returns |
| --- | --- | --- | --- |
| Planning gate, one bead | `anchor-plan-review` | `script`, `bead_id`, `anchor_ids` or `all`, `anchors_path`, optional `run_dir`, `threshold`, `dry_run`, `correlation_id` | fenced JSON: run path, pass flag, per-anchor verdicts, report markdown |
| Development gate, one sprint | `anchor-sprint-review` | `script`, `sprint_id`, `evidence` paths or `pr_number`, `anchor_ids` or `all`, `anchors_path`, optional `run_dir`, `threshold`, `dry_run`, `correlation_id` | fenced JSON: run path, pass flag, per-anchor verdicts, deterministic checks to pair, report markdown |
| Question tuning, one anchor and gate (background) | `anchor-question-tune` | `script`, `anchors_path`, `anchor_id`, `gate`, optional `known_good`, `known_bad`, `current_override`, `max_rounds`, `threshold`, `correlation_id` | fenced JSON: accepted flag, tested override JSON, baseline and final acceptance table, run IDs, or the statement problem |

In Claude Code, invoke with the Task tool: "Invoke `anchor-plan-review` with script `$S`, bead_id proj-a3f8, anchor_ids all, anchors_path doc/plan/phase-current/anchors.json, correlation_id proj-a3f8." Without agents (Codex, Cursor), follow `../../agents/anchor-plan-review.md` or `../../agents/anchor-sprint-review.md` (relative to this skill's directory) yourself.

Parallel runs (Claude Code only): at most 3 agents at once, `correlation_id` set to the bead ID, 120 s timeout per agent. A timed-out agent is recorded as `{"success": false, "canceled": true, "aborted_by": "timeout", "error": {"code": "EXECUTION.TIMEOUT"}}`. Aggregate in bead-ID order and summarise succeeded and failed beads; never drop a failed bead. Unfenced or malformed JSON from an agent is a failure. Agents never retry the script; it already makes three attempts per anchor and shortens oversize items itself.

## Design positions

- **Granularity**: one testable constraint per anchor; a compound statement becomes several anchors, each quoting the sentence carrying its constraint.
- **Capture**: the orchestrator proposes candidates it detects; nothing is locked until the user confirms or says "lock this".
- **Fast-model cross-check**: not in this version. Jev's per-question confidence drives the `underspecified` verdict instead.
- **Relationship to plan hardening**: standalone and composable; run the planning gate after hardening.
- **Data retention**: fail closed. The gates refuse to call TypeSafe unless `PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1` is set for the session, and `--dry-run` writes exactly what would leave the machine.
- **Fixture authorship**: the skill drafts one pass and one fail fixture per anchor at lock time; the user approves them. `check-fixtures` sends them to Jev (same opt-in) to confirm the wording lands them; `--answers-file` fixtures exercise the gating offline in CI.

## Hard stops

- An anchor without a verbatim `quote` has no provenance; one whose statement is just the quote is not testable. Both are refused.
- Never tune a question or reword a statement to make one bead pass. Fixtures and a known-bad bead must still fail.
- Never split a bead, trim its text, or raise a size cap to pass `PLAN.TOO_LARGE`. Triage unapproved work, then prose and ceremony, then split.
- Never send more than one bead or one work description per Jev request, and never the whole plan.
- Never compute coverage, counts, or "satisfied anywhere" by asking Jev. The script does it.
- Missing `bd`, `python3`, API key, or opt-in: surface and stop; do not degrade silently.
- Three consecutive gate runs on the same bead with the same failing set: escalate to the user rather than editing the plan again.

## References

- `references/anchor-capture.md` - detecting, proposing, confirming, and wording anchors
- `references/anchor-document.md` - intent.md, anchors.json schema, anchors.md, IDs, lifecycle
- `references/bead-layout.md` - the anchor bead tree, bd commands, labels, overrides in beads, reading beads
- `references/question-tuning.md` - the testability rubric, per-anchor question overrides, proving the wording
- `references/triage-and-iteration.md` - the run, triage, apply, re-run, diff loop and the stop conditions
- `references/plan-size-and-shape.md` - oversized beads and phases: triage order, what Jev reads, `shape` metrics
- `references/measured-behaviour.md` - the numbers behind every rule
- `references/planning-gate.md` - inputs, run, exit codes, reading results
- `references/development-gate.md` - inputs, evidence, run, exit codes, reading results
- `references/plan-anchor-questions.md` - the planning-gate question set and why it is orthogonal
- `references/code-review-questions.md` - the development-gate question set and its evidence
- `references/verdict-gating.md` - answers to verdicts, thresholds, coverage, diffs
- `references/report-format.md` - the provenance-quoting report and what to do with each verdict
- `references/jev-api.md` - endpoint, request and response, limits, retries, privacy opt-in, every flag and error code
- `references/installation-and-troubleshooting.md` - installing python3, bd, and the API key; PATH; Codex
- `assets/questions/plan.json`, `assets/questions/dev.json`, `assets/questions/testability.json`, `assets/questions/dedupe.json` - the question wording of record
- `assets/templates/intent.md`, `assets/templates/anchor-questions-bead.md`, `assets/examples/` - starting points and a worked example
- `tests/test_jev_judge.py` - offline test suite; `python3 -m unittest discover -s <skill dir>/tests`
