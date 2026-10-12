---
version: 0.4.4
---

# Agent delegation workflow

## Agent Delegation

Capture (A) and locking (B) stay in the main session. Question tuning (B2) and the gates (C, D) run in isolated agents that return fenced JSON; the tuner runs in the background. The skill presents results and makes every write to beads and the record.

Before invoking any bundled agent, resolve it once and use the single-envelope
boundary in `references/agent-runner-contract.md`. The planner owns trust
resolution and attestation. Leaf agents receive the immutable descriptor; they
do not search installation paths or repeat registry/frontmatter validation.

| Operation | Agent | Inputs | Returns |
| --- | --- | --- | --- |
| Planning gate, one bead | `anchor-plan-review` | `script`, `bead_id`, `anchor_ids` or `all`, `anchors_path`, optional `run_dir`, `threshold`, `dry_run`, `correlation_id` | fenced JSON: run path, pass flag, per-anchor verdicts, report markdown |
| Development gate, one sprint | `anchor-sprint-review` | `script`, `sprint_id`, `evidence` paths or `pr_number`, `anchor_ids` or `all`, `anchors_path`, optional `run_dir`, `threshold`, `dry_run`, `correlation_id` | fenced JSON: run path, pass flag, per-anchor verdicts, deterministic checks to pair, report markdown |
| Independent rubric calibration, question tuning, and evidence triage (Opus/Sol) | `anchor-tuning` | one strict fenced JSON object rendered from the checked-in testability, tune, or triage template, including resolved runtime, authority packet or frozen corpus, protocol/history where applicable, and correlation ID | strict Standard-envelope fenced JSON rendered from the result template and validated by schema plus `anchor_tuning_contract.py` |

In Claude Code, invoke the plugin's harness-specific file in `.claude/agents/` with the **Agent** tool, never the Task tool; the manifest registers gate agents with `model: sonnet` and the independent tuner with `model: opus`. In Codex, read the matching plugin-owned definition in `.codex/agents/` and launch it with the Agent tool; gate agents specify `model: luna` and the tuner `model: sol`. Codex plugins discover the skill but do not auto-register these agent profiles, so the skill owns this explicit launch step. Example gate input: "Invoke `anchor-plan-review` with script `$S`, bead_id proj-a3f8, anchor_ids all, anchors_path doc/plan/phase-current/anchors.json, correlation_id proj-a3f8." Cursor has no bundled agent definition in this release and follows the relevant gate steps inline; question tuning requires an independent higher-reasoning agent and does not fall back to the planner.

Parallel runs: at most 3 agents at once, `correlation_id` set to the bead ID, 300 s timeout per agent (Jev takes seconds; the budget is the agent's own turns). An agent that times out or returns unfenced or malformed JSON has produced no result: re-dispatch that bead once, passing as `run` any run file it already wrote, so Jev is not asked again. Only when the retry also fails, record `{"success": false, "canceled": true, "aborted_by": "timeout", "retried": true, "error": {"code": "EXECUTION.TIMEOUT"}}`. Aggregate in bead-ID order; never drop a failed bead. Agents never retry the script; it already makes three attempts per anchor and shortens oversize items itself.

<!-- The workflow text above is preserved verbatim from the 0.4.3 entrypoint. -->
