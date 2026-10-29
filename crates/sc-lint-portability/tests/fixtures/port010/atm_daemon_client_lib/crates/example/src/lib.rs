// Reduced from randlee/atm-core crates/atm-daemon-client/src/lib.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use interprocess::local_socket::Stream as LocalSocketStream;
#[cfg(unix)]
use interprocess::local_socket::prelude::*;
impl DaemonLocalIpcEndpoint {
    #[cfg(unix)]
    fn unix_socket_path(&self) -> Result<PathBuf, AtmError> {
        unimplemented!()
    }
}
#[cfg(windows)]
fn reject_socket_override() -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(unix)]
fn try_connect_unix_socket(
    endpoint: &DaemonLocalIpcEndpoint,
) -> Result<LocalSocketStream, AtmError> {
    unimplemented!()
}
