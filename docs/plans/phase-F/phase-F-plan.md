---
title: Phase F Plan — Consumer Setup Automation
status: planned
branch: feature/setup-automation-planning
target: develop
---

# Phase F Plan — Consumer Setup Automation

## Goal

Make adoption of the released `sc-lint` product a guided, reversible product
operation rather than a consumer-maintained integration project. A new Rust
repository and established repositories such as `sc-compose` and `atm-core`
must be able to reach the same committed end state without copying `.just`
Python utilities, writing an installer, selecting analyzer binaries,
reconstructing a profile, or hand-editing a CI workflow. The product is not
considered proven merely because its own fixtures pass: Phase P owns proof
against those two real consumer layouts before either adoption PR may merge.

The user-facing entry point is `sc-lint configure`. The MVP is deliberately a
small, transparent setup assistant: it reads a handful of conventional Rust
repository paths, explains what it found and what it proposes to set up, then
emits a reviewable JSON plan. It is not a general repository-analysis system.
The first wizard driver is a sc-lint-owned Python launcher around Wyvern; the
existing released `sc-lint` command owns product validation and later apply
semantics.

```text
interactive human:  sc-lint configure --root <repo> --ui wyvern
agent / automation: sc-lint configure --root <repo> --request setup.json --json
safe preview:       sc-lint configure --root <repo> --request setup.json --dry-run --json
apply (F.4):        sc-lint configure --root <repo> --request setup.json --apply --json
```

`sc-lint init --just` remains the narrow, empty-repository initializer from
ADR-012. `configure` is the replacement/adoption surface: it understands an
existing repository, gathers or accepts explicit policy decisions, produces a
complete change plan, and applies only approved, understood changes.

## Why A Separate Phase Is Necessary

Phase E correctly removed source-checkout orchestration from the intended
consumer contract, but its initializer treats a differing `Justfile` as an
unrecoverable conflict. That is safe, but it does not make adoption easy for
the repositories that most need it. `sc-compose` demonstrates the practical
result: it has a release pin in two places, a private installer, copied
`.just/*.py` utilities, and a hand-assembled CI lint profile. Those are
product-adoption defects, not complexity a consumer team should own.

Phase F does **not** add setup code to `sc-compose`. It moves the missing
conversion capability into `sc-lint`, publishes the released artifact and its
contract, then hands the artifact to Phase P for dual-reference qualification
and consumer-owned conversions. A sc-compose-only conversion is not sufficient
product proof.

## Locked Product Decisions

1. The MVP uses a versioned JSON context/request/plan boundary. A small Python
   launcher performs only the documented conventional-path checks and maps the
   resulting data through Wyvern or agent JSON; it does not parse arbitrary
   Justfiles, workflows, or source code, run Cargo metadata, or execute a
   consumer command. Product validation, mutations, profile selection, and CI
   generation remain outside the UI and in the released product boundary.
2. Every noninteractive use accepts a versioned JSON request and returns the
   normal top-level JSON envelope. Agents never scrape terminal prompts or UI
   markup.
3. The interactive flow first answers “what am I setting up?”: it displays the
   repository facts, the standard sc-lint developer contract (`just setup`,
   `just lint`, `just test`, `just upgrade`), every proposed file/change, and
   the reason for each recommendation. It then presents one confirmable page
   per lint family and integration choice. A final page displays the exact JSON
   plan and requires confirmation before any later apply operation.
4. Wyvern is the MVP web UI and schema-driven dialog host, launched by the
   sc-lint-owned Python script. It is optional: an agent supplies the same JSON
   and a human receives a JSON recovery path. An unavailable
   Wyvern must never block agent or CI use and is an error only when explicitly
   requested.
5. The configuration file has exactly one compatibility authority:
   `[tool.sc-lint].minimum_version`. The Action reads and preflights that value;
   it must not require a second independent product-version input.
6. Product-managed integration lives under `.sc-lint/`. For an established
   Justfile, the tool manages `.sc-lint/justfile` and one marked import block
   in the root Justfile **only when the root does not already define a reserved
   recipe**. It never silently replaces arbitrary user content. The root file
   remains owner-controlled outside that block.
7. Existing `lint`/`test` recipe collisions are explicit migration choices.
   The tool may apply a tested migration only when it recognizes a supported
   legacy shape (including the sc-compose 0.4 shape); otherwise it writes
   nothing and returns a structured plan conflict with an exportable patch.
   There is no heuristic deletion of commands or workflow steps.
