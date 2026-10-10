---
name: question-tuning
version: 0.3.0
description: When and how to replace the generic Jev questions with anchor-specific wording, where the overrides live, how to test a question set with fixtures and known beads before trusting it, and when to stop tuning.
---

# Tuning the questions

The default sets (`assets/questions/plan.json`, `assets/questions/dev.json`) ask the same generic questions of every anchor; they are a starting point. Every anchor's plan-questions and dev-questions beads start as `{}` (defaults); the orchestrator tunes and tests the wording for any anchor the defaults judge poorly.

## First: is the anchor testable?

Before tuning, run the rubric:

```bash
python3 "$S" rubric --anchors "$A" --id ANC-012            # a locked anchor
python3 "$S" rubric --statement "<candidate>" --quote "<words>" --scope "<scope>"   # before add
```

It runs the deterministic lint and asks Jev six questions about the statement: one constraint, observable outcome, requirement form, faithful to the quote (no added or dropped requirement), scoped, and positive form. `testable` means every answer is yes at confidence 0.6 or higher and the lint has no errors; `needs_work` lists what failed. `add --rubric` refuses a failing statement. Fix the statement before tuning a single question.

## Who tunes

The `anchor-question-tune` agent, launched in the background by the skill for each locked anchor and gate. It follows the procedure below and returns the tested override with an acceptance table; the skill writes the override into the question bead and posts the acceptance. The procedure is written out so it can be followed by hand and the agent's output checked.

## When to tune

Tune when a gate run shows one of these and the sent bead text does not explain it:

| Symptom | Likely cause | Tune |
| --- | --- | --- |
| `underspecified` on the same anchor across several beads, `min_confidence` under 0.7 | The generic `addressed` question cannot tell whether the directive's specific content is present | Name the content in the question |
| `not_addressed` on a bead the user says clearly does the thing | The bead uses different vocabulary than the statement ("fork the legacy repo" vs "copy the crates") | List the acceptable phrasings in the `yes` criterion |
| `satisfied` on a bead that only mentions the topic | The question accepts topic mention as compliance | Tighten the `yes` criterion to the specific starting point, method or constraint |
| `relevant: yes` on unrelated beads, or `relevant` is the limiting question across many beads | `anchor.scope` is a topic sentence Jev does not use (measured-behaviour.md) | First rewrite `scope` as bead-level signals (`update --scope`), then override `relevant` to list those signals ("the item adds or changes a CLI subcommand", "the item edits a Cargo.toml") |
| `contradicted` on a bead that merely omits the directive | The conflict question reads silence as conflict | Add "silence is not a contradiction" examples to the `no` criterion |
| `evidenced: no` on a report that cites a diff | The evidence question does not recognise the artifact type the team produces | Name the artifact types in the `yes` criterion |

Do not tune to make a bead pass. Tune until the fixtures land and a known-good and a known-bad bead are told apart.

## Where overrides live

An override has the shape of the default set and contains only the questions to replace, keyed by the same IDs (`relevant`, `addressed`, `contradicted` for plan; `relevant`, `implemented`, `violated`, `evidenced` for dev), each keeping `yes`, `no`, `unclear`. IDs and options are fixed because `decide()` depends on them (`verdict-gating.md`).

