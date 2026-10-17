# ADR-012 — Consumer Adoption And Just Contract

| Field | Value |
| --- | --- |
| ID | ADR-012 |
| Status | Proposed revision (2026-10-16), pending Rand's approval; the 2026-08-11 Accepted text is in git history |
| Date | 2026-08-11, revised 2026-10-16 |
| Deciders | Rand |

## Context

Consumer repositories need a small interface that agents and developers can
run without knowing this repository's Cargo packages or Python adapters.
`just` is already in use on 10-20 consumer repositories. The consumer owns its
`Justfile`; sc-lint supplies recommended recipes and naming conventions.

A bootstrap written once per platform cannot be verified once: the POSIX
script and the PowerShell companion diverged in behavior, and `pwsh -Command`
reports every native failure as exit 1. Rand: "the design is python"; "pwsh
fails on mac erratically. you can't test scripts that are different between
platforms."

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
   unchanged. PowerShell is not used.
3. **Kit recipes.** The managed import block defines `setup`, `lint` and
   `upgrade`. sc-lint provides a recommended `test` recipe calling the
   bootstrap's `test` operation; the installer adds it outside the block only when the
   consumer `Justfile` has none, and never changes an existing one. A
   repository with no `Justfile` starts from sc-lint's default one, which
   runs `lint` by default.
4. **Generation.** `sc-lint init --just` creates or updates only
   `sc-lint.toml`, `Justfile` and `.sc-lint/bootstrap.py`. It never writes a
   consumer `README.md` and reports a conflict rather than overwriting a
   user-owned file. A managed file from an earlier release is replaced only
   when its bytes match a shipped version.
5. `--check` and `--dry-run` are non-mutating. Re-running a current
   integration is idempotent and reports the managed files.
6. Source-maintainer recipes stay in this repository's root `Justfile`.
   Source versus consumer behavior is selected by the generated integration,
   never inferred from a directory name, Cargo manifest or package.
7. sc-lint generates no consumer CI workflow; consumer CI uses the reusable
   `setup-sc-lint` Action.

## Consequences

- One test suite run on Linux, macOS and Windows verifies the bootstrap on
  every platform.
- Python is required on every platform; it already is, for the managed wheel
  (ADR-016). A missing interpreter stops with
  `CLI.SC_LINT_PYTHON_UNAVAILABLE`.
- Missing or incompatible installations stop `lint` and `test` before work
  begins, with a structured recovery message.
- `just lint` runs complete configured profiles, never an advisory subset.
