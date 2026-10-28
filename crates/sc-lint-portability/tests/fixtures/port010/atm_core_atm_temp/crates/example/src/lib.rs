// Reduced from randlee/atm-core crates/atm-core/src/atm_temp.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
fn create_scratch_dir(path: &Path) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(windows)]
fn create_scratch_dir(path: &Path) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(unix)]
fn validate_existing_scratch_dir(
    path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(windows)]
fn validate_existing_scratch_dir(
    _path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<(), AtmTempError> {
    unimplemented!()
}
#[cfg(unix)]
#[must_use]
pub fn is_owned_by_uid(metadata: &std::fs::Metadata, owner_uid: u32) -> bool {
    unimplemented!()
}
#[cfg(unix)]
pub(crate) fn has_group_or_world_bits(mode: u32) -> bool {
    unimplemented!()
}
#[cfg(unix)]
#[must_use]
pub fn current_uid() -> u32 {
    unimplemented!()
}
#[cfg(any(unix, test))]
fn unix_default_scratch_root(temp_dir: &Path, uid: u32) -> PathBuf {
    unimplemented!()
}
#[cfg(any(windows, test))]
fn windows_default_scratch_root(temp_dir: &Path) -> PathBuf {
    unimplemented!()
}
#[cfg(unix)]
fn default_scratch_root() -> PathBuf {
    unimplemented!()
}
#[cfg(windows)]
fn default_scratch_root() -> PathBuf {
    unimplemented!()
}