- **Per anchor, per gate, in the question bead** (preferred): the fenced ```json block in the plan-questions or dev-questions bead (`bead-layout.md`). Edit with `bd update <bead> --body-file <file>`.
- **In the record**: `question_overrides.plan` / `.dev` in `anchors.json`, for a plan with no beads. The bead wins if both exist.
- **For a whole plan**: a complete alternative set passed with `--questions <file>`. Rare.

## How to write an anchor-specific question

Start from the default and make it concrete. `instructions` say what to judge and what not to judge; `criteria` describe situations, not degrees.

Default `addressed`:

> Does `item.text` explicitly carry out, preserve, or plan for the directive given in `anchor.statement`? ...

A measured example: this override for a layering anchor corrected every false pass caused by "layer" meaning a stacked-PR position (measured-behaviour.md). It names the four layers, lists the vocabulary that counts, and calls out the homonym:

```json
{
  "questions": {
    "addressed": {
      "type": "choice",
      "instructions": "The anchor requires two things of a sprint bead: (1) the bead states which architectural layer its change lands in, and (2) that layer is the lowest one able to carry the change. The architectural layers, lowest to highest, are: (a) shared types and attributes (the types crate); (b) independent analyzer tool crates; (c) the CLI crate; (d) the adoption-kit template. Decide whether `item.text` names its architectural layer in this sense. The word 'layer' in `item.text` often means something else: a stacked-PR position ('planned layer 1', 'the layer-7 PR') or a consumer test layer; those uses do not name an architectural layer. ...",
      "criteria": {
        "yes": "The item text identifies the architectural layer (a)-(d) its change lands in, by letter, by layer name, or by stating that the named crate or kit is the lowest layer able to carry the change.",
        "no": "The item text does not identify an architectural layer: it names only crates, files or paths without calling any of them the layer, or uses 'layer' only for stacked-PR positions or test layers.",
        "unclear": "The item text mentions the layered architecture but it is not possible to tell which layer it claims the change lands in."
      }
    }
  }
}
```

The same shape, tuned for ANC-012 ("The runtime crates are created by copying the legacy repo's runtime crates and deleting the legacy-specific modules; no runtime crate is written from scratch."):

```json
{
  "questions": {
    "addressed": {
      "type": "choice",
      "instructions": "Does `item.text` state that the runtime crates start as a copy of the legacy repo's runtime crates (words such as copy, fork, vendor, import, port over, start from the legacy repo) with legacy-specific modules then removed? A plan to design or write a new runtime, even one that follows the legacy design notes, is not a copy. Judge presence of the copy-then-strip plan only; do not judge conflict.",
      "criteria": {
        "yes": "The item names the legacy repo as the source the runtime crates are copied from, and mentions removing or stripping legacy-specific parts.",
        "no": "The item does not say the runtime crates come from the legacy repo, or says they are written new, or is silent about where they come from.",
        "unclear": "The item mentions the legacy repo but it cannot be determined whether the crates are copied from it or merely modelled on it."
      }
    }
  }
}
```

Rules (Jev reads literally):

- One judgment per question. Two things to check is two anchors, not a longer question.
- Point at fields with backticks: `item.text`, `anchor.statement`.
- Spell out scoping words, negations and the "not this" cases; avoid double negatives and "the former".
- Put vocabulary variants in the criteria, not the instructions.
- Keep `instructions` and `criteria` consistent.
- Never ask Jev to count, compare numbers, or look across beads.

## How to test a question set

Never deploy an override unread by Jev. Test in this order; each step is one command.

1. **Inspect the request.** `judge --anchor ANC-012 --item-file <known bead> --dry-run`, then read `requests[0].questions` in the run file. The override must appear verbatim and the other questions must still be the defaults.
2. **Fixtures.** With at least one `fixtures.pass` and one `fixtures.fail` sentence on the anchor (`update --fixture-pass/--fixture-fail`), run `check-fixtures --gate plan --id ANC-012` with the override in place. Expected: pass -> `satisfied`, fail -> `not_addressed` or `contradicted`, each at confidence 0.8 or higher. A fixture that lands wrong means the question is wrong, unless the user agrees the fixture misstates the anchor.
3. **Known beads.** Judge one bead the user says clearly satisfies the anchor and one that clearly does not. Keep both runs.
4. **A/B against the default.** Run the same two beads with the default set (`--questions assets/questions/plan.json`, or move the override aside) and `diff` the pairs. The override must fix the symptom without flipping a correct verdict.
5. **Whole-gate regression.** Re-run the planning gate for all beads and `coverage`. Compare the uncovered list before and after.

Acceptance: both fixtures land, known-good is `satisfied`, known-bad fails, `min_confidence` on both is 0.8 or higher, and no previously correct bead changed verdict. Judge "changed" against what is correct, not the previous run: `diff --expected <json>` reports corrections and regressions, because a correct `satisfied` to `not_addressed` flip is a correction and noise alone flips cells near the threshold (measured-behaviour.md). The script re-asks cells within 0.10 of the threshold once; a cell that disagrees with itself is `underspecified` with `unstable: true` and is not accepted. Record the acceptance as a comment on the question bead (`bd comment <question bead> ...`) with the run IDs.

## Testing a new anchor before the first gate run

Before any gate uses a freshly locked anchor: `lint` clean, fixtures written and approved, `check-fixtures` green with the defaults. Otherwise tune first; an untested question set produces findings nobody can act on.

## When to stop

- Stop when acceptance holds. Further specificity matches only today's wording of the bead.
- If three rounds do not reach acceptance, the statement is the problem: bundled, vague, or not expressible by a bead. Fix it (`update --statement`), re-lint, start again. Or the anchor is the wrong kind: a prohibition judged by `addressed` fails on every compliant bead; a structural anchor is a `shape` check, not a question (`anchor-capture.md`).
- Once every anchor has tuned questions, the planning-gate threshold may drop from 0.6 to 0.5 (measured-behaviour.md). Report the `leaning` of any cell still `underspecified`.
- If Jev's answers on the fixtures swing between runs at the same wording, the fixtures are ambiguous; rewrite them with the user.

A tuned question is a plan artifact: it lives in the question bead, versions with the work, and travels with the anchor on promotion to an ADR.
