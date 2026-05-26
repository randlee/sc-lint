# ADR-008 — sc-observability Integration and Logging Layer Boundaries

| Field | Value |
|---|---|
| ID | ADR-008 |
| Status | Accepted |
| Date | 2026-05-10 |
| Deciders | team-lead, clint |

## Context
sc-lint uses sc-observability for structured JSON logging. The crate has both a library target (`sc_lint`) and a binary target (`sc-lint`) in a single Cargo.toml. This creates tension between keeping the library free of observability dependencies and providing a clean logging API for the binary.

## Decisions

### ARCH-004 — sc-observability in [dependencies] (not binary-only)
sc-observability lives in `[dependencies]` rather than being scoped to the binary only. This is an accepted trade-off: Cargo does not support binary-only dependencies in a mixed lib+bin crate. Enforcement is by convention and arch-qa, not the build system.

### RULE-001 — sc-observability imports confined to binary modules
No lib crate module (anything other than `main.rs` and `logging.rs`) may import sc-observability. Library types must not hold or reference sc-observability types directly. Violations are tracked as `ARCH-010` findings.

### RULE-002 — No custom emit_* wrapper functions
Custom wrapper functions named `emit_*` are forbidden. The single approved dispatch helper is `dispatch_event` (formerly `emit_event`, renamed per `ARCH-006/011`). This prevents proliferation of logging indirection.

### CLI-LAYER-OWNS-LOGGER-INITIALIZATION
The logger must be initialized in `main.rs` before any operation. Library code must not initialize or depend on a logger being present. Functions accessible from library code must not require or assume logging is initialized.

### ServiceName seam pattern
The library defines its own `pub(crate)` `ServiceName` newtype in `contract.rs`. Conversion to `sc_observability::ServiceName` happens only at the `main.rs`/`logging.rs` binary boundary. This keeps `RULE-001` clean.

### OnceLock metadata cells
Lazy-initialized `OnceLock` metadata cells (service name, crate version, etc.) are held in the binary module tree. They are initialized once per process in `main.rs` and read by `dispatch_event` at each event emission.

### RELEASE-001 — Release-line maintenance stays inside the CLI seam
Compatibility work for later supported `sc-observability` releases must keep
the existing CLI-owned logger seam intact. If retained-log maintenance is
enabled, rotation/pruning/background cleanup remains logger-owned behavior,
and deprecated event-emission APIs must be replaced by explicit `log` or
`try_log` call-site decisions rather than new wrapper layers.

## Consequences
- Library types are clean of sc-observability imports ✓
- The binary module tree can use sc-observability freely ✓
- ARCH-004 means sc-observability is a compile-time dependency even for library consumers (accepted) ✗
