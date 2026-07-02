use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use cargo_metadata::MetadataCommand;
use toml::Table;
use toml::Value;

use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

const REQUIRED_WORKSPACE_FIELDS: [&str; 6] = [
    "edition",
    "rust-version",
    "authors",
    "license",
    "repository",
    "homepage",
];

pub(crate) struct ManifestPolicyReport {
    pub(crate) findings: Vec<Finding>,
    pub(crate) scanned_crates: usize,
}

pub(crate) fn analyze_manifest_policy(root: &Path) -> Result<ManifestPolicyReport> {
    let root = fs::canonicalize(root)
        .with_context(|| format!("failed to canonicalize repo root `{}`", root.display()))?;
    let workspace_version = workspace_version(&root)?;
    let manifests = member_manifests(&root)?;
    let mut expected_versions = BTreeMap::new();

    for manifest_path in &manifests {
        let manifest = load_manifest(manifest_path)?;
        let rel_manifest = relative_manifest_display(&root, manifest_path)?;
        let package_version =
            expected_package_version(&manifest, &workspace_version, &rel_manifest)?;
        expected_versions.insert(canonical_manifest_dir(manifest_path)?, package_version);
    }

    let mut findings = Vec::new();
    for manifest_path in &manifests {
        let manifest = load_manifest(manifest_path)?;
        let rel_manifest = relative_manifest_display(&root, manifest_path)?;
        let package = manifest
            .get("package")
            .and_then(Value::as_table)
            .with_context(|| format!("{rel_manifest} missing [package] table"))?;

        for field in REQUIRED_WORKSPACE_FIELDS {
            if !workspace_inherited(package, field) {
                findings.push(Finding {
                    rule_id: RuleId::ScbManifest001,
                    kind: "package_workspace_field_required".to_string(),
                    message: format!("{rel_manifest}: set [package].{field}.workspace = true"),
                    owner_ids: vec![OwnerId::new(rel_manifest.clone())],
                    node_ids: Vec::new(),
                });
            }
        }

        let manifest_dir = canonical_manifest_dir(manifest_path)?;
        for (section_name, dependencies) in dependency_sections(&manifest) {
            for (dependency_name, dependency) in dependencies {
                let Some(dependency_table) = dependency.as_table() else {
                    continue;
                };
                let Some(dependency_path) = dependency_table.get("path").and_then(Value::as_str)
                else {
                    continue;
                };
                let resolved_path = canonical_dependency_dir(&manifest_dir, dependency_path)?;
                let Some(expected_dependency_version) = expected_versions.get(&resolved_path)
                else {
                    continue;
                };
                let pinned_version = dependency_table.get("version").and_then(Value::as_str);
                if pinned_version != Some(expected_dependency_version.as_str()) {
                    findings.push(Finding {
                        rule_id: RuleId::ScbManifest002,
                        kind: "internal_path_dependency_version_mismatch".to_string(),
                        message: format!(
                            "{rel_manifest} [{section_name}.{dependency_name}]: path dependency version must match target crate version \"{expected_dependency_version}\""
                        ),
                        owner_ids: vec![OwnerId::new(rel_manifest.clone())],
                        node_ids: Vec::new(),
                    });
                }
            }
        }
    }

    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then_with(|| left.message.cmp(&right.message))
    });

    Ok(ManifestPolicyReport {
        findings,
        scanned_crates: manifests.len(),
    })
}

fn workspace_version(root: &Path) -> Result<String> {
    let manifest = load_manifest(&root.join("Cargo.toml"))?;
    manifest
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(Value::as_table)
        .and_then(|package| package.get("version"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("workspace.package.version missing from Cargo.toml")
}

fn member_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    let metadata = MetadataCommand::new()
        .current_dir(root)
        .manifest_path(root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("failed to load workspace metadata for manifest policy")?;
    let workspace_members = metadata.workspace_members;
    let mut manifests = metadata
        .packages
        .into_iter()
        .filter(|package| workspace_members.iter().any(|id| id == &package.id))
        .map(|package| package.manifest_path.into_std_path_buf())
        .collect::<Vec<_>>();
    manifests.sort_by(|left, right| {
        relative_manifest_display(root, left)
            .unwrap_or_else(|_| left.display().to_string())
            .cmp(
                &relative_manifest_display(root, right)
                    .unwrap_or_else(|_| right.display().to_string()),
            )
    });
    Ok(manifests)
}

fn load_manifest(path: &Path) -> Result<Table> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest `{}`", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse manifest `{}`", path.display()))
}

fn expected_package_version(
    manifest: &Table,
    workspace_version: &str,
    manifest_label: &str,
) -> Result<String> {
    let package = manifest
        .get("package")
        .and_then(Value::as_table)
        .with_context(|| format!("{manifest_label} missing [package] table"))?;
    let version_value = package
        .get("version")
        .with_context(|| {
            format!(
                "{manifest_label} must define [package].version either as a non-empty string or version.workspace = true"
            )
        })?;
    if let Some(version) = version_value.as_str()
        && !version.trim().is_empty()
    {
        return Ok(version.to_string());
    }
    if version_value
        .as_table()
        .and_then(|value| value.get("workspace"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Ok(workspace_version.to_string());
    }
    anyhow::bail!(
        "{manifest_label} must define [package].version either as a non-empty string or version.workspace = true"
    );
}

fn workspace_inherited(package: &Table, field: &str) -> bool {
    package
        .get(field)
        .and_then(Value::as_table)
        .and_then(|value| value.get("workspace"))
        .and_then(Value::as_bool)
        == Some(true)
}

fn dependency_sections(manifest: &Table) -> Vec<(String, &Table)> {
    let mut sections = Vec::new();
    for section_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(dependencies) = manifest.get(section_name).and_then(Value::as_table) {
            sections.push((section_name.to_string(), dependencies));
        }
    }

    if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
        let mut target_names = targets.keys().cloned().collect::<Vec<_>>();
        target_names.sort();
        for target_name in target_names {
            let Some(target) = targets.get(&target_name).and_then(Value::as_table) else {
                continue;
            };
            for section_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(dependencies) = target.get(section_name).and_then(Value::as_table) {
                    sections.push((format!("target.{target_name}.{section_name}"), dependencies));
                }
            }
        }
    }

    sections
}

fn relative_manifest_display(root: &Path, manifest_path: &Path) -> Result<String> {
    let relative = manifest_path.strip_prefix(root).with_context(|| {
        format!(
            "manifest `{}` is not under repo root `{}`",
            manifest_path.display(),
            root.display()
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn canonical_manifest_dir(manifest_path: &Path) -> Result<PathBuf> {
    let parent = manifest_path.parent().with_context(|| {
        format!(
            "manifest `{}` missing parent directory",
            manifest_path.display()
        )
    })?;
    fs::canonicalize(parent)
        .with_context(|| format!("failed to canonicalize manifest dir `{}`", parent.display()))
}

fn canonical_dependency_dir(manifest_dir: &Path, dependency_path: &str) -> Result<PathBuf> {
    fs::canonicalize(manifest_dir.join(dependency_path)).with_context(|| {
        format!(
            "failed to canonicalize dependency path `{}` from `{}`",
            dependency_path,
            manifest_dir.display()
        )
    })
}
