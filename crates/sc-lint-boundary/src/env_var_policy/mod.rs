use std::path::Path;

use cargo_metadata::Metadata;

use crate::BoundaryError;
use crate::Finding;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EnvVarBoundaryConfig {
    pub(crate) entries: Vec<EnvVarBoundary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EnvVarBoundary {
    pub(crate) forbidden_env_vars: Vec<String>,
    pub(crate) restricted_crate_roots: Vec<String>,
    pub(crate) boundary_reader_functions: Vec<String>,
    pub(crate) reason: Option<String>,
}

pub(crate) fn analyze(
    _root: &Path,
    _metadata: &Metadata,
    _config: &EnvVarBoundaryConfig,
) -> std::result::Result<Vec<Finding>, BoundaryError> {
    Ok(Vec::new())
}
