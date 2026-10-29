// Reduced from randlee/atm-core crates/atm-core/src/transfer_script.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use crate::atm_temp::{current_uid, is_owned_by_uid};
#[cfg(windows)]
#[must_use]
pub fn synthesized_transfer_script_env(
    env: &dyn EnvSource,
) -> Vec<(&'static str, OsString)> {
    unimplemented!()
}
#[cfg(not(windows))]
#[must_use]
pub fn synthesized_transfer_script_env(
    env: &dyn EnvSource,
) -> Vec<(&'static str, OsString)> {
    unimplemented!()
}
#[cfg(unix)]
fn unix_synthesized_transfer_script_path() -> OsString {
    unimplemented!()
}
#[cfg(windows)]
fn windows_synthesized_transfer_script_env(
    env: &dyn EnvSource,
) -> Vec<(&'static str, OsString)> {
    unimplemented!()
}
#[cfg(windows)]
fn windows_synthesized_transfer_script_path(
    system_root: &str,
    env: &dyn EnvSource,
) -> OsString {
    unimplemented!()
}
#[cfg(windows)]
fn locate_pwsh_dir(env: &dyn EnvSource) -> Option<PathBuf> {
    unimplemented!()
}
#[cfg(any(test, feature = "test-utils"))]
pub fn resolve_transfer_script_in_for_tests(
    transfer_root: &Path,
    host: &HostName,
) -> Result<TransferScript, AtmTempError> {
    unimplemented!()
}
#[cfg(unix)]
fn check_transfer_root_metadata(
    transfer_root: &Path,
    host: &HostName,
    metadata: &std::fs::Metadata,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(windows)]
fn check_transfer_root_metadata(
    transfer_root: &Path,
    host: &HostName,
    metadata: &std::fs::Metadata,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(windows)]
fn script_path_for(
    transfer_root: &Path,
    host: &HostName,
) -> (PathBuf, TransferScriptKind) {
    unimplemented!()
}
#[cfg(not(windows))]
fn script_path_for(
    transfer_root: &Path,
    host: &HostName,
) -> (PathBuf, TransferScriptKind) {
    unimplemented!()
}
#[cfg(unix)]
fn check_script_safety(
    path: &Path,
    host: &HostName,
    metadata: &std::fs::Metadata,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(windows)]
fn check_script_safety(
    path: &Path,
    host: &HostName,
    metadata: &std::fs::Metadata,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(windows)]
fn canonicalize_for_containment(path: &Path) -> PathBuf {
    unimplemented!()
}
#[cfg(windows)]
fn windows_profile_home_for_containment(env: &dyn EnvSource) -> Option<PathBuf> {
    unimplemented!()
}
#[cfg(any(windows, test))]
fn resolve_profile_home(
    from_env: Option<PathBuf>,
    from_os_account: Option<PathBuf>,
) -> Option<PathBuf> {
    unimplemented!()
}
#[cfg(any(windows, test))]
fn check_path_within_profile(
    path: &Path,
    profile_home: &Path,
    is_reparse_point: bool,
    what: &str,
    host: &HostName,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(any(windows, test))]
fn path_is_within(path: &Path, root: &Path) -> bool {
    unimplemented!()
}
#[cfg(any(windows, test))]
fn components_match(
    expected: std::path::Component<'_>,
    actual: std::path::Component<'_>,
) -> bool {
    unimplemented!()
}
#[cfg(any(windows, test))]
fn normalized_prefix_key(kind: std::path::Prefix<'_>) -> String {
    unimplemented!()
}
#[cfg(unix)]
fn is_owner_executable(mode: u32) -> bool {
    unimplemented!()
}
