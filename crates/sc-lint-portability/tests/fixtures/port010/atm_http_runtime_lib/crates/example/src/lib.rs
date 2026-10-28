// Reduced from randlee/atm-core crates/atm-http-runtime/src/lib.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(unix)]
mod unix_socket;
#[cfg(unix)]
use http1_server::serve_unix_http1;
#[cfg(unix)]
pub(crate) use runtime_setup::validate_unix_socket_path;
#[cfg(unix)]
use unix_socket::{
    UnixSocketPathGuard, UnixSocketStartupLock, bind_unix_listener,
    reclaim_stale_unix_socket,
};
#[cfg(unix)]
pub use client::unix_socket_client;
impl DirectPeerTcpConfig {
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn ephemeral_for_test() -> Self {
        unimplemented!()
    }
}
impl UnixSocketOwnerUid {
    #[must_use]
    #[cfg(unix)]
    const fn get(self) -> u32 {
        unimplemented!()
    }
}
impl UnixSocketMode {
    #[must_use]
    #[cfg(unix)]
    const fn get(self) -> u32 {
        unimplemented!()
    }
}
