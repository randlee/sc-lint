// Reduced from randlee/atm-core crates/atm-http-runtime/src/client_tail.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
pub fn unix_socket_client(
    socket_path: impl AsRef<Path>,
    request_timeout: Duration,
) -> Result<Arc<dyn DaemonApiClient>, AtmError> {
    unimplemented!()
}
#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct UnixSocketConnector {
    socket_path: PathBuf,
    transport: RwLock<Option<Arc<GenerationalReqwestClient>>>,
}
#[cfg(unix)]
impl UnixSocketConnector {}
#[cfg(unix)]
fn build_unix_socket_reqwest_client(
    socket_path: &Path,
) -> Result<reqwest::Client, HttpRuntimeClientFailure> {
    unimplemented!()
}
#[cfg(unix)]
fn unix_socket_generation(
    socket_path: &Path,
) -> Result<TransportGeneration, HttpRuntimeClientFailure> {
    unimplemented!()
}
#[cfg(unix)]
#[async_trait]
impl HttpRuntimeConnector for UnixSocketConnector {}
#[cfg(unix)]
impl UnixSocketConnector {}
