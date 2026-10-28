// Reduced from randlee/atm-core crates/atm-http-runtime/src/http1_server.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(unix)]
impl Http1Acceptor for UnixListener {}
#[cfg(unix)]
pub(crate) async fn serve_unix_http1(
    listener: UnixListener,
    router: Router,
    max_connections: usize,
    header_read_timeout: Duration,
    shutdown_rx: watch::Receiver<()>,
) -> io::Result<()> {
    unimplemented!()
}
