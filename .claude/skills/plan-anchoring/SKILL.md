---
name: plan-anchoring
version: 0.2.0
description: Capture the facts a user states while planning as numbered, verbatim anchors (ANC-001...) alongside existing requirements (REQ) and ADRs, turn them into Jev-judged questions, and enforce them at two gates. The planning gate checks each plan bead still says what the user said; the development gate checks the delivered sprint work followed it. Use when planning a phase or sprint with beads, when the user says "anchor this", "lock this", "don't let this drift", or "remember I said", when re-opening a plan after context compression, or when reviewing a sprint against its plan. Not for general code review, and not a replacement for plan hardening.
---

# Plan Anchoring

Facts stated during planning get compressed out of context, and plans and code quietly drift from them. This skill makes each stated fact durable: captured verbatim, numbered, turned into orthogonal Jev questions, and judged mechanically at two gates. You state a fact once; the skill holds the plan and the agents to it from then on.

This file is the table of contents. Each aspect has one reference document under `references/`; read the one for the step you are on, not all of them.

## Step 1 - Verify dependencies

Run this before any capture, delegation, or gate. Do not proceed with a missing dependency; surface the gap and stop.

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

If found at a non-PATH location, `export PATH="<dir>:$PATH"` for the session. If anything is missing, read `references/installation-and-troubleshooting.md` before proceeding. Capture and locking (steps A and B) need only `python3` and `bd`; the gates (steps C and D) also need the API key and the explicit opt-in described in `references/jev-api.md`, because they send the anchor statement and the item text to TypeSafe.

Then resolve the script once. It sits next to this file:

```bash
S="<this skill's directory>/scripts/jev_judge.py"   # absolute; as a plugin: $CLAUDE_PLUGIN_ROOT/skills/plan-anchoring/scripts/jev_judge.py
A=doc/plan/phase-current/anchors.json
python3 "$S" --help
```

Every command below uses `$S` and `$A`, and every agent invocation passes `$S` as `script`.

## Concepts

| Concept | ID | What it is | Where it comes from |
| --- | --- | --- | --- |
| Intent | none | One-page "why" for the plan; frames every question but is not judged | Written during planning |
| Anchor | ANC-NNN | A fact the user stated while planning, stored in their exact words | What the user says |
| Requirement | REQ-* | A numbered technical requirement already in the requirement register | Existing register |
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

Beads hold the questions so they live and version with the work: one anchor bead per fact, with a plan-questions child and a dev-questions child (`references/bead-layout.md`). Bead IDs carry the repository's prefix (`proj-a3f8`, not `bd-42`); `bd-42` in these documents is illustrative.

## Workflow

### A. Capture anchors while planning (main session, interactive)

Read `references/anchor-capture.md`. While planning, watch for directives the user states about how the work must be done. When you detect one, propose it as an anchor by quoting their exact words, and record it only when they confirm or when they say "lock this" or "anchor this". Never paraphrase the statement. One testable constraint per anchor.

### B. Lock and record

```bash
mkdir -p doc/plan/phase-current
[ -f doc/plan/phase-current/intent.md ] || cp "$(dirname "$S")/../assets/templates/intent.md" doc/plan/phase-current/intent.md
python3 "$S" add --anchors "$A" --statement "<verbatim words>" --stated-by "<name>" --context "planning session" \
  --summary "<short paraphrase for tables>" --scope "<where it applies>" \
  --deterministic-check "<diff, git log, grep or test that proves it literally>" --status locked   # first add creates anchors.json
python3 "$S" update --anchors "$A" --id ANC-001 --fixture-pass "<a sentence a bead would say if it satisfied this>" \
  --fixture-fail "<a sentence that would violate it>"                                              # draft, then get the user's approval
python3 "$S" validate --anchors "$A" && python3 "$S" render --anchors "$A"
```

Then create the bead tree and record the bead IDs with `set-beads` (`references/bead-layout.md`). Requirements and ADRs that govern this plan are added the same way with `--source-type REQ|ADR --source-ref <ID>`; they keep their register ID. Fill in `intent.md` from the template if it was just created.

### C. Planning gate

Read `references/planning-gate.md`. Enumerate the planning beads (the open children of the phase epic, `bd children <epic> --json`) and delegate each to `anchor-plan-review` (see Agent Delegation). It composes the bead text, resolves question overrides from the plan-questions beads, runs the script once (one Jev request per anchor, retries and shortening handled inside the script), and returns verdicts. Then compute coverage across beads in code, never by asking Jev:

```bash
python3 "$S" coverage --runs doc/plan/phase-current/anchor-runs --gate plan --format md
```

The gate passes when every bead's latest run has no `contradicted`, `not_addressed`, `underspecified` or `error` verdict (the script exits 1 otherwise) and every locked anchor is `satisfied` by at least one bead. Before editing on a failing verdict, re-read the exact bead text that was sent at the point the anchor governs; if Jev looks wrong, say so in the posted report and ask the user rather than rewording the bead until Jev agrees. Compare runs with `diff --prev <run> --next <run>`. Question design: `references/plan-anchor-questions.md`. Verdict logic: `references/verdict-gating.md`.

### D. Development gate (sprint review)

Read `references/development-gate.md`. When a sprint's work is delivered, the quality-manager side delegates to `anchor-sprint-review` with the sprint's epic bead ID and the evidence to judge (sprint report, PR number or body, test summary). It runs the development question set, which adds an evidence question so "we followed it" is separated from "here is where", and returns the deterministic checks to pair with each `satisfied` or `asserted_only` anchor. Run those checks and record them; Jev judges text, and only a diff or git history proves the code did what the text says. Question design: `references/code-review-questions.md`.

