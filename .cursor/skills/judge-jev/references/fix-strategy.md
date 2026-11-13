---
name: fix-strategy
version: 0.1.0
description: How to turn Jev's answers on a failing REQ, NFR or ADR into edits that pass Jev and keep the author's intent - routing each failure, edit tactics ranked by measured yield, noise rules, measuring and combining candidates, the drift review, known Jev misreads, stop rules and the report.
---

# Fix strategy

An accepted edit does two things: the record passes Jev in repeated runs, and it still requires exactly what the author's text required. Jev scores clarity; it cannot see a lost obligation. You are the author: proposers suggest, Jev measures, you decide and review. This is a loop you run to the end, not a proposal for the user to approve.

## 0. Set up

```bash
ID=REQ-ABC-0001; KIND=req          # KIND is req, nfr or adr
F=docs/requirements/abc.md         # the file that holds the record
W=.sc/judge-jev/fix/$ID; mkdir -p "$W"
cp "$F" "$W/original.md"           # the author's text: every drift check compares against it
cp "$F" "$W/current.md"            # the best accepted text so far
printf 'J=%s\nRAPTOR=%s\nID=%s\nKIND=%s\nF=%s\nW=%s\n' "$J" "$RAPTOR" "$ID" "$KIND" "$F" "$W" > "$W/env"
```

Shell variables do not survive between tool calls: start every later command with `. .sc/judge-jev/fix/<ID>/env`. Work on one record per file at a time.

Three commands do all the measuring.

Snapshot the file as it is now, as variant `<v>` (`base`, `c1`, `comb`, `final`):

```bash
"$RAPTOR/.venv/bin/python" "$RAPTOR/scripts/extract.py" --project-root . --output "$W/index-<v>.json" | jq '.issues'   # must print 0
```

Judge variants interleaved, the same number of runs each:

```bash
for r in 1 2 3; do for v in base c1 c2; do
  uv run -q --script "$J" quality --$KIND "$ID" --index "$W/index-$v.json" --run-name "$ID-$v-r$r" > "$W/$v-r$r.json"
done; done
```

Score every run:

```bash
for f in "$W"/*-r*.json; do jq -r --arg run "$(basename "$f" .json)" 'if .success | not then "\($run) ERROR \(.error.code): \(.error.message)" else .data.results[] | "\($run) \(.verdict) failing=\([.waves[] | select(.verdict == "no") | .matched_statements // ["record"]] | flatten)" end' "$f"; done | sort
```

Read one run's answers with the step 2 command of `references/quality-workflow.md`.

## 1. Baseline

Snapshot `base` and judge it 3 times. For every failing statement (ADR: every failing answer) note, per run, the answer that fired the rule and its confidence.

## 2. Route each failure before editing

| What the answers show | Cause | Action |
| --- | --- | --- |
| It fails in 1 of 3 runs, deciding answer below 0.35 | noise | Judge 6 more runs. Act only if it recurs. |
| `term` = `project term defined nowhere` or `undefined acronym` | a term the record does not define | Tactic 1 |
| `term` = `vague qualifier`, or `check` = `needs a measure` or `none` | text not specific | Tactic 2 |
| `obligations` 2+ with `and-list`, `alternatives` or `obligation+rationale` | compound statement | Tactics 3 to 5 |
| ADR `context`, `alternatives` or `consequences` fails | missing ADR content | ADR tactics |
| The record already states what the answer says is missing, and it persists after one fix | question misfit or Jev misread | Known misreads; do not reword |

Route by the answer that fired a fail rule, and only that answer. `joined_by` = `condition+obligation` never fails; a statement whose wave passed needs no edit.

Fix `term` failures first, with definitions alone, and re-measure before anything else: structure answers below 0.5 (`obligations`, `joined_by`) often settle once the terms are defined.

Confidence: an answer at 0.5 or above did not flip between identical runs; below 0.5 it can, below 0.35 often. A confident failing answer (0.75 or above) is a real defect in the text. Jev sees only this record: a term defined in another record or file is undefined to Jev.

## 3. Tactics, ranked by measured yield

