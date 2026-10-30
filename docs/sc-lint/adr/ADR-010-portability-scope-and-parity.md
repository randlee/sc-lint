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
5. New production-scope portability leaf rules stay consumer-neutral by using
   built-in pattern detection rather than repo-configured allowlists. Existing
   config-driven behavior in `PORT-002` remains test-scope-only and does not
   set the suppression model for future production env, path, or shell rules.
6. [Narrowed by ADR-020 decision 1, phase L: the cfg set and the PORT-008 rule are
   defined there.] Explicit `#[cfg(unix)]` production boundaries are treated as structural
   parity questions for the shared portability family. Leaf rules may suppress
   inside those items, with the companion-or-fallback decision owned by the
   structural parity rule family instead of parallel per-rule allowlists.

## Consequences

- future Windows-only path literal parity work belongs in
  `sc-lint-portability`
- broader generic env-portability rules belong in `sc-lint-portability`
- future production env-portability rules are unconditional on ungated
  production lookups and do not introduce a new repo-configured abstraction
  allowlist
- shell-portability rules for OS-specific shell-path and shell-command
  assumptions belong in `sc-lint-portability`
- structural `cfg(unix)` / `cfg(windows)` parity enforcement belongs in
  `sc-lint-portability` when the rule remains consumer-neutral
- explicit Unix-gated production items may suppress leaf portability findings,
  but that suppression must be backed by the shared structural parity rule
  family rather than ad hoc rule-local config
- consumer repos may adapt wrappers such as `unix-gating`, but those wrappers
  do not redefine the core product boundary or ownership model

## Amendment 2026-10-10 — portability keys in the analyzer table

| Field | Value |
| --- | --- |
| Status | Accepted (Rand, 2026-10-10) |
| Source | #202 |

1. Extends Decision 5: `PORT-001` and `PORT-006` share one Unix path prefix
   list. `unix_path_prefixes` in `[tool.sc-lint.analyzers.portability]`
   extends the built-in list; the built-in prefixes always apply. A configured
   prefix adds detection and never suppresses a finding.
2. Extends Decision 5: `PORT-002` reads `config_home_env` from the same
   `[tool.sc-lint.analyzers.portability]` table.
3. The legacy top-level `[portability]` table remains a compatibility fallback
   only for `config_home_env`; a value in the analyzer table takes precedence.
   Both tables require `config_home_env` to be a string and reject a wrong type
   through the analyzer configuration error path.
4. The rest of Decision 5 stands.

## Amendment 2026-10-12 — reviewed Phase K adapter pairs

ADR-017 (Accepted by Rand) extends Decisions 3, 5 and 6 for the consumer-neutral
PORT-010 structural parity rule. A consumer may declare reviewed
`platform_adapters` pairs under its portability analyzer table. After validation,
a pair covers its enclosing differently named Unix/Windows module declarations
and inherited-private implementation items. Unrelated declarations and separately
Unix-gated public/restricted-visible items retain ordinary parity checks.
Absent pairs preserve existing findings; malformed declarations are configuration
errors. This is narrow, explicit adapter configuration, not a general
repo-configured suppression of production portability leaf rules. See
[ADR-017, change 1](./ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).

## Amendment 2026-10-30 — PORT-010 companion shapes

| Field | Value |
| --- | --- |
| Status | Pending (Rand) |
| Source | #720, #786 ([shape table](https://github.com/randlee/sc-lint/issues/786#issuecomment-6513016641)); ADR-022 |

This amendment extends Decision 6. The structural parity rule (PORT-010)
recognizes the shapes below as a companion or portable fallback and does not
report the `#[cfg(unix)]` item. All matching is syntactic over the package
source; there is no rustc or cargo subprocess, no Boolean cfg equivalence and
no name heuristic.

- **Reported items.** PORT-010 reports `fn` items, `mod` declarations and
  inherent `impl` methods. A shape suppresses the finding on one of these.
- **Gated.** A Unix gate is `#[cfg(unix)]` or `#[cfg(any(unix, test))]`. A
  non-Unix gate is `#[cfg(windows)]`, `#[cfg(not(unix))]`,
  `#[cfg(any(windows, test))]` or `#[cfg(any(not(unix), test))]`. The `test`
  alternative keeps the item in production builds on that platform, so it
  counts. Stacked predicates, `all(...)` and `not(windows)` do not count.
- **References.** A reference from code gated `#[cfg(test)]`, inside a
  `#[cfg(test)]` module, or in a `#[test]` function is ignored. A shape that
  needs "at least one reference" needs one production reference.

1. **S3, same-name sibling.** A non-test item of the same kind and name gated
   `#[cfg(not(unix))]` or `#[cfg(any(not(unix), test))]` in the same lexical scope: crate root, inline module,
   file module or the same inherent `impl` block. Trait impls keep their
   existing treatment.
2. **S1, helper of paired callers.** A private or `pub(crate)` helper with at
   least one reference, where every reference resolves to a call inside a
   Unix-gated item that itself has a companion by any rule in this amendment,
   including S1. S1 is computed as a fixed point over the package; a cycle
   with no other companion gives no proof. Resolved forms are direct path calls through `use` aliases,
   `self::`, `crate::`, `super::`, `Self::` and `Type::`. Method-receiver
   calls, function pointers and calls inside macro token streams are
   unresolved.
3. **S5, helper inside cfg blocks.** A helper with at least one reference,
   where every reference sits inside a `#[cfg(unix)]`-attributed block,
   statement or expression of a function whose body also has a
   `#[cfg(not(unix))]`-attributed block or statement. `cfg!()` does not count.
   S5 has no visibility restriction.
4. **S2, renamed companion.** A Unix-gated function and a non-Unix-gated
   function with the same return type; parameters may differ. Each is called only from the matching gated branch of
   one caller. The caller may be one function with gated blocks or a gated
   item pair from S3. Neither function is called from anywhere else.
5. **S4, portable sibling.** A Unix-gated transport or adapter function whose
   every production reference is in a Unix-gated branch of one caller, where
   that caller's non-Unix branch calls an un-gated function or constructs an
   un-gated enum variant instead (for example loopback TCP). When the caller
   dispatches through a `match`, the Unix-gated arm calls the function and an
   un-gated arm handles the alternative. The reported item is the function;
   match arms are never reported themselves.
6. **S7, rejected configuration.** A `#[cfg(unix)]` accessor of a
   configuration type `T`, with a `#[cfg(not(unix))]` function or block in
   the same scope that takes `T` (as a parameter or `Self`) and whose every
   return path is `Err(..)`.
7. **S6, unix-only module.** The declaration `#[cfg(unix)] mod x;` and the
   `#[cfg(unix)]` items inside that module tree. Out-of-line resolution
   follows `x.rs`, `x/mod.rs` and `#[path]`. An item in a module that is
   compiled on Windows is not covered.

**Fail closed.** An unresolved reference, an ambiguous match or a missing
module file gives no proof, and the finding stays. A production
`#[cfg(unix)]` item with no companion and no portable fallback is still
reported.

**Per-site allowance.** A deliberate POSIX-only item with no parity concept
(#786 S8) uses `#[sc_lint(allow(PORT-010), reason = "...")]` from ADR-022
decision 4. The report lists each honored allowance with its reason.

**Open for Rand.** Whether S6 covers `pub` modules and `pub` items, or only
non-public modules, with `pub` ones needing a site allowance. Withdrawn
ADR-021 decision 7 would have kept S6 reported.
