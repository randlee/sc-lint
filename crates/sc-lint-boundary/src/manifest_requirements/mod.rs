use std::path::Path;

use cargo_metadata::Metadata;

use crate::BoundaryError;
use crate::Finding;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DependencyRequirementsConfig {
    pub(crate) entries: Vec<DependencyRequirements>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DependencyRequirements {
    pub(crate) template_manifests: Vec<String>,
    pub(crate) package_archives: Vec<String>,
    pub(crate) test_manifests: Vec<String>,
}

pub(crate) fn analyze(
    _root: &Path,
    _metadata: &Metadata,
    _config: &DependencyRequirementsConfig,
) -> std::result::Result<Vec<Finding>, BoundaryError> {
    Ok(Vec::new())
}
