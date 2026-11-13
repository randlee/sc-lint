---
name: judge-jev
description: Judge whether REQ, NFR and ADR records are well written with TypeSafe's Jev, reading them through Raptor, and fix failing records until they pass without changing what they require. Use when the user asks to check, judge, score, tune or fix requirements, NFRs or ADRs, asks whether a record is testable, vague or well written, or asks to add or test a definition for a record's term. Also use when asked whether a prompt, skill, agent, task template or code meets its REQ, NFR or ADR; that mode is not in this release, and the skill says so. Not for parsing records (Raptor does that) and not for general code review.
metadata:
  version: "0.1.0"
---

# judge-jev

judge-jev sends each REQ, NFR or ADR record to Jev with its kind's question tree. Jev answers every question with a choice and a confidence; judge-jev turns the answers into a verdict, `yes` or `no`. The answers are the reasons: each chosen option's criterion names the defect, and a failing one carries a `Fix:` line. Records come from Raptor's import index; judge-jev never parses Markdown.

This file is the table of contents. Do Steps 1 and 2, then read only the reference for your mode.

## Step 1 - Verify dependencies

```bash
which uv && uv --version
which jq && jq --version
J="<this skill's directory>/scripts/jev_judge.py"
uv run --script "$J" --help >/dev/null && echo "judge-jev ok"
test -n "$TYPESAFE_API_KEY" && echo "TYPESAFE_API_KEY set" || echo "TYPESAFE_API_KEY missing"
uv run -q --no-project --with pyyaml python "<this skill's directory>/scripts/judge_jev_settings.py" show | jq '.data.allow_remote_judge'   # true, false, or null
```

Run `jev_judge.py` only with `uv run --script "$J"`: its inline metadata supplies its dependencies, and plain `python3` fails with `DEPENDENCY.*`. Shell variables do not persist between tool calls; set `J` (and `RAPTOR`, below) in every shell.

`allow_remote_judge` in the committed `.sc/judge-jev/settings.yaml` is the user's answer to whether record text may be sent to TypeSafe (Jev). `true`: continue. `false`: live calls are refused with `POLICY.REMOTE_JUDGE_DISABLED`; tell the user and work offline only (`--dry-run`, `validate`, `doctor`). `null`: ask the user once, record the answer with the same command and `set allow_remote_judge true|false`, and commit the file. Never infer consent from run files or other repositories.

Raptor, once per machine (needs a Rust toolchain; it builds the `raptor_schema` PyO3 module):

```bash
git clone -b integration/judge-jev https://github.com/randlee/raptor "$RAPTOR"
uv venv "$RAPTOR/.venv" -p 3.11 && uv pip install -p "$RAPTOR/.venv/bin/python" "$RAPTOR" markdown
```

The repository needs a `.raptor/` inventory naming its record files (Raptor's README). New records are written from Raptor's templates (`$RAPTOR/templates/README.md`).

A missing tool or credential: read `references/installation-and-troubleshooting.md`, then stop until it is fixed.

## Step 2 - Build the index

```bash
"$RAPTOR/.venv/bin/python" "$RAPTOR/scripts/extract.py" --project-root . --output index.json
```

Exit 0 with `"issues": 0` means every record imported. Otherwise fix each diagnostic it prints (file, line, rule, fix) and rerun. judge-jev judges only records Raptor imported without diagnostics.

## Select the mode

| Mode | Question | Read |
| --- | --- | --- |
| quality | Is a REQ, NFR or ADR well written? Judge it, and fix it if it fails. | `references/quality-workflow.md`; to fix, also `references/fix-strategy.md` |
| compliance | Does a prompt, skill, agent, task template or code meet its REQ, NFR or ADR? | Not in this release: judge-jev has no `check` command yet. Tell the user so and stop. Do not judge it another way. |

## Agent

| Agent | Role | Returns |
| --- | --- | --- |
| `jev-fix-proposer` | Proposes candidate edits for one record's failing answers. Run several in parallel with no shared context; they share no context. | Fenced JSON: candidate edits, each naming the failing answer it targets |

You stay the author: you apply, measure, combine and review every candidate. A proposer never edits a file or calls Jev. Agents load when a session starts: if `jev-fix-proposer` is not available by name, launch a general-purpose agent whose prompt is the body of `.<harness>/agents/jev-fix-proposer.md` (at the install root) followed by the input object. If no agent can be launched, say so and write the candidates yourself from that file.

## Hard stops

- Every round launches the `jev-fix-proposer` agents before any candidate is measured (`references/fix-strategy.md` step 4). Your own candidates join theirs; they never replace them.
- Never edit a question, tree, criterion or threshold to make a record pass.
- Never answer for Jev. A failed call is rerun once, then reported with its code and verbatim message; it is never a verdict.
- Never show the user a proposed edit, or call a record passing, before live runs show it passing.
- Never keep an edit that drops, weakens or adds an obligation, even when Jev scores it higher.
- Never invent a number, term definition or alternative; take it from the repository and cite the file and line in your report, never in the record.
- Every round of edits ends with a live Jev run. Keep the run files and `.sc/judge-jev/logs/`.

Jev request format, limits and every error code: `references/jev-api.md`.
