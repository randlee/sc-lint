// Reduced from randlee/atm-core crates/atm-core/src/atm_temp_sweeper.rs at f4746d500 for PORT-010 parity fixtures; function bodies are stubbed.
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
impl EntryAge {
    #[cfg(unix)]
    #[must_use]
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        unimplemented!()
    }
    #[cfg(not(unix))]
    #[must_use]
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        unimplemented!()
    }
}
#[cfg(unix)]
fn unix_epoch_time(secs: i64, nsecs: i64) -> Option<SystemTime> {
    unimplemented!()
}
#[cfg(windows)]
fn is_symlink_or_reparse_point(metadata: &std::fs::Metadata) -> bool {
    unimplemented!()
}
#[cfg(not(windows))]
fn is_symlink_or_reparse_point(metadata: &std::fs::Metadata) -> bool {
    unimplemented!()
}
