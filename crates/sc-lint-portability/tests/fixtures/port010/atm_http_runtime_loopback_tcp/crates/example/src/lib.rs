// Reduced from randlee/atm-core crates/atm-http-runtime/src/loopback_tcp.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
fn create_private_record_file(path: &Path) -> std::io::Result<std::fs::File> {
    unimplemented!()
}
#[cfg(not(unix))]
fn create_private_record_file(path: &Path) -> std::io::Result<std::fs::File> {
    unimplemented!()
}
#[cfg(unix)]
fn validate_record_parent(parent: &Path) -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(not(unix))]
fn validate_record_parent(_parent: &Path) -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(unix)]
fn restrict_record_to_owner(path: &Path) -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(not(unix))]
#[cfg(not(windows))]
fn restrict_record_to_owner(_path: &Path) -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "Windows owner-only ACL FFI is confined to loopback endpoint-record publication"
)]
fn restrict_record_to_owner(path: &Path) -> Result<(), AtmError> {
    unimplemented!()
}
