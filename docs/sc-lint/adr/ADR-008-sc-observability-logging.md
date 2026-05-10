# ADR-008 — `sc-observability` For `sc-lint` Structured Logging

| Field | Value |
|---|---|
| ID | ADR-008 |
| Status | **Accepted** |
| Date | 2026-05-09 |
| Deciders | team-lead, quality-mgr, clint |
| Relates to | REQ-LOG-001 through REQ-LOG-005 |

---

## Context

`sc-lint` needs a structured logging baseline for the top-level CLI before the
first delegated backend integrations land.

The logging design needs to satisfy the current product requirements:

- CLI-owned logger initialization at startup
- default file logging with opt-in console logging
- structured entry, completion, and error events
- service-scoped log roots
- no backend-owned logger runtime construction

The repo already has `sc-observability` available in the local ecosystem, so
the main question is whether `sc-lint` should standardize on that crate or
introduce a fresh logging stack.

## Decision Drivers

- the product already has a nearby structured-logging implementation to reuse
- the chosen runtime must support file and console sinks without extra glue
- JSONL output is the desired default file format for machine-readable logs
- service-name scoping should be first-class rather than bolted on later
- the Phase A rollout should avoid inventing a second observability framework

## Options Considered

1. Use `sc-observability` as the structured logging runtime for the top-level
   `sc-lint` binary.
2. Build a new `tracing` + subscriber stack directly in `sc-lint`.
3. Build a `log`-crate-based wrapper with custom sink and file-layout logic.

## Decision

`sc-lint` adopts option 1.

The top-level CLI will use `sc-observability` for structured logging because
it already exists in the ecosystem and directly provides the baseline features
this phase needs:

- `LoggerBuilder` for explicit runtime assembly
- built-in file and console sinks
- JSONL file output support
- service-name-aware configuration and routing

This decision applies to the `sc-lint` binary process only. Subprocess
backends, including Python tools still used in Sprint `A.3`, run in separate
processes and are not governed by the CLI logger runtime. Their stdout/stderr
handling remains a separate concern defined in the `A.3` dispatch design.

## Consequences

### Positive

- `sc-lint` can reuse an existing runtime instead of creating a new logging
  framework
- file and console sinks are available without extra bootstrap code
- JSONL output aligns with the machine-readable logging requirement
- service-name scoping aligns with the planned `sc-lint` and analyzer command
  surfaces

### Negative

- `sc-lint` now depends on a local path-managed observability crate during the
  current workspace phase
- the CLI integration must adapt the sink layout to the desired
  `~/sc-lint/logs/<service>/` directory shape
- subprocess backend logging remains outside this runtime and still needs a
  separate stdout/stderr handling design

## Alternatives Rejected

### `tracing` + custom subscribers

This would be viable, but it recreates sink assembly, file-layout policy, and
service-scoping decisions that `sc-observability` already packages for the
local ecosystem.

### `log` + custom wrapper

This keeps the base dependency smaller, but it pushes JSONL formatting,
service-scoped file routing, and console/file sink coordination into `sc-lint`
itself with no product benefit for Phase `A`.

## Follow-Up

| Action | Owner | Gate |
|---|---|---|
| Keep `docs/sc-lint/logging.md` aligned with the `sc-observability` runtime surface and ownership model. | sc-lint implementation owner | Before A.1a implementation starts |
| Keep logger initialization in the top-level CLI and out of backend crates. | sc-lint implementation owner | Ongoing through A.6 |
| Define separate subprocess stdout/stderr handling in the A.3 backend-dispatch work. | sc-lint implementation owner | Before A.3 closes |

*ADR-008 | sc-lint | 2026-05-09*
