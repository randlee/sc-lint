# ADR-017 — Phase K consumer contracts

Status: Accepted — Rand, 2026-10-12 (session 01a12885-fd21-7043-a747-0d6f9e4d9869).
Approval: “approved.” following review of this record.

Related: ADR-004, ADR-005, ADR-010, ADR-012, ADR-013, ADR-015, ADR-016.
Plan authority: `lint-phase-k` and sprint beads listed in `docs/plans/phase-k.jsonl`.

## Intent

Close the gaps reported by fenix@atm-dev in a general-purpose product. Reuse
applicable atm-core implementations and regression tests. Keep lint semantics
in Rust and Python bindings thin; installation/rendering and existing external
process adapters retain their existing responsibilities. Do not introduce a
second consumer runner or copy atm-core policy values into product defaults.

## Accepted changes to existing contracts

1. Extend ADR-015 analyzer keys and REQ-PRODUCT-023 with the common
   `exclude_paths_by_rule` map. Portability also accepts reviewed
   `platform_adapters`; boundary accepts the optional upstream manifest policy
   families. Missing fields retain existing behavior. Unknown rule names and
   malformed values remain configuration errors. Per-rule exclusions filter
   findings, not entire files. Analyzer-owned adapters supply explicit normalized
   paths before converting their private findings; the shared Finding schema
   stays unchanged. Validated adapter pairs cover their enclosing differently
   named module declarations and inherited-private implementation items, while
   unrelated and separately Unix-gated public items retain normal parity checks.
2. Extend REQ-SCB-017 to aggregate incoming dependent allowlists per canonical
   owner package. For records i of owner P, runtime permissions are the union of R_i =
   `allowed_dependents`. Dev-only permissions union each supplied D_i =
   `allowed_dev_dependents`, falling back to R_i only for records omitting D_i.
   Explicit D_i=[] grants no dev permission from that record; it does not revoke
   another record's grant. Normal/build dominates mixed edges. Staleness uses
   only edge categories to which an entry contributes and is deduplicated by
   canonical owner/dependent; preserve the upstream active-record gate.
   REQ-SCB-019's third-party exclusion does not prevent explicitly configured
   manifest policy under `--rule manifests`; package edges remain distinct.
3. Extend inventory validation under REQ-SCB-020/023. Preserve existing local
   composition-root symbols; only resolved external crate-qualified roots
   participate in dependent-policy validation. Report each new semantic defect
   as a structured validation finding after parsing, with record/field context.
   Preserve fatal syntax, schema, layout and I/O errors and public loader
   signatures. A combined-invalid fixture must demonstrate that one semantic
   defect does not hide the others (#377).
4. Extend the existing Rust consumer lint executor with the named lint-profile
   membership and host/all/platform selector required by #199. Existing list
   forms remain accepted. Named profile membership is an ordered list of step
   names, distinct from `--platform-scope host|all|linux|macos|windows`; existing
   `--profile all` keeps its all-step meaning. Bootstrap accepts lint selector
   followed by optional scope. Platform denotes the intended target OS and must
   match the target triple. Only literal cargo check/clippy target commands are
   translated. Native targets use cargo; Linux/macOS to x86_64-pc-windows-msvc
   uses cargo xwin. Other cross combinations report explicit unsupported skips;
   missing tools or failures for supported combinations are failures.
   Generic test groups, parallel execution, keep-going,
   and test argument forwarding are outside this phase.
5. Add direct bootstrap `exec` using the existing managed installation and
   environment, preserving argv and child exit status without profile recursion
   (#388). No setup hooks or new interpreter-selection policy are introduced;
   ADR-016's restriction on hooks remains unchanged.
6. Amend ADR-016's tool-pin restriction: consumers may declare exact cargo-tool
   versions and rustup targets for setup to provision. Python installation
   ownership remains unchanged: a managed-wheel `sc_lint.setup_tools` helper
   reads the existing persisted `.sc-lint/install.json` tools object after
   ordinary bootstrap setup has ensured the product. No second tool manifest
   or Rust installation engine is added. Direct binary setup retains its
   current meaning; lint execution never installs tools. Cargo tools use
   `cargo install --locked --version =<exact-version> <package>` with installed
   package/version and executable checks; rustup targets use inspect/add/verify
   without selecting a toolchain. Respect CARGO_HOME and expose its bin directory.
   Product defaults do not choose consumer tools or versions. Dry-run never
   installs, creates a venv, downloads, or writes; if declared tools cannot be
   inspected because the managed helper is absent, it reports that failure
   rather than bootstrapping. Provisioning and platform execution are separate
   sequential sprints (k-7 then k-9), sharing only their declared configuration
   and installed environment.

## Compatibility and approval

Optional manifests/configuration activate policy only when present. An empty
manifest-policy family array adds no checks; a present allowlist record with
an empty allowed-dependency list allows none. Retain upstream nested record
field shapes and ignored unknown nested keys; the existing strict analyzer
top-level key contract remains unchanged. Existing
simpler consumers require no atm-core-specific configuration. Tightened inventory
and stale-entry diagnostics can reject previously accepted invalid declarations;
these effects must be included in release notes and consumer acceptance tests.
No release version or publication is authorized by this planning ADR.

These amendments were accepted by Rand in the session cited above. The plan
adds no shared sc-publish functionality or skill changes. Sprint dispatch still
requires the plan review gate; approval of this ADR does not dispatch work.