8. Configuration is transactional: discovery produces no writes; `--dry-run`
   produces an ordered file-operation plan and unified diffs; apply rechecks
   file digests, writes staged replacements, validates syntax, and rolls back
   every product-managed write if validation fails. Git commits, pushes, and
   workflow dispatch remain outside the write set.
9. Consumer profile entries stay argv arrays. The wizard recommends commands
   from discovered project facts but never turns shell text, an agent response,
   or a Wyvern answer into shell execution.
10. The recommended baseline profile is explicit argv data for format, Clippy,
    and tests. Boundary, portability, and runtime selections add explicit
    installed-product `sc-lint lint <family>` argv steps; they never call Cargo
    packages or source scripts. The attributes/directives page is informational
    and configures declarative source intent only until it has a real
   consumer-profile runner. Command groups may add only validated argv arrays.
11. MVP probing is strictly bounded to: root `Cargo.toml` presence and its
    workspace/package marker, `sc-lint.toml` presence, root `Justfile` presence,
    `.github/workflows/` presence, and existing `.sc-lint/` presence. These are
    facts for the UI, not a compatibility verdict. Anything ambiguous is shown
    as “not inspected” and requires a user/agent choice; it must not trigger
    bespoke probing code or an inferred rewrite.

## Relationships To Existing Decisions

ADR-012 remains correct for the empty-repository initializer but cannot govern
an established repository's scoped Justfile integration. F.1 adds ADR-014,
which supersedes ADR-012 only for `configure` and migration ownership. It does
not weaken ADR-012's prohibition on overwriting user-owned files.

Phase F extends, rather than reopens, Phase E requirements:

| Requirement | Phase F addition | Owning sprint | Closure evidence |
| --- | --- | --- | --- |
| `REQ-PRODUCT-019` | one version authority is retained during conversion | F.1, F.4, F.5 | request/config/Action drift fixtures |
| `REQ-PRODUCT-020` | configured output uses existing verified setup/upgrade | F.2, F.5 | clean-machine lifecycle fixtures |
| `REQ-PRODUCT-022` | Action version derives from config; optional workflow patch is safe | F.4, F.5 | Action and workflow fixtures |
| `REQ-PRODUCT-023` (new) | deterministic configuration/adoption planning and transaction | F.1-F.5 | JSON, conflict, rollback, cross-platform fixtures |
| `REQ-CLI-025`–`028` (new) | configure command, request schema, plan schema, stable errors | F.1-F.3e | CLI contract and schema tests |

## Consumer End State

After a successful conversion, a consumer commits only consumer policy and
small product-managed integration assets:

```text
sc-lint.toml                 # one SemVer floor and explicit argv profiles
.sc-lint/bootstrap*          # existing verified-product resolver
.sc-lint/justfile            # generated recipe module for established repos
Justfile                     # its existing content plus a marked import block
.github/workflows/sc-lint.yml # optional generated Action workflow
```

The everyday contract is intentionally boring:

```text
just setup
just lint
just test
just upgrade
```

The generated recipes call only the installed product. They never invoke a
source checkout, `cargo run -p sc-lint-*`, an analyzer sibling binary, copied
Python, or a consumer-local installer.

## Phase F And Phase P Ownership

Phase F owns the product contract, schemas, configuration engine, bounded
transformers, release artifact, product fixtures, and installed documentation.
It does not own a production change in either reference consumer.

Phase P owns qualification of that exact released artifact in disposable copies
of **both** `sc-compose` and `atm-core`, followed by separate consumer-owned
PRs. P.1 must prove preview, apply, reapply, the four public `just` commands,
and the operating-system CI matrix for both layouts before P.2 or P.3 begins.
An unsupported reserved-recipe shape is a product defect returned to Phase F;
it is never solved by a consumer-local script, manual Justfile edit, or a
sc-compose-only exception.

Consequently, F.5 prepares the release/documentation handoff and may validate
product fixtures, but it cannot claim a reference-consumer conversion or close
the real-consumer proof. The Phase P plan is the authority for that evidence.

## Sprint Sequence And Dependency Graph

```text
F.1 contracts / ADR / requirements
 └─ F.2 shallow context + deterministic JSON plan
     └─ F.3a wizard UX/specification contract
         └─ F.3b released Wyvern wizard capability gate
             └─ F.3c agent JSON and setup skill
                 └─ F.3d thin launcher and page implementation
                     └─ F.3e wizard acceptance, accessibility, and docs
                         └─ F.4 safe Just/config/Action transformers
                             └─ F.5 release, documentation, and Phase P qualification handoff
                                 └─ P.1 dual-reference released-artifact qualification
                                     ├─ P.2 sc-compose consumer PR
                                     └─ P.3 atm-core consumer PR
```