### E. Report

Reports quote the user's own words back, in the format of `references/report-format.md`. Post the markdown from `report --run <run> --format md` as a comment on the judged bead (`bd comment <id> --file <report.md>`) or on the PR so the record travels with the work. Posting is done here, in the main session; agents never write to beads.

### F. Maintain

An anchor that proves durable is promoted: `python3 "$S" promote --anchors "$A" --id ANC-012 --adr ADR-0007` keeps the ID and adds the reference. A statement is never reworded in place; a changed directive is a new anchor and the old one is retired with `python3 "$S" retire --anchors "$A" --id ANC-012 --replaced-by ANC-020 --reason "<why>"`. Scope, summary, fixtures and deterministic checks change through `update`. Re-render after every change. Lifecycle rules: `references/anchor-document.md`.

## Agent Delegation

Capture (A) and locking (B) are interactive and stay in the main session. The gates are tool-heavy and run in isolated agents that return fenced JSON; the skill presents results to the user.

| Operation | Agent | Inputs | Returns |
| --- | --- | --- | --- |
| Planning gate, one bead | `anchor-plan-review` | `script`, `bead_id`, `anchor_ids` or `all`, `anchors_path`, optional `run_dir`, `threshold`, `dry_run`, `correlation_id` | fenced JSON: run path, pass flag, per-anchor verdicts, report markdown |
| Development gate, one sprint | `anchor-sprint-review` | `script`, `sprint_id`, `evidence` paths or `pr_number`, `anchor_ids` or `all`, `anchors_path`, optional `run_dir`, `threshold`, `dry_run`, `correlation_id` | fenced JSON: run path, pass flag, per-anchor verdicts, deterministic checks to pair, report markdown |

In Claude Code, invoke the agent with the Task tool: "Invoke `anchor-plan-review` with script `$S`, bead_id proj-a3f8, anchor_ids all, anchors_path doc/plan/phase-current/anchors.json, correlation_id proj-a3f8." Where no such agent is available, as in Codex or Cursor, follow `../../agents/anchor-plan-review.md` or `../../agents/anchor-sprint-review.md` (relative to this skill's directory) yourself.

Parallel runs (Claude Code only): at most 3 beads at once, `correlation_id` set to the bead ID, 120 s timeout per agent. A timed-out agent is recorded as `{"success": false, "canceled": true, "aborted_by": "timeout", "error": {"code": "EXECUTION.TIMEOUT"}}`. Aggregate in bead-ID order and surface a summary of succeeded and failed beads; never drop a failed bead. Treat unfenced or malformed JSON from an agent as failure. Agents never retry the script; the script already makes three attempts per anchor and shortens oversize items itself.

## Positions taken on the PRD's open questions

- **Granularity**: one testable constraint per anchor. A compound statement becomes several anchors that each quote the sentence carrying their constraint.
- **Capture**: the orchestrator proposes candidates it detects; nothing is locked until the user confirms or says "lock this".
- **Fast-model cross-check**: not in this version. Jev's own per-question confidence drives the `underspecified` verdict instead.
- **Relationship to plan hardening**: standalone and composable. Run the planning gate as an extra step after hardening; neither skill depends on the other.
- **Data retention**: fail closed. The gates refuse to call TypeSafe unless `PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1` is set for the session, and `--dry-run` writes exactly what would leave the machine.
- **Fixture authorship**: the skill drafts one pass and one fail fixture per anchor at lock time; the user approves them. `check-fixtures` sends the sentences to Jev (same opt-in) to confirm the question wording lands them as expected; `--answers-file` fixtures exercise the gating offline in CI.

## Hard stops

- A statement that is not the user's verbatim words is not an anchor. Stop and ask for the exact wording.
- Never send more than one bead or one work description per Jev request, and never the whole plan.
- Never compute coverage, counts, or "satisfied anywhere" by asking Jev. The script does it.
- Missing `bd`, `python3`, API key, or opt-in: surface and stop; do not degrade silently.
- Three consecutive gate runs on the same bead with the same failing set: escalate to the user rather than editing the plan again.

## References

- `references/anchor-capture.md` - detecting, proposing, confirming, and wording anchors
- `references/anchor-document.md` - intent.md, anchors.json schema, anchors.md, IDs, lifecycle, promotion, retirement
- `references/bead-layout.md` - the anchor bead tree, bd commands, labels, question overrides in beads, reading beads
- `references/planning-gate.md` - inputs, run, exit codes and reading results for the planning gate
- `references/development-gate.md` - inputs, evidence, run, exit codes and reading results for the development gate
- `references/plan-anchor-questions.md` - the planning-gate question set and why it is orthogonal
- `references/code-review-questions.md` - the development-gate question set and what evidence to feed it
- `references/verdict-gating.md` - how independent Jev answers become one verdict, thresholds, coverage, diffs
- `references/report-format.md` - the provenance-quoting report and what to do with each verdict
- `references/jev-api.md` - endpoint, request and response shape, limits, retries, privacy opt-in, every flag and error code
- `references/installation-and-troubleshooting.md` - installing python3, bd, and the API key; PATH issues; Codex environment
- `assets/questions/plan.json`, `assets/questions/dev.json` - the question wording of record
- `assets/templates/intent.md`, `assets/templates/anchor-questions-bead.md`, `assets/examples/` - starting points and a worked example
- `tests/test_jev_judge.py` - offline test suite; `python3 -m unittest discover -s <skill dir>/tests`
