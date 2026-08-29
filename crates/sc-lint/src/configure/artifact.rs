use std::path::Path;
use std::path::PathBuf;

use crate::CliError;

/// Kind of a product-managed staged output. This is crate-private so later
/// workflow support shares the transaction without becoming a plugin surface.
#[allow(
    dead_code,
    reason = "F.4b is the only workflow/JSON producer; the closed shared enum is intentionally defined before that producer."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactKind {
    Toml,
    Justfile,
    Shell,
    Json,
    WorkflowYaml,
}

/// One validated output in the configure transaction.
///
/// The trait is object-safe intentionally: the transaction owns an ordered
/// heterogeneous set while preserving one validation and rollback path.
pub(crate) trait ManagedArtifact {
    fn kind(&self) -> ArtifactKind;
    fn target(&self) -> &Path;
    fn staged_bytes(&self) -> &[u8];
    fn is_removal(&self) -> bool {
        false
    }
    #[expect(
        clippy::result_large_err,
        reason = "The frozen F.4a extension contract uses CliError for structured recovery."
    )]
    fn validate_staged(&self) -> Result<(), CliError>;
}

/// A reviewed deletion uses the same backup and rollback transaction as a
/// generated file replacement. It is crate-private so callers cannot broaden
/// the finite configure allowlist.
#[allow(
    dead_code,
    reason = "Legacy fingerprint recognition wires the finite removal allowlist into this shared transaction next."
)]
pub(crate) struct RemoveArtifact {
    kind: ArtifactKind,
    target: PathBuf,
}

impl RemoveArtifact {
    #[allow(
        dead_code,
        reason = "Legacy fingerprint recognition wires the finite removal allowlist into this shared transaction next."
    )]
    pub(crate) fn new(kind: ArtifactKind, target: PathBuf) -> Self {
        Self { kind, target }
    }
}

impl ManagedArtifact for RemoveArtifact {
    fn kind(&self) -> ArtifactKind {
        self.kind
    }
    fn target(&self) -> &Path {
        &self.target
    }
    fn staged_bytes(&self) -> &[u8] {
        &[]
    }
    fn is_removal(&self) -> bool {
        true
    }
    fn validate_staged(&self) -> Result<(), CliError> {
        if self.target.is_file() {
            Ok(())
        } else {
            Err(invalid(
                &self.target,
                "reviewed removal target is absent or is not a file",
            ))
        }
    }
}

/// The product's ordinary generated files all use the same byte-owning
/// artifact.  Test-only artifacts can implement `ManagedArtifact` directly,
/// which proves the transaction does not have a hidden second write path.
pub(crate) struct BytesArtifact {
    kind: ArtifactKind,
    target: PathBuf,
    bytes: Vec<u8>,
}

impl BytesArtifact {
    pub(crate) fn new(kind: ArtifactKind, target: PathBuf, bytes: Vec<u8>) -> Self {
        Self {
            kind,
            target,
            bytes,
        }
    }
}

impl ManagedArtifact for BytesArtifact {
    fn kind(&self) -> ArtifactKind {
        self.kind
    }

    fn target(&self) -> &Path {
        &self.target
    }

    fn staged_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn validate_staged(&self) -> Result<(), CliError> {
        match self.kind {
            ArtifactKind::Toml => {
                let source = std::str::from_utf8(&self.bytes)
                    .map_err(|error| invalid(&self.target, error))?;
                toml::from_str::<toml::Value>(source)
                    .map_err(|error| invalid(&self.target, error))?;
            }
            ArtifactKind::Json => {
                serde_json::from_slice::<serde_json::Value>(&self.bytes)
                    .map_err(|error| invalid(&self.target, error))?;
            }
            ArtifactKind::Justfile => crate::configure::just::validate(&self.target, &self.bytes)?,
            ArtifactKind::Shell => {
                let source = std::str::from_utf8(&self.bytes)
                    .map_err(|error| invalid(&self.target, error))?;
                if source.trim().is_empty() {
                    return Err(invalid(&self.target, "a shell helper cannot be empty"));
                }
            }
            ArtifactKind::WorkflowYaml => {
                // F.4b supplies the YAML parser and concrete workflow artifact;
                // the shared transaction intentionally has no workflow shortcut.
                if self.bytes.is_empty() {
                    return Err(invalid(&self.target, "a workflow artifact cannot be empty"));
                }
            }
        }
        Ok(())
    }
}

fn invalid(target: &Path, cause: impl std::fmt::Display) -> CliError {
    CliError::config(format!(
        "generated artifact `{}` is invalid",
        target.display()
    ))
    .with_code("CLI.CONFIGURE_UNMANAGED_COLLISION")
    .with_cause(cause.to_string())
    .with_suggested_action("Review the exportable patch; no repository files were changed.")
    .with_documentation("sc-lint docs troubleshooting")
}
