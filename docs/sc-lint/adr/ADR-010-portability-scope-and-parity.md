# ADR-010 — Portability Scope And Parity

| Field | Value |
|---|---|
| ID | ADR-010 |
| Status | Accepted |
| Date | 2026-05-24 |
| Deciders | team-lead, clint |

## Context

`sc-lint-portability` currently owns the shipped shared portability rules
`PORT-001` through `PORT-005`. Those rules already catch several Unix-oriented
and general portability failure modes, but Phase `B` planning needs to clarify
where the next shared cross-platform families belong.

Consumer repositories such as `atm-core` may carry local wrappers or repo-local
portability checks, but `sc-lint` still needs one clear answer for which
future portability families belong in the product and which remain
consumer-local.

## Decision

1. `sc-lint-portability` remains the canonical home for consumer-neutral
   cross-platform path, environment-variable, shell-portability, and
   structural branch-parity rules.
2. When `sc-lint` ships or plans an OS-specific path-literal rule family for
   one major platform class, parity-companion detection for the matching
   opposite platform class belongs in the same shared portability crate when
   the semantics are consumer-neutral.
3. Repo-specific portability policies, such as local same-host adapter rules or
   project-specific shell conventions, do not migrate into `sc-lint`
   unchanged. They remain consumer-local unless generalized into a reusable
   product rule family.
4. Consumer wrappers or subset aliases may exist, but the primary product
   surface for shared portability checks remains `sc-lint lint sc-portability`.

## Consequences

- future Windows-only path literal parity work belongs in
  `sc-lint-portability`
- broader generic env-portability rules belong in `sc-lint-portability`
- shell-portability rules for OS-specific shell-path and shell-command
  assumptions belong in `sc-lint-portability`
- structural `cfg(unix)` / `cfg(windows)` parity enforcement belongs in
  `sc-lint-portability` when the rule remains consumer-neutral
- consumer repos may adapt wrappers such as `unix-gating`, but those wrappers
  do not redefine the core product boundary or ownership model
