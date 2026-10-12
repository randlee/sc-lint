---
name: plan-anchoring
description: Capture the facts a user states while planning as numbered anchors (ANC-001...), one clear, testable requirement each, with the user's verbatim words kept as provenance, alongside existing requirements (REQ) and ADRs. Turn them into Jev-judged questions, tune and test the questions per anchor, and enforce them at two gates with a triage-apply-rerun loop. The planning gate checks each plan bead still says what the user said; the development gate checks the delivered sprint work followed it. Use when planning a phase or sprint with beads, when the user says "anchor this", "lock this", "don't let this drift", or "remember I said", when re-opening a plan after context compression, or when reviewing a sprint against its plan. Not for general code review, and not a replacement for plan hardening.
metadata:
  version: "0.4.5"
---

# Plan Anchoring

Facts stated during planning get compressed out of context, ceremony accumulates, unapproved work creeps in, and plans and code drift from what was said. This skill makes each stated fact durable: the user's words are kept verbatim; the orchestrator writes each as one testable requirement the user approves; the requirement becomes orthogonal Jev questions, tuned per anchor; and the plan and the delivered work are judged against it in a loop of run, triage, fix, re-run.

This file is the table of contents; read the reference for the step you are on.

## Step 1 - Verify dependencies

Run before any capture, delegation, or gate. On a missing dependency, surface the gap and stop.

```bash
which python3 && python3 --version          # 3.9 or newer
which bd && bd --version                    # beads CLI, 1.x ("(dev)" suffix is fine)
which sc-compose && sc-compose --version    # 1.7.0 or newer
test -n "$TYPESAFE_API_KEY" && echo "TYPESAFE_API_KEY set" || echo "TYPESAFE_API_KEY missing"
echo "remote judge opt-in: ${PLAN_ANCHOR_ALLOW_REMOTE_JUDGE:-unset}"
```

If `bd` is not on PATH, Claude Code's bash may not share your shell's PATH. Check common locations:

```bash
for p in "/opt/homebrew/bin/bd" "$HOME/go/bin/bd" "$HOME/.local/bin/bd" "/usr/local/bin/bd"; do
  [ -x "$p" ] && echo "Found at: $p" && break
done
for p in "/opt/homebrew/bin/sc-compose" "$HOME/.local/bin/sc-compose" "/usr/local/bin/sc-compose"; do
  [ -x "$p" ] && echo "Found at: $p" && break
done
```

If found off PATH, `export PATH="<dir>:$PATH"` for the session. If anything is missing, read `references/installation-and-troubleshooting.md`. This skill requires Jev: capture runs the live rubric and question tuning and both gates make live judgments, so the normal workflow requires the API key and per-session opt-in in `references/jev-api.md`. Offline `--dry-run`, fixture-answer tests, lint, shape, rendering, and record maintenance remain available for development, but they are not a substitute for the Jev-backed workflow.

Then resolve the script once. It sits next to this file:

```bash
S="<this skill's directory>/scripts/jev_judge.py"   # absolute; Claude plugin: $CLAUDE_PLUGIN_ROOT/skills/plan-anchoring/scripts/jev_judge.py
A=doc/plan/phase-current/anchors.json
python3 "$S" --help
```

Every command below uses `$S` and `$A`; every agent invocation passes `$S` as `script`.

## Select the workflow

After dependency verification, read only the row for the current operation.
The workflow retains the tested language from this file and points to deeper
contracts only when that operation needs them.

| Operation | Read |
| --- | --- |
| Capture, lock, or maintain anchors and requirements | `references/capture-and-maintenance-workflow.md` |
| Calibrate the rubric or tune anchor questions | `references/question-tuning-workflow.md` |
| Review a Markdown or `bd`/`br` plan | `references/planning-review-workflow.md` |
| Review delivered sprint work | `references/development-review-workflow.md` |

Do not preload the other workflow references. Open only additional references
named by the selected workflow when their detail is needed.

## Agent Delegation

When the selected workflow delegates, read
`references/agent-delegation-workflow.md` before invoking an agent. Capture and
locking do not load that reference.

## Design positions

- **Granularity**: one testable constraint per anchor; a compound statement becomes several anchors, each quoting the sentence carrying its constraint.
- **Capture**: the orchestrator proposes candidates it detects; nothing is locked until the user confirms or says "lock this".
- **Fast-model cross-check**: not in this version. Jev's per-question confidence drives the `underspecified` verdict instead.
- **Relationship to plan hardening**: standalone and composable; run the planning gate after hardening.
- **Data retention**: fail closed. The gates refuse to call TypeSafe unless `PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1` is set for the session, and `--dry-run` writes exactly what would leave the machine.
- **Fixture authorship**: the skill drafts one pass and one fail fixture per anchor at lock time; the user approves them. `check-fixtures` sends them to Jev (same opt-in) to confirm the wording lands them; `--answers-file` fixtures exercise the gating offline in CI.
- **Question-tuning independence**: the planner cannot tune. An isolated Opus/Sol evaluator derives and freezes its oracle from approved intent and anchors before seeing Jev answers, then iterates wording against that oracle without regard to whether the current plan passes. A live gate failure alone never authorizes an override or scope change. The main session records only an override accompanied by the frozen oracle, digest, acceptance table, and run IDs.

## Hard stops

- An anchor without a verbatim `quote` has no provenance; one whose statement is just the quote is not testable. Both are refused.
- Never tune a question or reword a statement to make one bead pass. Fixtures and a known-bad bead must still fail.
- Never tune or rescope from a live failure alone. Require a frozen, user-approved fixture or expected verdict that independently demonstrates the false result; scope changes require user approval and invalidate prior gate runs for that anchor.
- A failing-to-passing plan edit that grows the composed item is a hard stop unless each added passage is traced to a locked ANC/REQ/ADR that predates the failed run.
- Never split a bead, trim its text, or raise a size cap to pass `PLAN.TOO_LARGE`. Triage unapproved work, then prose and ceremony, then split.
- Never send more than one bead or one work description per Jev request, and never the whole plan.
- Never compute coverage, counts, or "satisfied anywhere" by asking Jev. The script does it.
- Missing `bd`, `python3`, API key, or opt-in: surface and stop; do not degrade silently.
- Three consecutive gate runs on the same bead with the same failing set: escalate to the user rather than editing the plan again.

Maintainers only: the complete resource index is in
`references/reference-catalog.md`; ordinary workflow execution does not load it.
