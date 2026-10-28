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
| `unix_helper_in_paired_file` | `crates/atm-http-runtime/src/unix_socket.rs` | In this fixture, the private Unix helper is in a configured adapter pair and the fixture's `#[cfg(windows)] mod named_pipe` is its companion, so the fixture has no PORT-010 finding. The real atm-core `unix_socket.rs` still fires PORT-010. |
| `crate_root_main` | `crates/atm/src/main.rs` | Crate-root `main.rs` function with `cfg(unix)` and `cfg(not(unix))` siblings; remains reported as a known limitation tracked in issue `lint-phase-l.3`. |
| `unpaired_pub_item` | `crates/atm-core/src/descriptor_limit.rs` | Unpaired public Unix item; PORT-010 true positive remains. |
| `atm_core_atm_temp` | `crates/atm-core/src/atm_temp.rs` | Reduced; reports `is_owned_by_uid`, `has_group_or_world_bits`, `current_uid`, `unix_default_scratch_root`. |
| `atm_core_atm_temp_sweeper` | `crates/atm-core/src/atm_temp_sweeper.rs` | Reduced; reports `from_metadata`, `unix_epoch_time`. |
| `atm_core_persistence` | `crates/atm-core/src/persistence.rs` | Reduced; reports `sync_parent_directory`. |
| `atm_core_transfer_script` | `crates/atm-core/src/transfer_script.rs` | Reduced; reports `unix_synthesized_transfer_script_path`, `is_owner_executable`. |
| `atm_daemon_bootstrap_lib` | `crates/atm-daemon-bootstrap/src/lib.rs` | Reduced; reports `unix_socket_config`, `unix_socket_config_for_uid`. |
| `atm_daemon_bootstrap_protected_trust_root` | `crates/atm-daemon-bootstrap/src/protected_trust_root.rs` | Reduced; reports `verify_protected_path`, `verify_unix_metadata`. |
| `atm_daemon_client_lib` | `crates/atm-daemon-client/src/lib.rs` | Reduced; reports `unix_socket_path`, `try_connect_unix_socket`. |
| `atm_herdr_transport_socket` | `crates/atm-herdr/src/transport_socket.rs` | Reduced; reports `connect_unix`. |
| `atm_http_runtime_client` | `crates/atm-http-runtime/src/client.rs` | Reduced; reports `runtime_directory_is_root_owned`. |
| `atm_http_runtime_client_tail` | `crates/atm-http-runtime/src/client_tail.rs` | Reduced; reports `unix_socket_client`, `build_unix_socket_reqwest_client`, `unix_socket_generation`. |
| `atm_http_runtime_http1_server` | `crates/atm-http-runtime/src/http1_server.rs` | Reduced; reports `serve_unix_http1`. |
| `atm_http_runtime_lib` | `crates/atm-http-runtime/src/lib.rs` | Reduced; reports `unix_socket` and two `get` impl methods. |
| `atm_http_runtime_loopback_tcp` | `crates/atm-http-runtime/src/loopback_tcp.rs` | Reduced; reports `create_private_record_file`, `validate_record_parent`. |
| `atm_http_runtime_runtime_listener` | `crates/atm-http-runtime/src/runtime_listener.rs` | Reduced; reports `bind_configured_unix_listener`. |
| `atm_http_runtime_runtime_setup` | `crates/atm-http-runtime/src/runtime_setup.rs` | Reduced; reports `validate_unix_socket_path`. |

Fixtures from `atm_core_atm_temp` onward are function-body-stubbed reductions
of the pinned sources (cfg-gated items, same-name companions, and their
containing impls and modules). Their expected findings were produced by the
analyzer on the full source files and are pinned in `tests/cli.rs`.

The upstream exclusion was rule-scoped (`PORT-010`); no analyzer-wide
portability exclusion was present. This provenance file is for review and is
not read by the analyzer or tests.