### F.1 — Contract And Architecture Foundation

Lock request/plan schemas, ownership boundaries, errors, requirements,
ADR-014, and testable mutation semantics before implementation.

### F.2 — Shallow Repository Context And Deterministic Plan

Build the bounded conventional-path observation and no-write JSON plan that
the wizard presents. It deliberately does not become a repository parser.

### F.3a–F.3e — Fully Specified, Capability-Gated Wyvern Wizard

The original overloaded F.3 is split before implementation. F.3a writes the
field-level UX contract and handoff package; F.3b proves a **released** Wyvern
wizard capability; F.3c delivers the equivalent agent JSON contract; F.3d
implements only a thin adapter and static pages; and F.3e validates the human
experience. Wyvern 0.1.0's single-dialog API is not sufficient evidence for
this flow. `next`/`back` are adequate only when the released dependency also
provides browser-history data restoration, branching, cancel/dismiss, finish,
and headless-test behavior. sc-lint must not replace missing capability with a
Python state machine or an ad-hoc browser application.

- [F.3a UX contract and Wyvern handoff](sprint-F3.md)
- [F.3b released Wyvern capability gate](sprint-F3b-wyvern-capability-gate.md)
- [F.3c agent JSON and setup skill](sprint-F3c-agent-json-and-skill.md)
- [F.3d thin launcher and page implementation](sprint-F3d-wizard-adapter-and-pages.md)
- [F.3e wizard acceptance and documentation](sprint-F3e-wizard-acceptance-and-docs.md)

### F.4 — Safe Integration And CI Replacement Transformers

Implement staged apply/rollback, marked Justfile integration, generated
config/bootstrap/Action workflow, and only explicitly supported legacy
migrations.

### F.5 — Release, Documentation, And Phase P Qualification Handoff

Publish the product contract and its complete supporting documentation, then
hand its exact release artifact to Phase P. Phase P—not this phase—uses that
artifact to qualify and convert `sc-compose` and `atm-core`. Any step that
cannot be automated in Phase P is a Phase F product defect, not a consumer
manual task.

## Required Validation Lanes

| Lane | Required proof |
| --- | --- |
| Empty repo | `configure` produces the canonical integration and a clean machine passes setup/lint/test. |
| Existing Justfile | marked import is idempotent; unrelated recipes and comments are byte-preserved. |
| Collision/conflict | unknown user-owned `lint`/`test` or CI shapes cause no write and emit typed remediation/patch data. |
| Agent JSON | valid request, malformed request, unsupported schema, and explicit choices are deterministic without a TTY. |
| Wizard UX contract | every page has exact fields, defaults, schema mappings, validation, navigation, copy, and no-write/error states before implementation. |
| Wizard capability | the pinned released Wyvern artifact proves browser-history restoration, branching, cancel/dismiss, finish, and headless execution; its v0.1 single-dialog mode is not a substitute. |
| Wizard | the thin Python/Wyvern dialog and agent JSON yield the same request/plan; every page explains proposed setup and cancellation produces no write. |
| CI | generated Action workflow has exactly one product-version authority and no source/cargo/Python fallback. |
| sc-compose | supported legacy shape converts without manual edits; old copied scripts and custom source installer are absent afterward. |
| Safety | injected write/validation failures roll back; no README, arbitrary Justfile text, or unrelated workflow is overwritten. |
| Platform | all applicable empty/existing/agent/Action fixtures run on Linux, macOS, and Windows. |

## Out Of Scope

- putting Wyvern, the launcher, or any wizard code in `sc-compose`;
- automatically committing, pushing, opening PRs, or changing GitHub secrets;
- inferring project policy from LLM output or executing unvalidated shell text;
- migration of arbitrary third-party Justfile semantics by destructive rewrite;
- changing analyzer rule behavior merely because a page exposes it;
- retaining copied 0.4 source utilities as a fallback after acceptance.
- implementing a multi-page UI on top of Wyvern 0.1's single-dialog API or
  recreating Wyvern's history/state management in Python.

## Phase Exit Criteria

- a human can configure a consumer through one page per lint family, inspect a
  final diff, and apply it without manually wiring Just or CI;
- an agent can send the equivalent versioned JSON request and receive the same
  stable plan/result schema;
- fresh and established repository paths are safe, deterministic, and tested;
- the exact released artifact, schemas, offline documentation, and product
  fixture evidence are available to Phase P; Phase P must still prove the
  dual-consumer adoption outcome before a reference-consumer conversion can be
  claimed;
- all user/developer/agent/CI instructions describe only the public commands;
- `just lint` and `just test` are complete configured gates on all supported
  platforms.
