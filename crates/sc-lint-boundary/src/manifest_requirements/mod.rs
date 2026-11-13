mod archive;
mod glob;
mod manifest;
mod version;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use cargo_metadata::Metadata;

use crate::BoundaryError;
use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

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
    root: &Path,
    metadata: &Metadata,
    config: &DependencyRequirementsConfig,
) -> std::result::Result<Vec<Finding>, BoundaryError> {
    if config.entries.is_empty() {
        return Ok(Vec::new());
    }

    analyze_configured(root, metadata, config).map_err(|error| {
        match error.downcast::<BoundaryError>() {
            Ok(boundary_error) => boundary_error,
            Err(error) => BoundaryError::manifest_policy(root, error),
        }
    })
}

fn analyze_configured(
    root: &Path,
    metadata: &Metadata,
    config: &DependencyRequirementsConfig,
) -> Result<Vec<Finding>> {
    let root = fs::canonicalize(root)
        .with_context(|| format!("failed to canonicalize repo root `{}`", root.display()))?;
    let (manifest_patterns, archive_patterns) = config.entries.iter().fold(
        (Vec::new(), Vec::new()),
        |(mut manifests, mut archives), entry| {
            manifests.extend(entry.template_manifests.iter().cloned());
            manifests.extend(entry.test_manifests.iter().cloned());
            archives.extend(entry.package_archives.iter().cloned());
            (manifests, archives)
        },
    );

    let mut manifests = BTreeMap::<String, PathBuf>::new();
    manifests.insert("Cargo.toml".to_owned(), root.join("Cargo.toml"));
    for package in crate::workspace_packages(metadata) {
        let path = package.manifest_path.as_std_path();
        if let Some(relative) = relative_path(&root, path) {
            manifests.insert(relative, path.to_path_buf());
        }
    }
    for path in glob::expand(&root, &manifest_patterns)? {
        if let Some(relative) = relative_path(&root, &path) {
            manifests.insert(relative, path);
        }
    }

    let mut findings = Vec::new();
    for (label, path) in manifests {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read checked manifest `{label}`"))?;
        findings.extend(manifest::findings(&label, &text)?);
    }

    for archive_path in glob::expand(&root, &archive_patterns)? {
        let archive_label = relative_path(&root, &archive_path)
            .unwrap_or_else(|| archive_path.display().to_string());
        findings.extend(archive::findings(&archive_path, &archive_label)?);
    }

    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then_with(|| left.message.cmp(&right.message))
    });
    Ok(findings)
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn finding(rule_id: RuleId, kind: &str, message: String, owner: String) -> Finding {
    Finding {
        rule_id,
        kind: kind.into(),
        message,
        owner_ids: vec![OwnerId::new(owner)],
        node_ids: Vec::new(),
    }
}
