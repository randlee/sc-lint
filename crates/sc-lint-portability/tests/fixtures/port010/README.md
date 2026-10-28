# PORT-010 atm-core shape fixtures

Reduced from `randlee/atm-core`, branch `integrate/phase-bg`, commit
`f4746d500`. The source `sc-lint.toml` had these 18 PORT-010 paths in its
`exclude_paths_by_rule` entry, in source order; this fixture set removes that
exclusion and records representative classifications.

1. `crates/atm-core/src/atm_temp.rs`
2. `crates/atm-core/src/atm_temp_sweeper.rs`
3. `crates/atm-core/src/descriptor_limit.rs`
4. `crates/atm-core/src/persistence.rs`
5. `crates/atm-core/src/transfer_script.rs`
6. `crates/atm-daemon-bootstrap/src/lib.rs`
7. `crates/atm-daemon-bootstrap/src/protected_trust_root.rs`
8. `crates/atm-daemon-client/src/lib.rs`
9. `crates/atm-herdr/src/transport_socket.rs`
10. `crates/atm-http-runtime/src/client.rs`
11. `crates/atm-http-runtime/src/client_tail.rs`
12. `crates/atm-http-runtime/src/http1_server.rs`
13. `crates/atm-http-runtime/src/lib.rs`
14. `crates/atm-http-runtime/src/loopback_tcp.rs`
15. `crates/atm-http-runtime/src/runtime_listener.rs`
16. `crates/atm-http-runtime/src/runtime_setup.rs`
17. `crates/atm-http-runtime/src/unix_socket.rs`
18. `crates/atm/src/main.rs`

| Fixture | Reduced from | Classification |
|---|---|---|
| `unix_helper_in_paired_file` | `crates/atm-http-runtime/src/unix_socket.rs` | Private Unix helper in a configured adapter pair; no PORT-010 finding after L1-4. |
| `crate_root_main` | `crates/atm/src/main.rs` | Crate-root `main.rs` function with `cfg(unix)` and `cfg(not(unix))` siblings; remains reported as a known limitation tracked in issue `lint-phase-l.3`. |
| `unpaired_pub_item` | `crates/atm-core/src/descriptor_limit.rs` | Unpaired public Unix item; PORT-010 true positive remains. |

The upstream exclusion was rule-scoped (`PORT-010`); no analyzer-wide
portability exclusion was present. This provenance file is for review and is
not read by the analyzer or tests.
