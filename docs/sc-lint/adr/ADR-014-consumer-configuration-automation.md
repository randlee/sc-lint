# ADR-014 — Consumer Configuration Automation

| Field | Value |
| --- | --- |
| ID | ADR-014 |
| Status | Proposed |
| Date | 2026-08-14 |
| Deciders | team-lead, clint, product owner |
| Relates to | ADR-012, REQ-PRODUCT-023 through 025, REQ-CLI-025 through 028 |

## Context

ADR-012 established the correct safe contract for a new consumer repository:
`sc-lint init --just` renders a complete product-owned `Justfile`,
`sc-lint.toml`, and bootstrap helpers, and conflicts rather than overwriting
different files. Phase E delivered that contract.

The same safety boundary leaves an adoption gap for established repositories.
`sc-compose` demonstrates the effect: it retained an old private release
installer, source-archive download, copied `.just` Python utilities, two
independent version pins, and a manually reconstructed lint profile. Asking
each consumer team to resolve such a conflict manually recreates the product
complexity Phase E was meant to remove.

The product needs an adoption/replacement tool that is convenient for people
and agents while still protecting user-owned repository content.

## Decision

1. `sc-lint configure` is the consumer setup/replacement command. Its MVP uses
   a versioned JSON context/request/plan contract plus a small sc-lint-owned
   Python launcher. The launcher observes only conventional file presence
   (`Cargo.toml`, `sc-lint.toml`, `Justfile`, `.sc-lint/`, and
   `.github/workflows/`) and never parses arbitrary user integration, runs
   Cargo metadata, or executes a consumer command. Product validation and a
   later plan/apply transaction remain outside the UI adapter.
2. Agents use a JSON request (`--request <path|-> --json`); this is the stable
   noninteractive contract. Human users may use the optional Wyvern UI launched
   by that Python script. Its first page explicitly says what sc-lint will set
   up: detected conventional facts, the standard `just` commands, proposed
   files, uninspected existing integration, and recommendation rationales. The
   later pages are baseline, boundary, portability, runtime,
   attributes/directives, consumer command groups, Just, CI, and final review.
   Each page offers recommended settings that can be accepted, modified, or
   disabled. Attributes/directives are described as declarative source intent,
   not an executable analyzer profile.
3. The UI is optional. If Wyvern is requested but unavailable, the launcher
   reports a structured recovery error and JSON operation remains usable. The
   MVP contains no terminal implementation requirement and no new Rust wizard
   binary.
4. `init --just` retains ADR-012's exact empty-repository ownership and
   conflict behavior. `configure` may instead manage `.sc-lint/justfile` and
   one exact marked import block in an established root `Justfile`. Content
   outside the marker block is user-owned and byte-preserved. A missing,
   duplicated, moved, or changed marker is a conflict, never a whole-file
   rewrite.
5. Every change is previewed. Apply rechecks source digests, stages outputs,
   validates generated syntax, commits the bounded file transaction, and
   restores prior bytes and permissions on failure. F.4a owns one crate-private,
   object-safe `ManagedArtifact` transaction interface for all staged outputs;
   F.4b contributes a pre-validated `WorkflowYamlArtifact` through that same
   interface. This is not a public plugin or downstream extension surface.
   No README, Git commit, push, workflow dispatch, arbitrary Justfile content,
   or unknown workflow is in the write set.
6. `[tool.sc-lint].minimum_version` is the only product-version authority. The
   GitHub Action derives its artifact version from that config. An optional
   version field may assert equality for transition diagnostics but never select
   a different artifact.
7. Only a finite, versioned allowlist of legacy integration fingerprints may be
   transformed or deleted. An unrecognized or near-matching file returns a
   structured conflict and exportable patch/plan; it is not modified. The first
   acceptance fingerprint is sc-compose's documented 0.4 integration.
8. The first implementation is intentionally a usable but shallow MVP. It has
   no source scan, no Cargo metadata call, no Just/YAML parser, and no automatic
   migration claim. A later safe-transformer sprint may add a parser/fingerprint
   only when an acceptance fixture proves the exact behavior. Product growth is
   driven by real consumer gaps rather than speculative repository probes.

## CLI Boundary Registration

`configure` is a new public `sc-lint` CLI command family, not an internal
adapter. Its public parser root is `ConfigureCommand`; its structured failure
family is `ConfigureError`. Both are `BOUNDARY-ScLintCli` composition roots in
`boundaries/sc-lint/top-level-cli.toml`, alongside the existing top-level
`Command` and `CliError` roots. They retain the common machine envelope,
stable codes, path/cause context, recovery actions, and documentation links;
`ConfigureError` does not create a second unstructured error channel. The
private `ManagedArtifact` and `WorkflowYamlArtifact` transaction mechanism is
separate from those public roots and remains governed by this same ADR.

## Consequences

- New repositories retain the smallest Phase E path; established repositories
  gain a safe product-owned path rather than a manual migration guide.
- JSON requests make the full operation reliable for coding agents, CI
  preparation, and code review; no automation depends on screen scraping. A
  Claude Code skill drives the same context-to-JSON-to-preview flow.
- The product maintains fixtures for its small conventional context contract.
  An unsupported consumer shape is shown as uninspected and is a product
  finding, not permission to add a bespoke probe or workaround.
- The transaction remains one evolvable internal boundary: a synthetic second
  artifact and the real YAML workflow both prove that adding an artifact cannot
  create a second staging/rollback path. This internal mechanism is governed by
  this ADR rather than a new public-API ADR.
- The action's independent `version` input is removed/limited to assertion,
  requiring compatible Action, documentation, and fixture changes.
- `sc-compose` and `atm-core` are Phase P reference consumers. Phase P proves
  the exact released artifact in disposable copies of both before either
  consumer conversion PR can claim completion; neither receives wizard
  implementation code or a special permanent integration branch.

## Alternatives Rejected

### Continue with `init --just` plus manual conflict instructions

Safe but fails the product goal: existing repositories still need experts to
edit Justfiles, CI, installers, and profiles. It preserves sc-compose's
complexity for every next consumer.

### Always replace the root Justfile

Simple to implement but unacceptable: it destroys a consumer's broader command
surface and contradicts ADR-012's ownership guarantee.

### Make Wyvern the configuration engine

This would make automation depend on a graphical/interactive tool and put
product policy outside the product request/plan contract. Wyvern is a
schema-driven explanatory adapter over that contract.

### Ask consumers to vendor a shared Just/Python module

This recreates source-layout coupling and violates the Phase E contract that
consumers use released product behavior, not copied implementation scripts.
