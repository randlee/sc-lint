---
name: bead-layout
version: 0.2.0
description: The anchor bead tree in beads (bd), the commands to create it, labels, how question overrides live in the child beads, how agents read beads, and how reports are posted back.
---

# Bead layout

Jev questions live in beads so they version alongside the plan work and can be edited per gate without touching the anchor record. One small tree per fact:

```
ANC-012 anchor bead            (type decision, labels: anchor, anchor:ANC-012)
├── plan-questions bead        (type decision, labels: anchor, anchor-questions, anchor:ANC-012, gate:plan)
└── dev-questions bead         (type decision, labels: anchor, anchor-questions, anchor:ANC-012, gate:dev)
```

Requirements and ADRs that govern the plan get the same tree under their own ID (`REQ-014`, `ADR-0007`). Bead IDs carry the repository's prefix set at `bd init` (`proj-a3f8`, children `proj-a3f8.1`); `bd-42` in these documents is illustrative.

## Creating the tree

With `$S` and `$A` resolved as in `SKILL.md`:

```bash
ID=ANC-012
T="$(dirname "$S")/../assets/templates/anchor-questions-bead.md"
STATEMENT=$(python3 -c "import json;print(next(a['statement'] for a in json.load(open('$A'))['anchors'] if a['id']=='$ID'))")
SUMMARY=$(python3 -c "import json;print(next(a['summary'] for a in json.load(open('$A'))['anchors'] if a['id']=='$ID'))")

ANCHOR=$(bd create "$ID: $SUMMARY" --type decision --labels "anchor,anchor:$ID" --silent \
  --description "$STATEMENT" \
  --notes "Anchor record: $A. Do not reword the description; it is the user's verbatim statement.")
PLANQ=$(bd create "$ID plan questions" --type decision --parent "$ANCHOR" --labels "anchor,anchor-questions,anchor:$ID,gate:plan" --silent --body-file "$T")
DEVQ=$(bd create "$ID dev questions"  --type decision --parent "$ANCHOR" --labels "anchor,anchor-questions,anchor:$ID,gate:dev"  --silent --body-file "$T")
python3 "$S" set-beads --anchors "$A" --id "$ID" --anchor-bead "$ANCHOR" --plan-bead "$PLANQ" --dev-bead "$DEVQ"
python3 "$S" render --anchors "$A"
```

Labels are passed explicitly on the children even though current `bd` inherits parent labels, so the tree is correct on a `bd` configured otherwise. Anchor beads are type `decision` and labelled `anchor` so they are not mistaken for work; keep them out of work queues with `bd ready --exclude-label anchor`.

## Question overrides in the child beads

The default wording of record is `assets/questions/plan.json` and `assets/questions/dev.json`. A child question bead's description holds one fenced JSON block. `{}` means use the defaults. To change wording for this anchor only, include just the questions to override, keyed by the same IDs as the default set, keeping the `yes`/`no`/`unclear` options:

````markdown
```json
{
  "questions": {
    "addressed": {
      "type": "choice",
      "instructions": "Does `item.text` state that the runtime crates are copied from ATM rather than written new? ...",
      "criteria": {"yes": "...", "no": "...", "unclear": "..."}
    }
  }
}
```
````

Edit with `bd update <question bead> --body-file <file>`. Overrides may also be stored in `anchors.json` under `question_overrides.plan` or `.dev`; the bead wins when both exist. Prefer the bead: it is where the planning and development gates evolve their wording independently while pointing at the same root fact, which is the reason the tree exists. Do not rename question IDs; the script refuses a set whose roles no longer resolve (`QUESTIONS.ROLES`).

## How the agents read beads

- `bd show <id> --json` prints a **one-element JSON array**. Text fields are `title`, `description`, `design`, `acceptance_criteria`, `notes`; a field that is empty is **absent**, not empty. Agents do not parse this themselves: `python3 "$S" compose-item --show-json <file or -> --out <item.txt>` composes `# Title`, `## Description`, `## Design`, `## Acceptance criteria`, `## Notes` from whatever is present.
- `bd children <id> --json` lists children with `status`, `title`, `labels`, and for closed children `close_reason` and `closed_at`. `compose-item --children-json <file>` appends `## Closed beads` with one line per closed child.
- Question beads are resolved first from `anchors.json` (`beads.plan_questions` / `beads.dev_questions`), then by `bd children <anchor bead> --json` filtered on the `gate:plan` or `gate:dev` label.
- The fenced JSON is extracted from the question bead's `description` and written to an overrides file keyed by anchor ID, passed to the script as `--overrides-file`; the script does the merge.

All of these are read-only. Agents never create, update, close or comment on beads.

## Posting results back

The planning gate posts its markdown report as a comment on the judged planning bead; the development gate posts on the sprint bead or the PR. Done in the main session after the agent returns:

```bash
python3 "$S" report --run doc/plan/phase-current/anchor-runs/<run>.json --format md > "$W/report.md"
bd comment <bead> --file "$W/report.md"          # or: bd comment <bead> "$(cat "$W/report.md")"
```

`bd comment` takes `--file` or `--stdin`; `--body-file` belongs to `bd create` and `bd update` only.
