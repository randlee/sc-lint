---
name: quality-workflow
version: 0.1.0
description: Judge whether REQ, NFR and ADR records are well written - the quality command, reading its verdict and answers, definitions files with compare-definition, and the offline checks validate, doctor and --dry-run.
---

# Quality workflow

`J` and `RAPTOR` as in SKILL.md; `index.json` from Step 2.

## 1. Judge

```bash
uv run -q --script "$J" quality --req REQ-ABC-0001 --nfr NFR-ABC-0001 --adr ADR-ABC-0001 --index index.json --run-name <name> > <name>.json
```

`--req`, `--nfr` and `--adr` repeat and mix. Every request and answer is also logged to `.sc/judge-jev/logs/judge-jev.log.jsonl`.

## 2. Read the result

```bash
jq -r 'if .success | not then "ERROR \(.error.code): \(.error.message)" else .data.results[] | "\(.record.id) verdict=\(.verdict)", (.waves[] | . as $w | .reason[] | if has("statement") then (select($w.verdict == "no" and (.statement.index as $i | any($w.matched_statements[]; . == $i))) | "  s\(.statement.index): \(.statement.text | gsub("\\s+"; " "))", (.answers[] | "    \(.id)=\(.choice) @\(.confidence) | \(.criterion)")) else "  \(.id)=\(.choice) @\(.confidence) | \(.criterion)" end) end' <name>.json
```

It prints each record's verdict and, for a failing REQ or NFR, every failing statement (`s<index>`) with all four answers as `question=choice @confidence | criterion`. For an ADR it prints its three answers.

What fails a statement (REQ, NFR):

- `obligations` is 2, 3 or 4+ **and** `joined_by` is `and-list`, `alternatives` or `obligation+rationale`;
- `term` is `project term defined nowhere`, `vague qualifier` or `undefined acronym`;
- `check` is `needs a measure` or `none`.

What fails an ADR: `context` is `background only` or `missing`; `alternatives` is `named only` or `none`; `consequences` is `positive only` or `missing`.

`success: false` is a tool error, not a verdict: rerun once; if it fails again, report the code and message verbatim.

## 3. Fix a failing record

Read `references/fix-strategy.md` and follow it.

## Definitions files

The default fix for an undefined term is a sentence in the record's own `Definitions:` paragraph (fix-strategy.md). A definitions file instead attaches a definition to records without editing them: use it when the record text must not change. It lives in `.sc/judge-jev/definitions.ttl`, attached to the record's Raptor IRI, `<repository URN>#<record id>` (the index's `metadata.repository`):

```turtle
@prefix jj: <urn:judge-jev:ontology#> .
@prefix d:  <urn:judge-jev:definition:> .
@prefix a:  <urn:judge-jev:attachment:> .

d:fast a jj:Definition ; jj:term "fast" ; jj:text "Finishes within 5 s on the reference host." ;
    jj:source "docs/requirements/perf.md" ; jj:attachedTo a:fast-1 .
a:fast-1 a jj:Attachment ; jj:record <urn:raptor:repo:example#REQ-ABC-0001> .
```

Check the file, then measure it:

```bash
uv run -q --script "$J" doctor --index index.json
uv run -q --script "$J" compare-definition --req REQ-ABC-0001 --index index.json --definition urn:judge-jev:definition:fast
```

`doctor` exits 2 on an invalid file and warns on an unattached definition or a record the index lacks. `compare-definition` judges the record without and with the candidates. Keep a definition only when `data.material` is `true`: an answer's choice flips, or its confidence crosses the threshold (default 0.6). Moving from one failing choice to another, or from 0.90 to 0.95, is not material. Jev is noisy below 0.5: confirm a material result with a second comparison before keeping it. Remove a definition that changes nothing.

## Offline checks

They call no Jev and need no key:

- `uv run -q --script "$J" quality --req REQ-ABC-0001 --index index.json --dry-run` prints the composed request: confirm what Jev will see.
- `uv run -q --script "$J" validate --index index.json` checks the index (`--tree <file>` checks a tree).
- `uv run -q --script "$J" doctor --index index.json` checks every tree and definitions file.
