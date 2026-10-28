// Reduced from randlee/atm-core crates/atm-daemon-bootstrap/src/lib.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use std::num::NonZeroU32;
#[cfg(unix)]
use atm_core::home::HOST_RUNTIME_SOCKET_FILE;
#[cfg(feature = "test-support")]
pub mod test_support;
#[cfg(feature = "benchmark-harness")]
pub use received_hook_selector::{BenchmarkHookMode, benchmark_received_hook_selector};
#[cfg(feature = "benchmark-harness")]
pub async fn run_benchmark_daemon(hook_mode: BenchmarkHookMode) -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(unix)]
fn unix_socket_config(
    scope: &atm_core::home::HostRuntimeScope,
) -> Result<Option<atm_http_runtime::UnixSocketConfig>, AtmError> {
    unimplemented!()
}
#[cfg(unix)]
fn unix_socket_config_for_uid(
    runtime_root: &std::path::Path,
    uid: u32,
) -> Option<atm_http_runtime::UnixSocketConfig> {
    unimplemented!()
}
#[cfg(not(unix))]
fn unix_socket_config(
    _scope: &atm_core::home::HostRuntimeScope,
) -> Result<Option<atm_http_runtime::UnixSocketConfig>, AtmError> {
    unimplemented!()
}
