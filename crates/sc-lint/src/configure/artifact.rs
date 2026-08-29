use std::path::Path;

use crate::CliError;

/// Kind of a product-managed staged output. This is crate-private so later
/// workflow support shares the transaction without becoming a plugin surface.
#[allow(
    dead_code,
    reason = "F.4a wires concrete TOML and Just artifacts after the reviewed-plan CLI is added."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactKind {
    Toml,
    Justfile,
    Json,
    WorkflowYaml,
}

/// One validated output in the configure transaction.
///
/// The trait is object-safe intentionally: the transaction owns an ordered
/// heterogeneous set while preserving one validation and rollback path.
#[allow(
    dead_code,
    reason = "F.4a's transaction consumes this object-safe seam once apply dispatch is wired."
)]
pub(crate) trait ManagedArtifact {
    fn kind(&self) -> ArtifactKind;
    fn target(&self) -> &Path;
    fn staged_bytes(&self) -> &[u8];
    fn validate_staged(&self) -> Result<(), CliError>;
}
