// Reduced from randlee/atm-core crates/atm-herdr/src/transport_socket.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
async fn connect_unix(
    endpoint: &HerdrEndpoint,
    deadline: RequestDeadline,
) -> Result<tokio::net::UnixStream, HerdrError> {
    unimplemented!()
}
#[cfg(windows)]
async fn connect_named_pipe(
    endpoint: &HerdrEndpoint,
    deadline: RequestDeadline,
    retry_delay: Duration,
) -> Result<tokio::net::windows::named_pipe::NamedPipeClient, HerdrError> {
    unimplemented!()
}
#[cfg(windows)]
async fn retry_pipe_busy<T>(
    deadline: RequestDeadline,
    retry_delay: Duration,
    mut open: impl FnMut() -> io::Result<T>,
) -> Result<T, HerdrError> {
    unimplemented!()
}
