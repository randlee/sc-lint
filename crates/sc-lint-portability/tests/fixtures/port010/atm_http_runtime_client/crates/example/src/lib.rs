// Reduced from randlee/atm-core crates/atm-http-runtime/src/client.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
pub use client_tail::unix_socket_client;
#[cfg(unix)]
fn runtime_directory_is_root_owned(runtime_directory: &Path) -> Result<bool, AtmError> {
    unimplemented!()
}
