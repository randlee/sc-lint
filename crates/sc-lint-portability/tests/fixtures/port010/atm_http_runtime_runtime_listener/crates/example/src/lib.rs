// Reduced from randlee/atm-core crates/atm-http-runtime/src/runtime_listener.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
pub(crate) async fn bind_configured_unix_listener(
    config: &HttpRuntimeConfig,
    health: &RuntimeHealth,
) -> Result<Option<(UnixListener, UnixSocketPathGuard)>, AtmError> {
    unimplemented!()
}