1. **Define the term in the record.** Add one sentence to the record's `Definitions:` paragraph (the first paragraph under `### Requirement Statement`, starting `Definitions:`; create it if absent). Leave the statement unchanged. Take the meaning from the repository; if another record already defines the term, reuse its wording (`grep -rn "<term> is" docs/`). Expand acronyms; define product names once ("sc-agent is ..."). Undefined terms caused about nine failures in ten, and definitions alone took most records from failing to passing. The `Fix:` line says "define it or link the record": linking never worked. Define it here.
2. **Replace a vague qualifier with the measure the code uses.** Find the number in the code or tests and cite the file and line in your report. If the repository states no number, do not invent one: the statement is unresolved and needs the user's decision.
3. **Split a list-joined statement** into one statement per item, each with the original subject and modal, every item and gloss kept. Never move the items into a definition and leave the statement naming only the set.
4. **State one choice over a named set** for alternatives ("exactly one of `--a` (x) or `--b` (y)" fails; define the set and require one of it). "Do X without Y" reads as two obligations; "do X with no Y" reads as one.
5. **Move rationale** (because, so that, to ...) out of the statement into `### Rationale`.

One tactic per candidate, so each result is attributable. Write a definition from how the record and the repository use the term: it must cover every use (a "question set" that may be a shipped id or an override path is not "the shipped set"), and each sentence needs a source file and line, cited in your report. The record cites only record ids: no file paths or line numbers. Keep the source's qualifiers: a number stated as a default or bounded by a flag stays a default ("24000 bytes by default", not "the largest size"). Reword a statement only when it still fails after its terms are defined.

Low yield, do not lead with them: citing or linking a record; rewording, splitting or hardening statements that pass (splits aimed at passing statements almost never helped); any edit not aimed at a failing answer. Never delete words from a MUST or MUST NOT statement to raise a score: every such deletion that passed was drift.

**ADR tactics.**

- `context` = `background only` or `missing`: add one sentence starting "The problem is ...", built from the record's own Decision and Rationale.
- `alternatives` = `none` or `named only`: add `### Alternatives Considered` with one `#### Alternative <n>: <name>` per option, each with `**Description:**` and `**Why Rejected:**`. Take the options from the Decision's "never" or "instead" clauses, the Rationale, or other records, and cite the REQ or ADR id that rules each out. Each alternative is a different design; two that are rejected for the same reason are one. Never invent one.
- `consequences` = `positive only` or `missing`: add the cost the record or code already shows.

Fix every Raptor import diagnostic before judging.

## 4. Propose candidates

Always launch proposers, even when the fixes look obvious: your own candidates are one model's view. Launch at least two `jev-fix-proposer` agents in parallel, with no shared context. The agent runs on the frontier model (Claude Code: `opus`; Codex: `sol`; Cursor: `grok`, high effort); do not override it. When another harness is available, add one proposer there. Each gets one fenced JSON object:

```json
{
  "record_id": "REQ-ABC-0001",
  "kind": "req",
  "file": "docs/requirements/abc.md",
  "record_text": "<the record as it is now>",
  "original_text": "<the record as the author wrote it>",
  "failing": [
    {"statement": 3, "text": "<statement text>", "runs": ["obligations=1 @0.93; joined_by=none @0.79; term=project term defined nowhere @0.44; check=CLI run @0.94"]}
  ],
  "blocked": ["<a span or edit rejected in an earlier round, with the reason>"],
  "sources": ["docs/", "src/"]
}
```

`failing` holds every failing statement (ADR: `"statement": null` and the three answers) with each run's answers. Add your own candidates from the tactics, then drop duplicates.

## 5. Screen

Wait until every proposer has returned or failed, then screen. Reject a candidate, before spending runs on it, when:

- its `old` text does not occur exactly once in the file;
- it targets no answer that failed in your runs;
- the token guard reports a loss:

```bash
python3 - "$W/original.md" "$W/c1.md" <<'EOF'
import re, sys
from collections import Counter
pats = {"code span": r"`[^`\n]+`", "number": r"(?<![\w.])\d+(?:\.\d+)?(?![\w.])",
        "record id": r"\b[A-Z]+-[A-Z0-9]+-\d+\b", "MUST": r"\bMUST(?! NOT)\b", "MUST NOT": r"\bMUST NOT\b"}
