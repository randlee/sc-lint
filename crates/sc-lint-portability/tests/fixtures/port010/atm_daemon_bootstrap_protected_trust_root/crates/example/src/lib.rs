// Reduced from randlee/atm-core crates/atm-daemon-bootstrap/src/protected_trust_root.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[path = "protected_trust_root/policy.rs"]
#[cfg(any(windows, test))]
mod policy;
#[cfg(unix)]
const TRUST_ROOT_FILE: &str = "/etc/atm/tool-sender-trust-root.json";
#[cfg(windows)]
#[path = "protected_trust_root/windows.rs"]
mod windows;
#[cfg(unix)]
fn read_protected_file(path: &Path) -> Result<String, AtmError> {
    unimplemented!()
}
#[cfg(windows)]
fn read_protected_file(path: &Path) -> Result<String, AtmError> {
    unimplemented!()
}
#[cfg(not(any(unix, windows)))]
fn read_protected_file(path: &Path) -> Result<String, AtmError> {
    unimplemented!()
}
#[cfg(unix)]
fn verify_protected_path(path: &Path) -> Result<(), AtmError> {
    unimplemented!()
}
#[cfg(unix)]
fn verify_unix_metadata(
    path: &Path,
    parent_is_dir: bool,
    parent_uid: u32,
    parent_mode: u32,
    file_is_regular: bool,
    file_uid: u32,
    file_mode: u32,
) -> Result<(), AtmError> {
    unimplemented!()
}
