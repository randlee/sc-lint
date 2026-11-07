# ADR-012 — Consumer Adoption And Just Contract

| Field | Value |
| --- | --- |
| ID | ADR-012 |
| Status | Accepted; decision 3 amended 2026-11-07, approved by Rand (ADR-023) |
| Approved by | Rand, 2026-10-16 |
| Date | 2026-08-11, revised 2026-10-16 |
| Deciders | Rand |

## Context

Consumer repositories need a small interface that agents and developers can
run without knowing this repository's Cargo packages or Python adapters.
`just` is already in use on 10-20 consumer repositories. The consumer owns its
`Justfile`; sc-lint supplies recommended recipes and naming conventions.

A helper that is a different program on each platform cannot be verified by
one test suite.

## Decision

1. **One bootstrap.** `.sc-lint/bootstrap.py` is the only generated helper. It
   is one Python program, identical on Linux, macOS and Windows, with the
   operations `setup`, `lint`, `test`, `upgrade` and `exec`. It resolves
   `SC_LINT_BIN`, the managed binary and `PATH`; when no compatible
   installation exists, `setup` installs the verified configured release
   before product work starts. Its own exit codes are stable, and a child's
   exit code is returned unchanged. There is no per-platform script.
2. **Recipes call it the same way on every OS:**
   `<python> .sc-lint/bootstrap.py <operation> --config sc-lint.toml`, where
   `<python>` is `python` on Windows and `python3` elsewhere. On Windows,
   `just` runs recipes with `cmd.exe /c`, which returns the child's exit code
   unchanged. When the interpreter is missing, the shell's command-not-found
   failure is reported before the bootstrap runs; an interpreter older than
   3.9 makes the bootstrap stop with `CLI.SC_LINT_PYTHON_UNAVAILABLE`.
3. `--dry-run` is non-mutating. Re-running a current integration is
   idempotent. (`--check` removed by ADR-023 amendment 2026-11-07.)
4. Source-maintainer recipes stay in this repository's root `Justfile`.
   Source versus consumer behavior is selected by the generated integration,
   never inferred from a directory name, Cargo manifest or package.
8. sc-lint generates no consumer CI workflow; consumer CI uses the reusable
   `setup-sc-lint` Action.

## Consequences

- One test suite run on Linux, macOS and Windows verifies the bootstrap on
  every platform.
- Python 3.9 or newer is required on every platform (amended 2026-11-04: Rand's
  ruling that Python 3.9 is supported, with the wheel floor per GitHub #778,
  `cp39-abi3`). Windows ARM64 still requires Python 3.11 or newer (ADR-016).
- Missing or incompatible installations stop `lint` and `test` before work
  begins, with a structured recovery message.
- `just lint` runs complete configured profiles, never an advisory subset.