a, b = (open(p, encoding="utf-8").read() for p in sys.argv[1:3])
lost = {k: sorted((Counter(re.findall(p, a)) - Counter(re.findall(p, b))).elements()) for k, p in pats.items()}
lost = {k: v for k, v in lost.items() if v}
print(f"REJECT, lost: {lost}" if lost else "guard ok")
EOF
```

- reading it shows drift (section 8).

Append one line per candidate to `$W/tried.jsonl`: `{"id", "proposer", "statement", "answer", "type", "old", "new", "result", "why"}`. Rejected spans go into the next round's `blocked`.

## 6. Measure each candidate alone

For candidate `<n>`: apply its edit to `$F`, `cp "$F" "$W/c<n>.md"`, snapshot `c<n>`, then `cp "$W/current.md" "$F"`. Judge `base` and every candidate interleaved, 3 runs each. Keep a candidate when the statement it targets passes in all 3 runs where `base` failed it in at least 2, and no other statement fails more often than in `base`. Anything less clear: 3 more runs of both. One rejudge is never evidence.

## 7. Combine

Apply all kept candidates to `$F` together. Where two target the same statement, take the one with fewer failing runs, then fewer words. Snapshot `comb` and judge it 6 times, interleaved with `base`. Candidates that pass alone often fail together: if `comb` fails, drop one candidate at a time, the weakest first, and re-measure. When it holds, `cp "$F" "$W/current.md"`.

## 8. Drift review

Run the token guard on `$W/original.md` against `$F`, then read `diff -u "$W/original.md" "$F"` line by line. Revert any change that:

- deletes or weakens an obligation, qualifier, scope phrase, enumeration or parenthetical gloss ("(agent text or code)", "for the same file", "in its state next to the record");
- abstracts a concrete item away (`--profile` becoming "the profile form");
- turns rationale or description into a MUST, or adds a condition the original lacked ("When a command completes ...");
- widens or narrows the scope ("every tree, question set and definitions file" becoming "every file in the scope");
- changes one of two parallel statements but not the other (two statements that described the same entry the same way must still match);
- defines a term more narrowly or widely than the record uses it (a location that excludes code when the record allows `code` mode);
- leaves a definition false, circular ("Recorded answers are an answers file"), cut short ("The digest is the SHA-256.") or starting with a pronoun that has no antecedent;
- defines a term differently from another record in the repository;
- states a fact the repository does not support, or changes a user's quoted words;
- speaks to the judge instead of the reader (stating that the record complies, mentioning a topic to "cover" it).

Keep changes that make the record more specific, testable or accurate. After any revert, re-measure.

## 9. Confirm

Run the token guard on `$W/original.md` against `$F`; it must print `guard ok`, or every loss is a deliberate, reviewed move. Snapshot `final` and judge it 6 times. The record passes when all 6 verdicts are `yes`. Otherwise start a new round at step 2 with the statements that failed.

## Stop

- A record is done when it passes 6 of 6.
- A statement still failing after 3 rounds, or whose only passing candidates drift, is unresolved: keep its `current.md` text and move on to the next record.
- Known Jev misreads; report them, do not reword:
  - a lowercase or hyphenated product name (`sc-agent`, `judge-jev`) read as `undefined acronym` at low confidence, even once defined;
  - `undefined acronym` on a statement with no acronym, at low confidence;
  - `project term defined nowhere` near 0.3 for a term this record defines.
- Never rerun until a pass appears, ask the user to approve wording mid-loop, or build scripts and harnesses beyond these commands. Jev answers in about 100 ms; a record takes minutes.

## Report to the user

Only measured results:

- each passing record: runs passed before and after (for example 0/3 to 6/6), the token guard's output, the proposer models used, `diff -u "$W/original.md" "$F"`, and one line per change naming the answer it fixed;
- each unresolved statement: its text, Jev's answers per run verbatim (`question=choice @confidence`), what was tried and its measured result, your diagnosis, and the decision you need from the user (for example a number the repository does not state).

Leave `$W` and `.sc/judge-jev/logs/` in place.
